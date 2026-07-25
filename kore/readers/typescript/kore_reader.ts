/**
 * KORE v2 Reader — TypeScript/JavaScript (Node.js + Deno + Bun)
 * Zero external dependencies.
 *
 * Usage (Node.js):
 *   import { KoreReader } from './kore_reader';
 *   const reader = KoreReader.open('data.kore');
 *   console.log(reader.info());
 *   const ids = reader.readIntColumn('id');       // BigInt64Array
 *   const prices = reader.readFloatColumn('price'); // Float64Array
 *   const names = reader.readStrColumn('name');     // string[]
 *   const flags = reader.readBoolColumn('active');  // boolean[]
 */

import { readFileSync, existsSync } from 'fs';

// ── Constants ────────────────────────────────────────────────────
const KORE_MAGIC = 'KORE';
const HEADER_SIZE = 64;
const KTYPE_INT = 1, KTYPE_FLOAT = 2, KTYPE_BOOL = 3, KTYPE_STR = 4, KTYPE_BYTES = 5;
const CODEC_RAW = 0, CODEC_RLE = 1, CODEC_DELTA = 2, CODEC_DICTRLE = 3,
      CODEC_BITPACK = 4, CODEC_BDICT = 5, CODEC_CDELTA = 6,
      CODEC_FOR = 7, CODEC_HUFFDICT = 8;
const RC_SCALE = 4096, RC_BOT = 1 << 16;

const TYPE_NAMES: Record<number, string> = {1:'Int',2:'Float',3:'Bool',4:'Str',5:'Bytes'};

// ── CRC32 ────────────────────────────────────────────────────────
const CRC_TABLE = new Uint32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
  CRC_TABLE[i] = c >>> 0;
}
function crc32(d: Uint8Array, off: number, len: number): number {
  let crc = 0xFFFFFFFF;
  for (let i = off; i < off + len; i++) crc = CRC_TABLE[(crc ^ d[i]) & 0xFF] ^ (crc >>> 8);
  return (crc ^ 0xFFFFFFFF) >>> 0;
}

// ── LE reads ─────────────────────────────────────────────────────
function rd16(d: Uint8Array, o: number): number { return d[o] | (d[o+1] << 8); }
function rd32(d: Uint8Array, o: number): number { return (d[o] | (d[o+1]<<8) | (d[o+2]<<16) | (d[o+3]<<24)) >>> 0; }
function rd64(d: Uint8Array, o: number): bigint {
  return BigInt(rd32(d, o)) | (BigInt(rd32(d, o+4)) << 32n);
}

// ── Varint ───────────────────────────────────────────────────────
function readVarint(d: Uint8Array, pos: number): [number, number] {
  let result = 0, shift = 0;
  while (pos < d.length) {
    const b = d[pos++];
    result |= (b & 0x7F) << shift;
    if (!(b & 0x80)) break;
    shift += 7;
  }
  return [result >>> 0, pos];
}

function readVarintBig(d: Uint8Array, pos: number): [bigint, number] {
  let result = 0n, shift = 0n;
  while (pos < d.length) {
    const b = d[pos++];
    result |= BigInt(b & 0x7F) << shift;
    if (!(b & 0x80)) break;
    shift += 7n;
  }
  return [result, pos];
}

function zigzagDecode(v: bigint): bigint { return (v >> 1n) ^ -(v & 1n); }

function readZvar(d: Uint8Array, pos: number): [bigint, number] {
  const [v, p] = readVarintBig(d, pos);
  return [zigzagDecode(v), p];
}

// ── LZ77 ─────────────────────────────────────────────────────────
function lz77Decompress(d: Uint8Array): Uint8Array {
  const out: number[] = [];
  let i = 0;
  while (i < d.length) {
    if (d[i] === 0xFF && i + 4 < d.length) {
      const off = rd16(d, i + 1), length = rd16(d, i + 3);
      i += 5;
      if (off === 0 && length === 1) { out.push(0xFF); }
      else {
        const start = out.length - off;
        for (let j = 0; j < length; j++) out.push(out[start + j]);
      }
    } else { out.push(d[i++]); }
  }
  return new Uint8Array(out);
}

