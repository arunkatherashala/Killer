// Package kore implements a pure-Go reader for the KORE v2 columnar file format.
// Zero external dependencies.
//
// Usage:
//
//	f, err := kore.Open("data.kore")
//	fmt.Println(f.Info())
//	ints, _ := f.ReadIntColumn("id")
//	floats, _ := f.ReadFloatColumn("price")
//	strs, _ := f.ReadStrColumn("name")
//	bools, _ := f.ReadBoolColumn("active")
package kore

import (
	"encoding/binary"
	"fmt"
	"math"
	"os"
	"strings"
)

// Column types
const (
	KTypeInt   = 1
	KTypeFloat = 2
	KTypeBool  = 3
	KTypeStr   = 4
	KTypeBytes = 5
)

// Codecs
const (
	CodecRaw     = 0
	CodecRLE     = 1
	CodecDelta   = 2
	CodecDictRLE = 3
	CodecBitpack = 4
	CodecBDict   = 5
	CodecCDelta  = 6
	CodecFOR     = 7
	CodecHuffDict = 8
)

const (
	headerSize = 64
	rcScale    = 4096
	rcBot      = 1 << 16
)

// Column describes a column in the schema.
type Column struct {
	Name      string
	KType     uint8
	Encrypted bool
}

func (c Column) TypeName() string {
	switch c.KType {
	case 1: return "Int"
	case 2: return "Float"
	case 3: return "Bool"
	case 4: return "Str"
	case 5: return "Bytes"
	default: return "?"
	}
}

type colMeta struct {
	FileOffset uint64
	CompLen    uint32
	Codec      uint8
	NullCount  uint32
	MinI64     int64
	MaxI64     int64
}

// File is a parsed KORE v2 file.
type File struct {
	data       []byte
	Version    uint8
	NCols      int
	NRows      uint64
	NChunks    int
	ChunkSize  uint32
	Created    uint64
	Columns    []Column
	Dictionary []string
	ChunkNRows []uint32
	colMeta    [][]colMeta // [chunk][col]
	delBitmap  []uint64
	delCount   uint64
}

// ── CRC32 ────────────────────────────────────────────────────────
var crcTable [256]uint32

func init() {
	for i := 0; i < 256; i++ {
		c := uint32(i)
		for j := 0; j < 8; j++ {
			if c&1 != 0 {
				c = 0xEDB88320 ^ (c >> 1)
			} else {
				c >>= 1
			}
		}
		crcTable[i] = c
	}
}

func crc32(d []byte) uint32 {
	crc := uint32(0xFFFFFFFF)
	for _, b := range d {
		crc = crcTable[(crc^uint32(b))&0xFF] ^ (crc >> 8)
	}
	return crc ^ 0xFFFFFFFF
}

// ── LE reads ─────────────────────────────────────────────────────
func rd16(d []byte, o int) uint16 { return binary.LittleEndian.Uint16(d[o:]) }
func rd32(d []byte, o int) uint32 { return binary.LittleEndian.Uint32(d[o:]) }
func rd64(d []byte, o int) uint64 { return binary.LittleEndian.Uint64(d[o:]) }

// ── Varint ───────────────────────────────────────────────────────
func readVarint(d []byte, pos int) (uint64, int) {
	var result uint64
	shift := 0
	for pos < len(d) {
		b := d[pos]; pos++
		result |= uint64(b&0x7F) << shift
		if b&0x80 == 0 { break }
		shift += 7
	}
	return result, pos
}

func zigzagDecode(v uint64) int64 {
	return int64((v >> 1) ^ -(v & 1))
}

func readZvar(d []byte, pos int) (int64, int) {
	v, p := readVarint(d, pos)
	return zigzagDecode(v), p
}

// ── LZ77 ─────────────────────────────────────────────────────────
func lz77Decompress(d []byte) []byte {
	out := make([]byte, 0, len(d)*4)
	i := 0
	for i < len(d) {
		if d[i] == 0xFF && i+4 < len(d) {
			off := int(rd16(d, i+1))
			length := int(rd16(d, i+3))
			i += 5
			if off == 0 && length == 1 {
				out = append(out, 0xFF)
			} else {
				start := len(out) - off
				for j := 0; j < length; j++ {
					out = append(out, out[start+j])
				}
			}
		} else {
			out = append(out, d[i])
			i++
		}
	}
	return out
}

// ── Huffman ──────────────────────────────────────────────────────
func huffmanDecompress(d []byte) []byte {
	if len(d) < 6 { return nil }
	pos := 0
	origLen := int(rd32(d, pos)); pos += 4
	nsyms := int(rd16(d, pos)); pos += 2
	if nsyms == 0 { return nil }

	symLens := make([]int, 256)
	if nsyms <= 64 {
		for s := 0; s < nsyms && pos+2 <= len(d); s++ {
			sym := d[pos]; pos++
			symLens[sym] = int(d[pos]); pos++
		}
	} else {
		if pos+256 > len(d) { return nil }
		for s := 0; s < 256; s++ { symLens[s] = int(d[pos+s]) }
		pos += 256
	}

	maxBits := 0
	for _, l := range symLens { if l > maxBits { maxBits = l } }
	if maxBits == 0 || maxBits > 24 { return nil }

	type symLen struct{ sym, cl int }
	var sorted []symLen
	for s := 0; s < 256; s++ {
		if symLens[s] > 0 { sorted = append(sorted, symLen{s, symLens[s]}) }
	}
	// Sort by (cl, sym)
	for i := 1; i < len(sorted); i++ {
		tmp := sorted[i]; j := i - 1
		for j >= 0 && (sorted[j].cl > tmp.cl || (sorted[j].cl == tmp.cl && sorted[j].sym > tmp.sym)) {
			sorted[j+1] = sorted[j]; j--
		}
		sorted[j+1] = tmp
	}

	tableSize := 1 << maxBits
	lookup := make([]int, tableSize)
	lookupLen := make([]int, tableSize)
	for i := range lookup { lookup[i] = -1 }

	code := 0; prevLen := 0
	for _, sl := range sorted {
		cl := sl.cl
		if cl > prevLen { code <<= (cl - prevLen); prevLen = cl }
		rev := 0
		for b := 0; b < cl; b++ { if code&(1<<(cl-1-b)) != 0 { rev |= 1 << b } }
		fill := 1 << (maxBits - cl)
		for f := 0; f < fill; f++ {
			idx := (f << cl) | rev
			lookup[idx] = sl.sym
			lookupLen[idx] = cl
		}
		code++
	}

	out := make([]byte, 0, origLen)
	var bitbuf uint64; bitsIn := 0
	for len(out) < origLen && (pos < len(d) || bitsIn >= maxBits) {
		for bitsIn < 56 && pos < len(d) {
			bitbuf |= uint64(d[pos]) << bitsIn; pos++; bitsIn += 8
		}
		if bitsIn < 1 { break }
		lookBits := bitsIn; if lookBits > maxBits { lookBits = maxBits }
		idx := int(bitbuf & ((1 << lookBits) - 1))
		if idx < tableSize && lookup[idx] >= 0 {
			out = append(out, byte(lookup[idx]))
			cl := lookupLen[idx]
			bitbuf >>= cl; bitsIn -= cl
		} else { break }
	}
	if len(out) > origLen { out = out[:origLen] }
	return out
}