// ── Huffman ──────────────────────────────────────────────────────
function huffmanDecompress(d: Uint8Array): Uint8Array {
  if (d.length < 6) return new Uint8Array(0);
  let pos = 0;
  const origLen = rd32(d, pos); pos += 4;
  const nsyms = rd16(d, pos); pos += 2;
  if (nsyms === 0) return new Uint8Array(0);

  const symLens = new Uint8Array(256);
  if (nsyms <= 64) {
    for (let s = 0; s < nsyms && pos + 2 <= d.length; s++) {
      const sym = d[pos++]; symLens[sym] = d[pos++];
    }
  } else {
    if (pos + 256 > d.length) return new Uint8Array(0);
    for (let s = 0; s < 256; s++) symLens[s] = d[pos + s];
    pos += 256;
  }

  let maxBits = 0;
  for (let s = 0; s < 256; s++) if (symLens[s] > maxBits) maxBits = symLens[s];
  if (maxBits === 0 || maxBits > 24) return new Uint8Array(0);

  const sorted: Array<{sym: number, cl: number}> = [];
  for (let s = 0; s < 256; s++) if (symLens[s] > 0) sorted.push({sym: s, cl: symLens[s]});
  sorted.sort((a, b) => a.cl !== b.cl ? a.cl - b.cl : a.sym - b.sym);

  const tableSize = 1 << maxBits;
  const lookup = new Int16Array(tableSize).fill(-1);
  const lookupLen = new Uint8Array(tableSize);

  let code = 0, prevLen = 0;
  for (const sl of sorted) {
    if (sl.cl > prevLen) { code <<= (sl.cl - prevLen); prevLen = sl.cl; }
    let rev = 0;
    for (let b = 0; b < sl.cl; b++) if (code & (1 << (sl.cl - 1 - b))) rev |= (1 << b);
    const fill = 1 << (maxBits - sl.cl);
    for (let f = 0; f < fill; f++) {
      const idx = (f << sl.cl) | rev;
      lookup[idx] = sl.sym; lookupLen[idx] = sl.cl;
    }
    code++;
  }

  const out = new Uint8Array(origLen);
  let oi = 0, bitbuf = 0, bitsIn = 0;
  while (oi < origLen && (pos < d.length || bitsIn >= maxBits)) {
    while (bitsIn < 24 && pos < d.length) { bitbuf |= d[pos++] << bitsIn; bitsIn += 8; }
    if (bitsIn < 1) break;
    const lookBits = Math.min(bitsIn, maxBits);
    const idx = bitbuf & ((1 << lookBits) - 1);
    if (idx < tableSize && lookup[idx] >= 0) {
      out[oi++] = lookup[idx];
      const cl = lookupLen[idx]; bitbuf >>>= cl; bitsIn -= cl;
    } else break;
  }
  return out.slice(0, oi);
}

// ── Range Coder ──────────────────────────────────────────────────
function rangeDecompress(d: Uint8Array): Uint8Array {
  if (d.length < 2) return new Uint8Array(0);
  let p = 0;
  const active = rd16(d, p); p += 2;
  if (active === 0) return new Uint8Array(0);

  const norm = new Uint16Array(256);
  for (let i = 0; i < active && p < d.length; i++) {
    const sym = d[p++];
    if (p + 1 >= d.length) break;
    norm[sym] = rd16(d, p); p += 2;
  }
  if (p + 3 >= d.length) return new Uint8Array(0);
  const origLen = rd32(d, p); p += 4;

  if (active === 1) {
    let sym = 0;
    for (let i = 0; i < 256; i++) if (norm[i] > 0) { sym = i; break; }
    return new Uint8Array(origLen).fill(sym);
  }

  const cdf = new Uint32Array(257);
  for (let i = 0; i < 256; i++) cdf[i + 1] = cdf[i] + norm[i];
  const symLookup = new Uint8Array(RC_SCALE);
  for (let i = 0; i < 256; i++)
    if (norm[i] > 0) for (let j = cdf[i]; j < cdf[i + 1] && j < RC_SCALE; j++) symLookup[j] = i;

  let codeVal = 0;
  for (let i = 0; i < 4; i++) { codeVal = ((codeVal << 8) | (p < d.length ? d[p++] : 0)) >>> 0; }

  let low = 0, rng = 0xFFFFFFFF;
  const out = new Uint8Array(origLen);
  for (let i = 0; i < origLen; i++) {
    const r = (rng >>> 0) / RC_SCALE >>> 0;
    let offset = ((codeVal - low) >>> 0) / r >>> 0;
    if (offset >= RC_SCALE) offset = RC_SCALE - 1;
    const sym = symLookup[offset];
    low = (low + r * cdf[sym]) >>> 0;
    if (cdf[sym + 1] - cdf[sym] < RC_SCALE) rng = r * (cdf[sym + 1] - cdf[sym]);
    else rng = rng - r * cdf[sym];
    while ((rng >>> 0) < RC_BOT) {
      low = (low << 8) >>> 0; rng = (rng << 8) >>> 0;
      codeVal = ((codeVal << 8) | (p < d.length ? d[p++] : 0)) >>> 0;
    }
    out[i] = sym;
  }
  return out;
}