// ── Range Coder ──────────────────────────────────────────────────
func rangeDecompress(d []byte) []byte {
	if len(d) < 2 { return nil }
	p := 0
	active := int(rd16(d, p)); p += 2
	if active == 0 { return nil }

	norm := make([]int, 256)
	for i := 0; i < active && p < len(d); i++ {
		sym := d[p]; p++
		if p+1 >= len(d) { break }
		norm[sym] = int(rd16(d, p)); p += 2
	}
	if p+3 >= len(d) { return nil }
	origLen := int(rd32(d, p)); p += 4

	if active == 1 {
		var sym byte
		for i := 0; i < 256; i++ { if norm[i] > 0 { sym = byte(i); break } }
		out := make([]byte, origLen)
		for i := range out { out[i] = sym }
		return out
	}

	cdf := make([]int, 257)
	for i := 0; i < 256; i++ { cdf[i+1] = cdf[i] + norm[i] }
	symLookup := make([]byte, rcScale)
	for i := 0; i < 256; i++ {
		if norm[i] > 0 {
			for j := cdf[i]; j < cdf[i+1] && j < rcScale; j++ { symLookup[j] = byte(i) }
		}
	}

	var codeVal uint32
	for i := 0; i < 4; i++ {
		b := byte(0); if p < len(d) { b = d[p]; p++ }
		codeVal = (codeVal << 8) | uint32(b)
	}

	var low, rng uint32
	rng = 0xFFFFFFFF
	out := make([]byte, origLen)
	for i := 0; i < origLen; i++ {
		r := rng / uint32(rcScale)
		offset := (codeVal - low) / r
		if offset >= uint32(rcScale) { offset = uint32(rcScale) - 1 }
		sym := symLookup[offset]
		si := int(sym)
		low += r * uint32(cdf[si])
		if cdf[si+1]-cdf[si] < rcScale { rng = r * uint32(cdf[si+1]-cdf[si]) } else { rng -= r * uint32(cdf[si]) }
		for rng < uint32(rcBot) {
			low <<= 8; rng <<= 8
			b := byte(0); if p < len(d) { b = d[p]; p++ }
			codeVal = (codeVal << 8) | uint32(b)
		}
		out[i] = sym
	}
	return out
}

// ── Block Decompressor ───────────────────────────────────────────
func decompressBlock(d []byte) []byte {
	if len(d) == 0 { return nil }
	tag := d[0]; payload := d[1:]
	switch tag {
	case 0x02: return append([]byte(nil), payload...)
	case 0x00: return lz77Decompress(payload)
	case 0x01: return lz77Decompress(huffmanDecompress(payload))
	case 0x03: return huffmanDecompress(payload)
	case 0x04: return rangeDecompress(payload)
	case 0x05: return lz77Decompress(rangeDecompress(payload))
	default:   return lz77Decompress(payload)
	}
}

// ── Codec Decoders ───────────────────────────────────────────────
func decodeDeltaInt(d []byte, nrows int) []int64 {
	out := make([]int64, nrows)
	pos := 0; var acc int64
	acc, pos = readZvar(d, pos)
	out[0] = acc
	for i := 1; i < nrows; i++ {
		var delta int64; delta, pos = readZvar(d, pos)
		acc += delta; out[i] = acc
	}
	return out
}

func decodeRleInt(d []byte, nrows int) []int64 {
	out := make([]int64, 0, nrows)
	pos := 0
	for len(out) < nrows && pos < len(d) {
		count, p1 := readVarint(d, pos); pos = p1
		val, p2 := readZvar(d, pos); pos = p2
		for c := uint64(0); c < count && len(out) < nrows; c++ { out = append(out, val) }
	}
	return out
}

func decodeCdelta(d []byte, nrows int) []int64 {
	pos := 0
	base, p := readZvar(d, pos); pos = p
	step, _ := readZvar(d, pos)
	out := make([]int64, nrows)
	for i := 0; i < nrows; i++ { out[i] = base + step*int64(i) }
	return out
}

func decodeFOR(d []byte, nrows int) []int64 {
	pos := 0
	minval, p := readZvar(d, pos); pos = p
	bits := 0; if pos < len(d) { bits = int(d[pos]); pos++ }
	out := make([]int64, nrows)
	if bits == 0 { for i := range out { out[i] = minval }; return out }
	var bitbuf uint64; bitpos := 0
	for i := 0; i < nrows; i++ {
		for bitpos < bits && pos < len(d) { bitbuf |= uint64(d[pos]) << bitpos; pos++; bitpos += 8 }
		mask := uint64((1 << bits) - 1)
		out[i] = minval + int64(bitbuf&mask); bitbuf >>= bits; bitpos -= bits
	}
	return out
}

func decodeBitpack(d []byte, nrows int) []bool {
	out := make([]bool, nrows)
	for i := 0; i < nrows; i++ {
		byteIdx, bitIdx := i/8, uint(i%8)
		if byteIdx < len(d) { out[i] = (d[byteIdx]>>bitIdx)&1 == 1 }
	}
	return out
}

func decodeRleStr(d []byte, nrows int) []string {
	out := make([]string, 0, nrows)
	pos := 0
	for len(out) < nrows && pos < len(d) {
		count, p1 := readVarint(d, pos); pos = p1
		slen, p2 := readVarint(d, pos); pos = p2
		s := string(d[pos : pos+int(slen)]); pos += int(slen)
		for c := uint64(0); c < count && len(out) < nrows; c++ { out = append(out, s) }
	}
	return out
}

func decodeBdict(d []byte, nrows int) []string {
	pos := 0
	nuniq, p := readVarint(d, pos); pos = p
	dict := make([]string, nuniq)
	for i := uint64(0); i < nuniq; i++ {
		slen, p2 := readVarint(d, pos); pos = p2
		dict[i] = string(d[pos : pos+int(slen)]); pos += int(slen)
	}
	bits := 0; if pos < len(d) { bits = int(d[pos]); pos++ }
	out := make([]string, nrows)
	if bits == 0 {
		def := ""; if nuniq > 0 { def = dict[0] }
		for i := range out { out[i] = def }
		return out
	}
	var bitbuf uint64; bitpos := 0
	for i := 0; i < nrows; i++ {
		for bitpos < bits && pos < len(d) { bitbuf |= uint64(d[pos]) << bitpos; pos++; bitpos += 8 }
		mask := uint64((1 << bits) - 1)
		idx := bitbuf & mask; bitbuf >>= bits; bitpos -= bits
		if idx < nuniq { out[i] = dict[idx] } else { out[i] = "" }
	}
	return out
}

func decodeRawStr(d []byte, nrows int) []string {
	out := make([]string, nrows)
	pos := 0
	for i := 0; i < nrows; i++ {
		if pos >= len(d) { continue }
		slen, p := readVarint(d, pos); pos = p
		out[i] = string(d[pos : pos+int(slen)]); pos += int(slen)
	}
	return out
}

// ── Open / Parse ─────────────────────────────────────────────────

// Open reads and parses a KORE v2 file from disk.
func Open(path string) (*File, error) {
	data, err := os.ReadFile(path)
	if err != nil { return nil, err }
	f := &File{data: data}
	if err := f.parse(); err != nil { return nil, err }
	f.loadDeleteBitmap(path)
	return f, nil
}

// FromBytes parses KORE v2 data from a byte slice.
func FromBytes(data []byte) (*File, error) {
	f := &File{data: data}
	return f, f.parse()
}