// ── Block Decompressor ───────────────────────────────────────────
function decompressBlock(d: Uint8Array): Uint8Array {
  if (d.length === 0) return new Uint8Array(0);
  const tag = d[0], payload = d.slice(1);
  switch (tag) {
    case 0x02: return payload;
    case 0x00: return lz77Decompress(payload);
    case 0x01: return lz77Decompress(huffmanDecompress(payload));
    case 0x03: return huffmanDecompress(payload);
    case 0x04: return rangeDecompress(payload);
    case 0x05: return lz77Decompress(rangeDecompress(payload));
    default:   return lz77Decompress(payload);
  }
}

// ── Codec Decoders ───────────────────────────────────────────────
function decodeDeltaInt(d: Uint8Array, nrows: number): bigint[] {
  const out: bigint[] = new Array(nrows);
  let pos = 0, acc: bigint;
  [acc, pos] = readZvar(d, pos); out[0] = acc;
  for (let i = 1; i < nrows; i++) {
    let delta: bigint; [delta, pos] = readZvar(d, pos);
    acc += delta; out[i] = acc;
  }
  return out;
}

function decodeRleInt(d: Uint8Array, nrows: number): bigint[] {
  const out: bigint[] = [];
  let pos = 0;
  while (out.length < nrows && pos < d.length) {
    let count: number, val: bigint;
    [count, pos] = readVarint(d, pos);
    [val, pos] = readZvar(d, pos);
    for (let c = 0; c < count && out.length < nrows; c++) out.push(val);
  }
  return out;
}

function decodeCdelta(d: Uint8Array, nrows: number): bigint[] {
  let pos = 0, base: bigint, step: bigint;
  [base, pos] = readZvar(d, pos);
  [step, pos] = readZvar(d, pos);
  const out: bigint[] = new Array(nrows);
  for (let i = 0; i < nrows; i++) out[i] = base + step * BigInt(i);
  return out;
}

function decodeFOR(d: Uint8Array, nrows: number): bigint[] {
  let pos = 0, minval: bigint;
  [minval, pos] = readZvar(d, pos);
  const bits = pos < d.length ? d[pos] : 0; pos++;
  const out: bigint[] = new Array(nrows);
  if (bits === 0) { out.fill(minval); return out; }
  let bitbuf = 0n, bitpos = 0;
  for (let i = 0; i < nrows; i++) {
    while (bitpos < bits && pos < d.length) { bitbuf |= BigInt(d[pos++]) << BigInt(bitpos); bitpos += 8; }
    const mask = (1n << BigInt(bits)) - 1n;
    out[i] = minval + (bitbuf & mask); bitbuf >>= BigInt(bits); bitpos -= bits;
  }
  return out;
}

function decodeBitpack(d: Uint8Array, nrows: number): boolean[] {
  const out: boolean[] = new Array(nrows);
  for (let i = 0; i < nrows; i++) {
    const byteIdx = i >> 3, bitIdx = i & 7;
    out[i] = byteIdx < d.length ? ((d[byteIdx] >> bitIdx) & 1) === 1 : false;
  }
  return out;
}

function decodeRleStr(d: Uint8Array, nrows: number): string[] {
  const out: string[] = [];
  let pos = 0;
  const dec = new TextDecoder();
  while (out.length < nrows && pos < d.length) {
    let count: number, slen: number;
    [count, pos] = readVarint(d, pos);
    [slen, pos] = readVarint(d, pos);
    const s = dec.decode(d.slice(pos, pos + slen)); pos += slen;
    for (let c = 0; c < count && out.length < nrows; c++) out.push(s);
  }
  return out;
}