func (f *File) parse() error {
	d := f.data
	if len(d) < headerSize+12 { return fmt.Errorf("kore: file too short") }
	if string(d[:4]) != "KORE" { return fmt.Errorf("kore: bad magic") }

	f.Version   = d[4]
	f.NCols     = int(rd16(d, 6))
	f.NRows     = rd64(d, 8)
	f.NChunks   = int(rd32(d, 16))
	f.ChunkSize = rd32(d, 20)
	f.Created   = rd64(d, 24)

	pos := headerSize
	schemaCompLen := int(rd32(d, pos)); pos += 4
	schemaRaw := decompressBlock(d[pos : pos+schemaCompLen]); pos += schemaCompLen

	f.Columns = make([]Column, f.NCols)
	sp := 0
	for i := 0; i < f.NCols; i++ {
		nlen, p := readVarint(schemaRaw, sp); sp = p
		name := string(schemaRaw[sp : sp+int(nlen)]); sp += int(nlen)
		ktype := uint8(4); if sp < len(schemaRaw) { ktype = schemaRaw[sp] }; sp++
		enc := false; if sp < len(schemaRaw) { enc = schemaRaw[sp] != 0 }; sp++
		f.Columns[i] = Column{name, ktype, enc}
	}

	dictCompLen := int(rd32(d, pos)); pos += 4
	dictRaw := decompressBlock(d[pos : pos+dictCompLen]); pos += dictCompLen
	dp := 0
	dc, p := readVarint(dictRaw, dp); dp = p
	f.Dictionary = make([]string, dc)
	for i := uint64(0); i < dc; i++ {
		slen, p2 := readVarint(dictRaw, dp); dp = p2
		f.Dictionary[i] = string(dictRaw[dp : dp+int(slen)]); dp += int(slen)
	}

	trailer := len(d) - 12
	footerCompLen := int(rd32(d, trailer))
	footerOffset := int(rd64(d, trailer+4))
	fr := decompressBlock(d[footerOffset : footerOffset+footerCompLen])

	fp := 0
	_ = rd32(fr, fp); fp += 4 // ft_nchunks
	_ = rd16(fr, fp); fp += 2 // ft_ncols
	f.ChunkNRows = make([]uint32, f.NChunks)
	for c := 0; c < f.NChunks; c++ { f.ChunkNRows[c] = rd32(fr, fp); fp += 4 }

	f.colMeta = make([][]colMeta, f.NChunks)
	for c := 0; c < f.NChunks; c++ {
		f.colMeta[c] = make([]colMeta, f.NCols)
		for ci := 0; ci < f.NCols; ci++ {
			cm := &f.colMeta[c][ci]
			cm.FileOffset = rd64(fr, fp); fp += 8
			cm.CompLen = rd32(fr, fp); fp += 4
			if fp < len(fr) { cm.Codec = fr[fp] }; fp++
			cm.NullCount = rd32(fr, fp); fp += 4
			cm.MinI64, fp = readZvar(fr, fp)
			cm.MaxI64, fp = readZvar(fr, fp)
			slen, p2 := readVarint(fr, fp); fp = p2 + int(slen)
			slen, p2 = readVarint(fr, fp); fp = p2 + int(slen)
			fp += 512 // bloom filter
		}
	}
	return nil
}

func (f *File) loadDeleteBitmap(path string) {
	data, err := os.ReadFile(path + ".del")
	if err != nil || len(data) < 16 { return }
	totalRows := rd64(data, 0)
	f.delCount = rd64(data, 8)
	nwords := (totalRows + 63) / 64
	f.delBitmap = make([]uint64, nwords)
	for i := uint64(0); i < nwords && 16+i*8+8 <= uint64(len(data)); i++ {
		f.delBitmap[i] = rd64(data, int(16+i*8))
	}
}

func (f *File) isDeleted(row uint64) bool {
	if f.delBitmap == nil { return false }
	word := row / 64
	if word >= uint64(len(f.delBitmap)) { return false }
	return (f.delBitmap[word]>>(row%64))&1 == 1
}

// ── Chunk decoder ────────────────────────────────────────────────
func (f *File) decodeChunkCol(ci, chunk int) []byte {
	cm := f.colMeta[chunk][ci]
	off := int(cm.FileOffset)
	storedCrc := rd32(f.data, off)
	compLen := int(rd32(f.data, off+4))
	compressed := f.data[off+8 : off+8+compLen]
	if crc32(compressed) != storedCrc {
		panic(fmt.Sprintf("kore: CRC mismatch col %d chunk %d", ci, chunk))
	}
	return decompressBlock(compressed)
}

// ColIndex returns the index of a column by name, or -1.
func (f *File) ColIndex(name string) int {
	for i, c := range f.Columns { if c.Name == name { return i } }
	return -1
}

// ReadIntColumn reads an Int column and returns the values.
func (f *File) ReadIntColumn(name string) ([]int64, error) {
	ci := f.ColIndex(name); if ci < 0 { return nil, fmt.Errorf("kore: column %q not found", name) }
	return f.readIntCol(ci), nil
}

func (f *File) readIntCol(ci int) []int64 {
	var out []int64
	globalRow := uint64(0)
	for c := 0; c < f.NChunks; c++ {
		raw := f.decodeChunkCol(ci, c)
		nr := int(f.ChunkNRows[c])
		var chunk []int64
		switch f.colMeta[c][ci].Codec {
		case CodecCDelta: chunk = decodeCdelta(raw, nr)
		case CodecDelta:  chunk = decodeDeltaInt(raw, nr)
		case CodecRLE:    chunk = decodeRleInt(raw, nr)
		case CodecFOR:    chunk = decodeFOR(raw, nr)
		default:          chunk = decodeDeltaInt(raw, nr)
		}
		for _, v := range chunk {
			if !f.isDeleted(globalRow) { out = append(out, v) }
			globalRow++
		}
	}
	return out
}

// ReadFloatColumn reads a Float column.
func (f *File) ReadFloatColumn(name string) ([]float64, error) {
	ci := f.ColIndex(name); if ci < 0 { return nil, fmt.Errorf("kore: column %q not found", name) }
	return f.readFloatCol(ci), nil
}

func (f *File) readFloatCol(ci int) []float64 {
	var out []float64
	globalRow := uint64(0)
	for c := 0; c < f.NChunks; c++ {
		raw := f.decodeChunkCol(ci, c)
		nr := int(f.ChunkNRows[c])
		scale := 10000.0
		dataStart := 0
		if len(raw) >= 2 && raw[0] == 0xFE {
			se := int(raw[1])
			muls := []float64{1, 10, 100, 1000, 10000}
			if se <= 4 { scale = muls[se] } else { scale = 10000 }
			dataStart = 2
		}
		codecData := raw[dataStart:]
		var chunk []int64
		switch f.colMeta[c][ci].Codec {
		case CodecCDelta: chunk = decodeCdelta(codecData, nr)
		case CodecDelta:  chunk = decodeDeltaInt(codecData, nr)
		case CodecRLE:    chunk = decodeRleInt(codecData, nr)
		case CodecFOR:    chunk = decodeFOR(codecData, nr)
		default:          chunk = decodeDeltaInt(codecData, nr)
		}
		for _, v := range chunk {
			if !f.isDeleted(globalRow) { out = append(out, float64(v)/scale) }
			globalRow++
		}
	}
	return out
}

// ReadBoolColumn reads a Bool column.
func (f *File) ReadBoolColumn(name string) ([]bool, error) {
	ci := f.ColIndex(name); if ci < 0 { return nil, fmt.Errorf("kore: column %q not found", name) }
	return f.readBoolCol(ci), nil
}

func (f *File) readBoolCol(ci int) []bool {
	var out []bool
	globalRow := uint64(0)
	for c := 0; c < f.NChunks; c++ {
		raw := f.decodeChunkCol(ci, c)
		nr := int(f.ChunkNRows[c])
		chunk := decodeBitpack(raw, nr)
		for _, v := range chunk {
			if !f.isDeleted(globalRow) { out = append(out, v) }
			globalRow++
		}
	}
	return out
}

// ReadStrColumn reads a Str column.
func (f *File) ReadStrColumn(name string) ([]string, error) {
	ci := f.ColIndex(name); if ci < 0 { return nil, fmt.Errorf("kore: column %q not found", name) }
	return f.readStrCol(ci), nil
}

func (f *File) readStrCol(ci int) []string {
	var out []string
	globalRow := uint64(0)
	for c := 0; c < f.NChunks; c++ {
		raw := f.decodeChunkCol(ci, c)
		nr := int(f.ChunkNRows[c])
		var chunk []string
		switch f.colMeta[c][ci].Codec {
		case CodecRLE:      chunk = decodeRleStr(raw, nr)
		case CodecBDict:    chunk = decodeBdict(raw, nr)
		case CodecHuffDict: chunk = decodeBdict(raw, nr)
		default:            chunk = decodeRawStr(raw, nr)
		}
		for _, v := range chunk {
			if !f.isDeleted(globalRow) { out = append(out, v) }
			globalRow++
		}
	}
	return out
}

// Info returns a human-readable summary.
func (f *File) Info() string {
	var cols []string
	for _, c := range f.Columns { cols = append(cols, c.Name+":"+c.TypeName()) }
	return fmt.Sprintf("KORE v%d | %d rows × %d cols | %d chunks | %d bytes | [%s]",
		f.Version, f.NRows, f.NCols, f.NChunks, len(f.data), strings.Join(cols, ", "))
}

// Suppress unused import warnings
var _ = math.MaxFloat64