function decodeBdict(d: Uint8Array, nrows: number): string[] {
  let pos = 0;
  let nuniq: number; [nuniq, pos] = readVarint(d, pos);
  const dec = new TextDecoder();
  const dict: string[] = [];
  for (let i = 0; i < nuniq; i++) {
    let slen: number; [slen, pos] = readVarint(d, pos);
    dict.push(dec.decode(d.slice(pos, pos + slen))); pos += slen;
  }
  const bits = pos < d.length ? d[pos] : 0; pos++;
  const out: string[] = new Array(nrows);
  if (bits === 0) { out.fill(nuniq > 0 ? dict[0] : ''); return out; }
  let bitbuf = 0, bitpos = 0;
  for (let i = 0; i < nrows; i++) {
    while (bitpos < bits && pos < d.length) { bitbuf |= d[pos++] << bitpos; bitpos += 8; }
    const mask = (1 << bits) - 1;
    const idx = bitbuf & mask; bitbuf >>>= bits; bitpos -= bits;
    out[i] = idx < nuniq ? dict[idx] : '';
  }
  return out;
}

function decodeRawStr(d: Uint8Array, nrows: number): string[] {
  const out: string[] = new Array(nrows);
  const dec = new TextDecoder();
  let pos = 0;
  for (let i = 0; i < nrows; i++) {
    if (pos >= d.length) { out[i] = ''; continue; }
    let slen: number; [slen, pos] = readVarint(d, pos);
    out[i] = dec.decode(d.slice(pos, pos + slen)); pos += slen;
  }
  return out;
}

// ── Schema types ─────────────────────────────────────────────────
export interface KColumn { name: string; ktype: number; encrypted: boolean; }
interface ColMeta { fileOffset: bigint; compLen: number; codec: number; nullCount: number; }

// ── KORE Reader ──────────────────────────────────────────────────
export class KoreReader {
  private data: Uint8Array;
  version: number = 0;
  ncols: number = 0;
  nrows: bigint = 0n;
  nchunks: number = 0;
  chunkSize: number = 0;
  created: bigint = 0n;
  columns: KColumn[] = [];
  dictionary: string[] = [];
  chunkNrows: number[] = [];
  private colMeta: ColMeta[][] = [];
  private delBitmap: bigint[] | null = null;
  private delCount: bigint = 0n;

  private constructor(data: Uint8Array) { this.data = data; }

  static open(path: string): KoreReader {
    const data = readFileSync(path);
    const r = new KoreReader(new Uint8Array(data));
    r.parse();
    r.loadDeleteBitmap(path);
    return r;
  }

  static fromBytes(data: Uint8Array): KoreReader {
    const r = new KoreReader(data);
    r.parse();
    return r;
  }

  private parse(): void {
    const d = this.data;
    if (d.length < HEADER_SIZE + 12) throw new Error('Not a valid KORE file');
    const dec = new TextDecoder();
    if (dec.decode(d.slice(0, 4)) !== KORE_MAGIC) throw new Error('Bad KORE magic');

    this.version   = d[4];
    this.ncols     = rd16(d, 6);
    this.nrows     = rd64(d, 8);
    this.nchunks   = rd32(d, 16);
    this.chunkSize = rd32(d, 20);
    this.created   = rd64(d, 24);

    let pos = HEADER_SIZE;
    const schemaCompLen = rd32(d, pos); pos += 4;
    const schemaRaw = decompressBlock(d.slice(pos, pos + schemaCompLen)); pos += schemaCompLen;

    let sp = 0;
    for (let i = 0; i < this.ncols; i++) {
      let nlen: number; [nlen, sp] = readVarint(schemaRaw, sp);
      const name = dec.decode(schemaRaw.slice(sp, sp + nlen)); sp += nlen;
      const ktype = sp < schemaRaw.length ? schemaRaw[sp] : 4; sp++;
      const encrypted = sp < schemaRaw.length ? schemaRaw[sp] !== 0 : false; sp++;
      this.columns.push({name, ktype, encrypted});
    }

    const dictCompLen = rd32(d, pos); pos += 4;
    const dictRaw = decompressBlock(d.slice(pos, pos + dictCompLen)); pos += dictCompLen;
    let dp = 0;
    let dc: number; [dc, dp] = readVarint(dictRaw, dp);
    for (let i = 0; i < dc; i++) {
      let slen: number; [slen, dp] = readVarint(dictRaw, dp);
      this.dictionary.push(dec.decode(dictRaw.slice(dp, dp + slen))); dp += slen;
    }

    const trailer = d.length - 12;
    const footerCompLen = rd32(d, trailer);
    const footerOffset = Number(rd64(d, trailer + 4));
    const fr = decompressBlock(d.slice(footerOffset, footerOffset + footerCompLen));

    let fp = 0;
    fp += 4; // ft_nchunks
    fp += 2; // ft_ncols
    for (let c = 0; c < this.nchunks; c++) { this.chunkNrows.push(rd32(fr, fp)); fp += 4; }

    for (let c = 0; c < this.nchunks; c++) {
      const chunk: ColMeta[] = [];
      for (let ci = 0; ci < this.ncols; ci++) {
        const fileOffset = rd64(fr, fp); fp += 8;
        const compLen = rd32(fr, fp); fp += 4;
        const codec = fp < fr.length ? fr[fp] : 0; fp++;
        const nullCount = rd32(fr, fp); fp += 4;
        // skip min/max i64
        let _v: bigint;
        [_v, fp] = readZvar(fr, fp);
        [_v, fp] = readZvar(fr, fp);
        // skip min/max str
        let slen: number;
        [slen, fp] = readVarint(fr, fp); fp += slen;
        [slen, fp] = readVarint(fr, fp); fp += slen;
        fp += 512; // bloom
        chunk.push({fileOffset, compLen, codec, nullCount});
      }
      this.colMeta.push(chunk);
    }
  }

  private loadDeleteBitmap(path: string): void {
    const delPath = path + '.del';
    if (!existsSync(delPath)) return;
    const bd = readFileSync(delPath);
    const d = new Uint8Array(bd);
    if (d.length < 16) return;
    const totalRows = rd64(d, 0);
    this.delCount = rd64(d, 8);
    const nwords = Number((totalRows + 63n) / 64n);
    this.delBitmap = [];
    for (let i = 0; i < nwords && 16 + i * 8 + 8 <= d.length; i++)
      this.delBitmap.push(rd64(d, 16 + i * 8));
  }

  private isDeleted(row: number): boolean {
    if (!this.delBitmap) return false;
    const word = Math.floor(row / 64);
    if (word >= this.delBitmap.length) return false;
    return ((this.delBitmap[word] >> BigInt(row % 64)) & 1n) === 1n;
  }

  colIndex(name: string): number {
    return this.columns.findIndex(c => c.name === name);
  }

  private decodeChunkCol(ci: number, chunk: number): Uint8Array {
    const cm = this.colMeta[chunk][ci];
    const off = Number(cm.fileOffset);
    const storedCrc = rd32(this.data, off);
    const compLen = rd32(this.data, off + 4);
    const compressed = this.data.slice(off + 8, off + 8 + compLen);
    if (crc32(compressed, 0, compressed.length) !== storedCrc)
      throw new Error(`CRC mismatch col ${ci} chunk ${chunk}`);
    return decompressBlock(compressed);
  }

  readIntColumn(name: string): bigint[] {
    const ci = this.colIndex(name); if (ci < 0) throw new Error(`Column "${name}" not found`);
    const out: bigint[] = [];
    let globalRow = 0;
    for (let c = 0; c < this.nchunks; c++) {
      const raw = this.decodeChunkCol(ci, c);
      const nr = this.chunkNrows[c];
      const cm = this.colMeta[c][ci];
      let chunk: bigint[];
      switch (cm.codec) {
        case CODEC_CDELTA: chunk = decodeCdelta(raw, nr); break;
        case CODEC_DELTA:  chunk = decodeDeltaInt(raw, nr); break;
        case CODEC_RLE:    chunk = decodeRleInt(raw, nr); break;
        case CODEC_FOR:    chunk = decodeFOR(raw, nr); break;
        default:           chunk = decodeDeltaInt(raw, nr); break;
      }
      for (const v of chunk) { if (!this.isDeleted(globalRow++)) out.push(v); }
    }
    return out;
  }

  readFloatColumn(name: string): number[] {
    const ci = this.colIndex(name); if (ci < 0) throw new Error(`Column "${name}" not found`);
    const out: number[] = [];
    let globalRow = 0;
    for (let c = 0; c < this.nchunks; c++) {
      const raw = this.decodeChunkCol(ci, c);
      const nr = this.chunkNrows[c];
      let scale = 10000;
      let dataStart = 0;
      if (raw.length >= 2 && raw[0] === 0xFE) {
        const se = raw[1];
        scale = [1, 10, 100, 1000, 10000][Math.min(se, 4)];
        dataStart = 2;
      }
      const codecData = raw.slice(dataStart);
      const cm = this.colMeta[c][ci];
      let chunk: bigint[];
      switch (cm.codec) {
        case CODEC_CDELTA: chunk = decodeCdelta(codecData, nr); break;
        case CODEC_DELTA:  chunk = decodeDeltaInt(codecData, nr); break;
        case CODEC_RLE:    chunk = decodeRleInt(codecData, nr); break;
        case CODEC_FOR:    chunk = decodeFOR(codecData, nr); break;
        default:           chunk = decodeDeltaInt(codecData, nr); break;
      }
      for (const v of chunk) { if (!this.isDeleted(globalRow++)) out.push(Number(v) / scale); }
    }
    return out;
  }

  readBoolColumn(name: string): boolean[] {
    const ci = this.colIndex(name); if (ci < 0) throw new Error(`Column "${name}" not found`);
    const out: boolean[] = [];
    let globalRow = 0;
    for (let c = 0; c < this.nchunks; c++) {
      const raw = this.decodeChunkCol(ci, c);
      const nr = this.chunkNrows[c];
      const chunk = decodeBitpack(raw, nr);
      for (const v of chunk) { if (!this.isDeleted(globalRow++)) out.push(v); }
    }
    return out;
  }

  readStrColumn(name: string): string[] {
    const ci = this.colIndex(name); if (ci < 0) throw new Error(`Column "${name}" not found`);
    const out: string[] = [];
    let globalRow = 0;
    for (let c = 0; c < this.nchunks; c++) {
      const raw = this.decodeChunkCol(ci, c);
      const nr = this.chunkNrows[c];
      const cm = this.colMeta[c][ci];
      let chunk: string[];
      switch (cm.codec) {
        case CODEC_RLE:      chunk = decodeRleStr(raw, nr); break;
        case CODEC_BDICT:    chunk = decodeBdict(raw, nr); break;
        case CODEC_HUFFDICT: chunk = decodeBdict(raw, nr); break;
        default:             chunk = decodeRawStr(raw, nr); break;
      }
      for (const v of chunk) { if (!this.isDeleted(globalRow++)) out.push(v); }
    }
    return out;
  }

  info(): string {
    const cols = this.columns.map(c => `${c.name}:${TYPE_NAMES[c.ktype] || '?'}`).join(', ');
    return `KORE v${this.version} | ${this.nrows} rows × ${this.ncols} cols | ${this.nchunks} chunks | ${this.data.length} bytes | [${cols}]`;
  }
}

// ── CLI test ─────────────────────────────────────────────────────
{
  const path = process.argv[2] || '../../test/test_v2.kore';
  const r = KoreReader.open(path);
  console.log(r.info());
  for (const col of r.columns) {
    console.log(`\n${col.name} (${TYPE_NAMES[col.ktype]}):`);
    switch (col.ktype) {
      case KTYPE_INT: {
        const vals = r.readIntColumn(col.name);
        vals.slice(0, 10).forEach((v, i) => console.log(`  [${i}] ${v}`));
        break;
      }
      case KTYPE_FLOAT: {
        const vals = r.readFloatColumn(col.name);
        vals.slice(0, 10).forEach((v, i) => console.log(`  [${i}] ${v.toFixed(4)}`));
        break;
      }
      case KTYPE_BOOL: {
        const vals = r.readBoolColumn(col.name);
        vals.slice(0, 10).forEach((v, i) => console.log(`  [${i}] ${v}`));
        break;
      }
      case KTYPE_STR: {
        const vals = r.readStrColumn(col.name);
        vals.slice(0, 10).forEach((v, i) => console.log(`  [${i}] "${v}"`));
        break;
      }
    }
  }
  console.log('\nDONE');
}
