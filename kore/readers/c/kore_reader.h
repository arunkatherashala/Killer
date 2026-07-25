/*
 * KORE v2 Reader — Pure C (C99) — Zero Dependencies
 * ===================================================
 * Single-header library. #define KORE_IMPLEMENTATION in ONE .c file before including.
 *
 * Usage:
 *   #define KORE_IMPLEMENTATION
 *   #include "kore_reader.h"
 *
 *   KoreFile kf;
 *   if (kore_open(&kf, "data.kore") != 0) { ... error ... }
 *   printf("%llu rows x %d cols\n", kf.nrows, kf.ncols);
 *   KoreColumn col;
 *   kore_read_column(&kf, "price", &col);
 *   for (int i = 0; i < col.len; i++) printf("%f\n", col.floats[i]);
 *   kore_free_column(&col);
 *   kore_close(&kf);
 */
#ifndef KORE_READER_H
#define KORE_READER_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Constants ────────────────────────────────────────────────────── */
#define KORE_MAGIC      0x45524F4B  /* "KORE" LE */
#define KORE_V2         2
#define KORE_HEADER_LEN 64

/* Column types */
#define KTYPE_INT   1
#define KTYPE_FLOAT 2
#define KTYPE_BOOL  3
#define KTYPE_STR   4
#define KTYPE_BYTES 5

/* Codecs */
#define KCODEC_RAW     0
#define KCODEC_RLE     1
#define KCODEC_DELTA   2
#define KCODEC_DICTRLE 3
#define KCODEC_BITPACK 4
#define KCODEC_BDICT   5
#define KCODEC_CDELTA  6
#define KCODEC_FOR     7
#define KCODEC_HUFFDICT 8
#define KCODEC_DERIVED 9

/* ── Types ────────────────────────────────────────────────────────── */
typedef struct {
    char    name[256];
    uint8_t ktype;
    uint8_t encrypted;
} KoreSchema;

typedef struct {
    uint64_t file_offset;
    uint32_t comp_len;
    uint8_t  codec;
    uint32_t null_count;
    int64_t  min_i64, max_i64;
} KoreColMeta;

typedef struct {
    uint8_t  *data;
    size_t    data_len;
    uint8_t   version;
    uint16_t  ncols;
    uint64_t  nrows;
    uint32_t  nchunks;
    uint32_t  chunk_size;
    uint64_t  created;
    KoreSchema *schema;       /* [ncols] */
    char     **dictionary;    /* global string dict */
    uint32_t   dict_count;
    uint32_t  *chunk_nrows;   /* [nchunks] */
    KoreColMeta **col_meta;   /* [nchunks][ncols] */
    /* Delete bitmap */
    uint64_t  *del_bitmap;
    uint64_t   del_count;
    int        has_del;
} KoreFile;

/* Decoded column (caller must free via kore_free_column) */
typedef struct {
    uint8_t   ktype;
    uint64_t  len;
    union {
        int64_t *ints;
        double  *floats;
        uint8_t *bools;
        char   **strings;
    };
} KoreColumn;

/* ── API ──────────────────────────────────────────────────────────── */
int  kore_open(KoreFile *kf, const char *path);
int  kore_open_mem(KoreFile *kf, const uint8_t *data, size_t len);
void kore_close(KoreFile *kf);

int  kore_read_column(KoreFile *kf, const char *col_name, KoreColumn *out);
int  kore_read_column_idx(KoreFile *kf, int col_idx, KoreColumn *out);
void kore_free_column(KoreColumn *col);

int  kore_col_index(KoreFile *kf, const char *name);
const char *kore_type_name(uint8_t ktype);

#ifdef __cplusplus
}
#endif

/* ══════════════════════════════════════════════════════════════════ */
/*                          IMPLEMENTATION                          */
/* ══════════════════════════════════════════════════════════════════ */
#ifdef KORE_IMPLEMENTATION

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── Helpers ──────────────────────────────────────────────────────── */
static uint16_t rd16(const uint8_t *p) { return (uint16_t)p[0] | ((uint16_t)p[1]<<8); }
static uint32_t rd32(const uint8_t *p) { return (uint32_t)p[0]|((uint32_t)p[1]<<8)|((uint32_t)p[2]<<16)|((uint32_t)p[3]<<24); }
static uint64_t rd64(const uint8_t *p) { return (uint64_t)rd32(p) | ((uint64_t)rd32(p+4)<<32); }

static size_t read_varint(const uint8_t *d, size_t pos, size_t dlen, uint64_t *out) {
    uint64_t result = 0; int shift = 0;
    while (pos < dlen) {
        uint8_t b = d[pos++];
        result |= (uint64_t)(b & 0x7F) << shift;
        if (!(b & 0x80)) break;
        shift += 7;
    }
    *out = result;
    return pos;
}

static int64_t zigzag_decode(uint64_t v) { return (int64_t)((v >> 1) ^ -(int64_t)(v & 1)); }

static size_t read_zvar(const uint8_t *d, size_t pos, size_t dlen, int64_t *out) {
    uint64_t v;
    pos = read_varint(d, pos, dlen, &v);
    *out = zigzag_decode(v);
    return pos;
}

/* ── CRC32 ────────────────────────────────────────────────────────── */
static uint32_t kore_crc_table[256];
static int kore_crc_init = 0;

static void kore_init_crc(void) {
    if (kore_crc_init) return;
    for (int i = 0; i < 256; i++) {
        uint32_t c = (uint32_t)i;
        for (int j = 0; j < 8; j++)
            c = (c & 1) ? (0xEDB88320u ^ (c >> 1)) : (c >> 1);
        kore_crc_table[i] = c;
    }
    kore_crc_init = 1;
}

static uint32_t kore_crc32(const uint8_t *d, size_t len) {
    kore_init_crc();
    uint32_t crc = 0xFFFFFFFF;
    for (size_t i = 0; i < len; i++)
        crc = kore_crc_table[(crc ^ d[i]) & 0xFF] ^ (crc >> 8);
    return crc ^ 0xFFFFFFFF;
}

/* ── LZ77 Decompress ─────────────────────────────────────────────── */
static uint8_t *lz77_decompress(const uint8_t *d, size_t dlen, size_t *out_len) {
    size_t cap = dlen * 4, sz = 0;
    uint8_t *out = (uint8_t *)malloc(cap);
    if (!out) return NULL;
    size_t i = 0;
    while (i < dlen) {
        if (d[i] == 0xFF && i + 4 < dlen) {
            uint16_t off = rd16(d + i + 1);
            uint16_t length = rd16(d + i + 3);
            i += 5;
            if (off == 0 && length == 1) {
                if (sz >= cap) { cap *= 2; out = (uint8_t *)realloc(out, cap); }
                out[sz++] = 0xFF;
            } else {
                size_t start = sz - off;
                for (uint16_t j = 0; j < length; j++) {
                    if (sz >= cap) { cap *= 2; out = (uint8_t *)realloc(out, cap); }
                    out[sz] = out[start + j];
                    sz++;
                }
            }
        } else {
            if (sz >= cap) { cap *= 2; out = (uint8_t *)realloc(out, cap); }
            out[sz++] = d[i++];
        }
    }
    *out_len = sz;
    return out;
}

/* ── Huffman Decompress ───────────────────────────────────────────── */
static uint8_t *huffman_decompress(const uint8_t *d, size_t dlen, size_t *out_len) {
    if (dlen < 6) { *out_len = 0; return NULL; }
    size_t pos = 0;
    uint32_t orig_len = rd32(d); pos += 4;
    uint16_t nsyms = rd16(d + pos); pos += 2;
    if (nsyms == 0) { *out_len = 0; return (uint8_t *)calloc(1, 1); }

    uint16_t sym_lens[256]; memset(sym_lens, 0, sizeof(sym_lens));
    if (nsyms <= 64) {
        for (int s = 0; s < nsyms && pos + 2 <= dlen; s++) {
            uint8_t sym = d[pos++]; sym_lens[sym] = d[pos++];
        }
    } else {
        if (pos + 256 > dlen) { *out_len = 0; return NULL; }
        for (int s = 0; s < 256; s++) sym_lens[s] = d[pos + s];
        pos += 256;
    }

    /* Find max bits */
    int max_bits = 0;
    for (int s = 0; s < 256; s++)
        if (sym_lens[s] > max_bits) max_bits = sym_lens[s];
    if (max_bits == 0 || max_bits > 24) { *out_len = 0; return NULL; }

    /* Sort symbols by (code_len, symbol) */
    typedef struct { uint8_t sym; uint8_t len; } SymLen;
    SymLen sorted[256]; int ns = 0;
    for (int s = 0; s < 256; s++)
        if (sym_lens[s] > 0) { sorted[ns].sym = (uint8_t)s; sorted[ns].len = sym_lens[s]; ns++; }
    /* Insertion sort (small n) */
    for (int i = 1; i < ns; i++) {
        SymLen tmp = sorted[i]; int j = i - 1;
        while (j >= 0 && (sorted[j].len > tmp.len || (sorted[j].len == tmp.len && sorted[j].sym > tmp.sym)))
            { sorted[j+1] = sorted[j]; j--; }
        sorted[j+1] = tmp;
    }

    /* Build lookup table: [code_len][reversed_code] → symbol */
    /* For speed, build a flat table up to max_bits */
    int table_size = 1 << max_bits;
    uint16_t *lookup = (uint16_t *)calloc(table_size, sizeof(uint16_t)); /* 0xFFFF = unused */
    memset(lookup, 0xFF, table_size * sizeof(uint16_t));

    uint32_t code = 0; int prev_len = 0;
    for (int i = 0; i < ns; i++) {
        int cl = sorted[i].len;
        if (cl > prev_len) { code <<= (cl - prev_len); prev_len = cl; }
        /* Reverse bits */
        uint32_t rev = 0;
        for (int b = 0; b < cl; b++)
            if (code & (1 << (cl - 1 - b))) rev |= (1 << b);
        /* Fill all table entries that share this prefix */
        int fill = 1 << (max_bits - cl);
        for (int f = 0; f < fill; f++)
            lookup[(f << cl) | rev] = sorted[i].sym;
        code++;
    }

    /* Decode bitstream */
    uint8_t *out = (uint8_t *)malloc(orig_len);
    size_t oi = 0;
    uint64_t bitbuf = 0; int bits_in = 0;
    while (oi < orig_len && (pos < dlen || bits_in >= max_bits)) {
        while (bits_in < 56 && pos < dlen) { bitbuf |= (uint64_t)d[pos++] << bits_in; bits_in += 8; }
        if (bits_in < 1) break;
        int look_bits = (bits_in < max_bits) ? bits_in : max_bits;
        uint32_t idx = (uint32_t)(bitbuf & ((1ULL << look_bits) - 1));
        if (idx < (uint32_t)table_size && lookup[idx] != 0xFFFF) {
            out[oi++] = (uint8_t)lookup[idx];
            int cl = sym_lens[lookup[idx]];
            bitbuf >>= cl; bits_in -= cl;
        } else break;
    }

    free(lookup);
    *out_len = oi;
    return out;
}

/* ── Range Coder Decompress ───────────────────────────────────────── */
#define RC_SCALE 4096
#define RC_BOT   (1u << 16)

static uint8_t *range_decompress(const uint8_t *d, size_t dlen, size_t *out_len) {
    if (dlen < 2) { *out_len = 0; return NULL; }
    size_t p = 0;
    uint16_t active = rd16(d); p += 2;
    if (active == 0) { *out_len = 0; return (uint8_t *)calloc(1, 1); }

    uint16_t norm[256]; memset(norm, 0, sizeof(norm));
    for (uint16_t i = 0; i < active && p < dlen; i++) {
        uint8_t sym = d[p++];
        if (p + 1 >= dlen) break;
        norm[sym] = rd16(d + p); p += 2;
    }
    if (p + 3 >= dlen) { *out_len = 0; return NULL; }
    uint32_t orig = rd32(d + p); p += 4;

    /* Single-symbol shortcut */
    if (active == 1) {
        uint8_t sym = 0;
        for (int i = 0; i < 256; i++) if (norm[i]) { sym = (uint8_t)i; break; }
        uint8_t *out = (uint8_t *)malloc(orig);
        memset(out, sym, orig);
        *out_len = orig;
        return out;
    }

    /* Build CDF + sym_lookup */
    uint32_t cdf[257]; cdf[0] = 0;
    for (int i = 0; i < 256; i++) cdf[i+1] = cdf[i] + norm[i];
    uint8_t sym_lookup[RC_SCALE];
    for (int i = 0; i < 256; i++)
        if (norm[i]) for (uint32_t j = cdf[i]; j < cdf[i+1] && j < RC_SCALE; j++)
            sym_lookup[j] = (uint8_t)i;

    const uint8_t *coded = d + p; size_t coded_len = dlen - p;
    uint32_t codeVal = 0; size_t cp = 0;
    for (int i = 0; i < 4; i++) { codeVal = (codeVal << 8) | (cp < coded_len ? coded[cp] : 0); cp++; }

    uint32_t low = 0, rng = 0xFFFFFFFF;
    uint8_t *out = (uint8_t *)malloc(orig);
    for (uint32_t i = 0; i < orig; i++) {
        uint32_t r = rng / RC_SCALE;
        uint32_t offset = ((codeVal - low) / r);
        if (offset >= RC_SCALE) offset = RC_SCALE - 1;
        uint8_t sym = sym_lookup[offset];
        low = low + r * cdf[sym];
        if (cdf[sym+1] - cdf[sym] < RC_SCALE) rng = r * (cdf[sym+1] - cdf[sym]);
        else rng = rng - r * cdf[sym];
        while (rng < RC_BOT) {
            low <<= 8; rng <<= 8;
            codeVal = (codeVal << 8) | (cp < coded_len ? coded[cp] : 0); cp++;
        }
        out[i] = sym;
    }
    *out_len = orig;
    return out;
}

/* ── Block Decompressor ───────────────────────────────────────────── */
static uint8_t *decompress_block(const uint8_t *d, size_t dlen, size_t *out_len) {
    if (dlen == 0) { *out_len = 0; return (uint8_t *)calloc(1, 1); }
    uint8_t tag = d[0];
    const uint8_t *payload = d + 1;
    size_t plen = dlen - 1;
    size_t tmp_len;

    switch (tag) {
        case 0x02: { /* Raw */
            uint8_t *out = (uint8_t *)malloc(plen);
            memcpy(out, payload, plen);
            *out_len = plen;
            return out;
        }
        case 0x00: return lz77_decompress(payload, plen, out_len);
        case 0x01: {
            uint8_t *huff = huffman_decompress(payload, plen, &tmp_len);
            if (!huff) { *out_len = 0; return NULL; }
            uint8_t *out = lz77_decompress(huff, tmp_len, out_len);
            free(huff);
            return out;
        }
        case 0x03: return huffman_decompress(payload, plen, out_len);
        case 0x04: return range_decompress(payload, plen, out_len);
        case 0x05: {
            uint8_t *rc = range_decompress(payload, plen, &tmp_len);
            if (!rc) { *out_len = 0; return NULL; }
            uint8_t *out = lz77_decompress(rc, tmp_len, out_len);
            free(rc);
            return out;
        }
        default: return lz77_decompress(payload, plen, out_len);
    }
}

/* ── Codec Decoders ───────────────────────────────────────────────── */
static int64_t *decode_delta_int(const uint8_t *d, size_t dlen, uint64_t nrows) {
    int64_t *out = (int64_t *)malloc(nrows * sizeof(int64_t));
    if (!out || nrows == 0) return out;
    size_t pos = 0; int64_t base;
    pos = read_zvar(d, pos, dlen, &base);
    out[0] = base;
    for (uint64_t i = 1; i < nrows; i++) {
        int64_t delta;
        pos = read_zvar(d, pos, dlen, &delta);
        base += delta;
        out[i] = base;
    }
    return out;
}

static int64_t *decode_rle_int(const uint8_t *d, size_t dlen, uint64_t nrows) {
    int64_t *out = (int64_t *)malloc(nrows * sizeof(int64_t));
    if (!out) return NULL;
    size_t pos = 0; uint64_t idx = 0;
    while (idx < nrows && pos < dlen) {
        uint64_t count; int64_t val;
        pos = read_varint(d, pos, dlen, &count);
        pos = read_zvar(d, pos, dlen, &val);
        for (uint64_t c = 0; c < count && idx < nrows; c++)
            out[idx++] = val;
    }
    return out;
}

static int64_t *decode_cdelta(const uint8_t *d, size_t dlen, uint64_t nrows) {
    int64_t *out = (int64_t *)malloc(nrows * sizeof(int64_t));
    if (!out) return NULL;
    size_t pos = 0; int64_t base, step;
    pos = read_zvar(d, pos, dlen, &base);
    pos = read_zvar(d, pos, dlen, &step);
    for (uint64_t i = 0; i < nrows; i++) out[i] = base + step * (int64_t)i;
    return out;
}

static int64_t *decode_for(const uint8_t *d, size_t dlen, uint64_t nrows) {
    int64_t *out = (int64_t *)malloc(nrows * sizeof(int64_t));
    if (!out) return NULL;
    size_t pos = 0; int64_t minval;
    pos = read_zvar(d, pos, dlen, &minval);
    uint8_t bits = (pos < dlen) ? d[pos] : 0; pos++;
    if (bits == 0) { for (uint64_t i = 0; i < nrows; i++) out[i] = minval; return out; }
    uint64_t bitbuf = 0; int bitpos = 0;
    for (uint64_t i = 0; i < nrows; i++) {
        while (bitpos < bits && pos < dlen) { bitbuf |= (uint64_t)d[pos++] << bitpos; bitpos += 8; }
        uint64_t mask = ((uint64_t)1 << bits) - 1;
        out[i] = minval + (int64_t)(bitbuf & mask);
        bitbuf >>= bits; bitpos -= bits;
    }
    return out;
}

static uint8_t *decode_bitpack(const uint8_t *d, size_t dlen, uint64_t nrows) {
    uint8_t *out = (uint8_t *)calloc(nrows, 1);
    if (!out) return NULL;
    for (uint64_t i = 0; i < nrows; i++) {
        size_t byte_idx = i / 8, bit_idx = i % 8;
        if (byte_idx < dlen) out[i] = (d[byte_idx] >> bit_idx) & 1;
    }
    return out;
}

typedef struct { char **strs; uint64_t len; } StrVec;

static StrVec decode_rle_str(const uint8_t *d, size_t dlen, uint64_t nrows) {
    StrVec sv; sv.strs = (char **)calloc(nrows, sizeof(char *)); sv.len = nrows;
    size_t pos = 0; uint64_t idx = 0;
    while (idx < nrows && pos < dlen) {
        uint64_t count, slen;
        pos = read_varint(d, pos, dlen, &count);
        pos = read_varint(d, pos, dlen, &slen);
        char *s = (char *)malloc(slen + 1);
        memcpy(s, d + pos, slen); s[slen] = '\0'; pos += slen;
        for (uint64_t c = 0; c < count && idx < nrows; c++)
            sv.strs[idx++] = (c == 0) ? s : strdup(s);
    }
    return sv;
}

static StrVec decode_bdict(const uint8_t *d, size_t dlen, uint64_t nrows) {
    StrVec sv; sv.strs = (char **)calloc(nrows, sizeof(char *)); sv.len = nrows;
    size_t pos = 0; uint64_t nuniq;
    pos = read_varint(d, pos, dlen, &nuniq);
    char **dict = (char **)calloc(nuniq, sizeof(char *));
    for (uint64_t i = 0; i < nuniq; i++) {
        uint64_t slen;
        pos = read_varint(d, pos, dlen, &slen);
        dict[i] = (char *)malloc(slen + 1);
        memcpy(dict[i], d + pos, slen); dict[i][slen] = '\0'; pos += slen;
    }
    uint8_t bits = (pos < dlen) ? d[pos] : 0; pos++;
    if (bits == 0) {
        for (uint64_t i = 0; i < nrows; i++)
            sv.strs[i] = strdup(nuniq > 0 ? dict[0] : "");
    } else {
        uint64_t bitbuf = 0; int bitpos = 0;
        for (uint64_t i = 0; i < nrows; i++) {
            while (bitpos < (int)bits && pos < dlen) { bitbuf |= (uint64_t)d[pos++] << bitpos; bitpos += 8; }
            uint64_t mask = ((uint64_t)1 << bits) - 1;
            uint64_t idx = bitbuf & mask;
            bitbuf >>= bits; bitpos -= bits;
            sv.strs[i] = strdup(idx < nuniq ? dict[idx] : "");
        }
    }
    for (uint64_t i = 0; i < nuniq; i++) free(dict[i]);
    free(dict);
    return sv;
}

static StrVec decode_raw_str(const uint8_t *d, size_t dlen, uint64_t nrows) {
    StrVec sv; sv.strs = (char **)calloc(nrows, sizeof(char *)); sv.len = nrows;
    size_t pos = 0;
    for (uint64_t i = 0; i < nrows; i++) {
        uint64_t slen = 0;
        if (pos < dlen) pos = read_varint(d, pos, dlen, &slen);
        sv.strs[i] = (char *)malloc(slen + 1);
        if (pos + slen <= dlen) { memcpy(sv.strs[i], d + pos, slen); pos += slen; }
        sv.strs[i][slen] = '\0';
    }
    return sv;
}

/* ── Core API ─────────────────────────────────────────────────────── */
const char *kore_type_name(uint8_t ktype) {
    switch (ktype) {
        case 1: return "Int"; case 2: return "Float"; case 3: return "Bool";
        case 4: return "Str"; case 5: return "Bytes"; default: return "?";
    }
}

int kore_col_index(KoreFile *kf, const char *name) {
    for (int i = 0; i < kf->ncols; i++)
        if (strcmp(kf->schema[i].name, name) == 0) return i;
    return -1;
}

static int kore_parse(KoreFile *kf) {
    uint8_t *d = kf->data;
    size_t dlen = kf->data_len;
    if (dlen < KORE_HEADER_LEN + 12) return -1;
    if (memcmp(d, "KORE", 4) != 0) return -2;

    kf->version    = d[4];
    kf->ncols      = rd16(d + 6);
    kf->nrows      = rd64(d + 8);
    kf->nchunks    = rd32(d + 16);
    kf->chunk_size = rd32(d + 20);
    kf->created    = rd64(d + 24);

    /* Schema */
    size_t pos = KORE_HEADER_LEN;
    uint32_t schema_comp_len = rd32(d + pos); pos += 4;
    size_t schema_raw_len;
    uint8_t *schema_raw = decompress_block(d + pos, schema_comp_len, &schema_raw_len);
    pos += schema_comp_len;

    kf->schema = (KoreSchema *)calloc(kf->ncols, sizeof(KoreSchema));
    size_t sp = 0;
    for (int i = 0; i < kf->ncols; i++) {
        uint64_t nlen;
        sp = read_varint(schema_raw, sp, schema_raw_len, &nlen);
        if (nlen > 255) nlen = 255;
        memcpy(kf->schema[i].name, schema_raw + sp, nlen);
        kf->schema[i].name[nlen] = '\0';
        sp += nlen;
        kf->schema[i].ktype = (sp < schema_raw_len) ? schema_raw[sp] : 4; sp++;
        kf->schema[i].encrypted = (sp < schema_raw_len) ? schema_raw[sp] : 0; sp++;
    }
    free(schema_raw);

    /* Dictionary */
    uint32_t dict_comp_len = rd32(d + pos); pos += 4;
    size_t dict_raw_len;
    uint8_t *dict_raw = decompress_block(d + pos, dict_comp_len, &dict_raw_len);
    pos += dict_comp_len;

    size_t dp = 0; uint64_t dc;
    dp = read_varint(dict_raw, dp, dict_raw_len, &dc);
    kf->dict_count = (uint32_t)dc;
    kf->dictionary = (char **)calloc(dc + 1, sizeof(char *));
    for (uint64_t i = 0; i < dc; i++) {
        uint64_t slen;
        dp = read_varint(dict_raw, dp, dict_raw_len, &slen);
        kf->dictionary[i] = (char *)malloc(slen + 1);
        memcpy(kf->dictionary[i], dict_raw + dp, slen);
        kf->dictionary[i][slen] = '\0'; dp += slen;
    }
    free(dict_raw);

    /* Footer */
    size_t trailer = dlen - 12;
    uint32_t footer_comp_len = rd32(d + trailer);
    uint64_t footer_offset   = rd64(d + trailer + 4);
    size_t footer_raw_len;
    uint8_t *fr = decompress_block(d + footer_offset, footer_comp_len, &footer_raw_len);

    size_t fp = 0;
    uint32_t ft_nchunks = rd32(fr + fp); fp += 4;
    uint16_t ft_ncols = rd16(fr + fp); fp += 2;
    (void)ft_nchunks; (void)ft_ncols;

    kf->chunk_nrows = (uint32_t *)calloc(kf->nchunks, sizeof(uint32_t));
    for (uint32_t c = 0; c < kf->nchunks; c++) {
        kf->chunk_nrows[c] = rd32(fr + fp); fp += 4;
    }

    kf->col_meta = (KoreColMeta **)calloc(kf->nchunks, sizeof(KoreColMeta *));
    for (uint32_t c = 0; c < kf->nchunks; c++) {
        kf->col_meta[c] = (KoreColMeta *)calloc(kf->ncols, sizeof(KoreColMeta));
        for (int ci = 0; ci < kf->ncols; ci++) {
            KoreColMeta *cm = &kf->col_meta[c][ci];
            cm->file_offset = rd64(fr + fp); fp += 8;
            cm->comp_len    = rd32(fr + fp); fp += 4;
            cm->codec       = (fp < footer_raw_len) ? fr[fp] : 0; fp++;
            cm->null_count  = rd32(fr + fp); fp += 4;
            int64_t mn, mx;
            fp = read_zvar(fr, fp, footer_raw_len, &mn);
            fp = read_zvar(fr, fp, footer_raw_len, &mx);
            cm->min_i64 = mn; cm->max_i64 = mx;
            /* Skip min/max strings */
            uint64_t slen;
            fp = read_varint(fr, fp, footer_raw_len, &slen); fp += slen;
            fp = read_varint(fr, fp, footer_raw_len, &slen); fp += slen;
            fp += 512; /* bloom filter */
        }
    }
    free(fr);
    return 0;
}

static void kore_load_del(KoreFile *kf, const char *path) {
    kf->has_del = 0; kf->del_bitmap = NULL; kf->del_count = 0;
    if (!path) return;
    char dpath[1024];
    snprintf(dpath, sizeof(dpath), "%s.del", path);
    FILE *f = fopen(dpath, "rb");
    if (!f) return;
    fseek(f, 0, SEEK_END); long sz = ftell(f);
    if (sz < 16) { fclose(f); return; }
    fseek(f, 0, SEEK_SET);
    uint8_t *buf = (uint8_t *)malloc(sz);
    if (fread(buf, 1, sz, f) != (size_t)sz) { free(buf); fclose(f); return; }
    fclose(f);
    uint64_t total_rows = rd64(buf);
    kf->del_count = rd64(buf + 8);
    uint64_t nwords = (total_rows + 63) / 64;
    kf->del_bitmap = (uint64_t *)calloc(nwords, sizeof(uint64_t));
    for (uint64_t i = 0; i < nwords && 16 + i * 8 + 8 <= (uint64_t)sz; i++)
        kf->del_bitmap[i] = rd64(buf + 16 + i * 8);
    kf->has_del = 1;
    free(buf);
}

static int kore_is_deleted(KoreFile *kf, uint64_t row) {
    if (!kf->has_del || !kf->del_bitmap) return 0;
    uint64_t word = row / 64;
    return (kf->del_bitmap[word] >> (row % 64)) & 1;
}

int kore_open(KoreFile *kf, const char *path) {
    memset(kf, 0, sizeof(KoreFile));
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END); long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    kf->data = (uint8_t *)malloc(sz);
    kf->data_len = sz;
    if (fread(kf->data, 1, sz, f) != (size_t)sz) { fclose(f); free(kf->data); return -1; }
    fclose(f);
    int rc = kore_parse(kf);
    if (rc != 0) return rc;
    kore_load_del(kf, path);
    return 0;
}

int kore_open_mem(KoreFile *kf, const uint8_t *data, size_t len) {
    memset(kf, 0, sizeof(KoreFile));
    kf->data = (uint8_t *)malloc(len);
    memcpy(kf->data, data, len);
    kf->data_len = len;
    return kore_parse(kf);
}

void kore_close(KoreFile *kf) {
    if (kf->data) free(kf->data);
    if (kf->schema) free(kf->schema);
    if (kf->dictionary) {
        for (uint32_t i = 0; i < kf->dict_count; i++) free(kf->dictionary[i]);
        free(kf->dictionary);
    }
    if (kf->chunk_nrows) free(kf->chunk_nrows);
    if (kf->col_meta) {
        for (uint32_t c = 0; c < kf->nchunks; c++) free(kf->col_meta[c]);
        free(kf->col_meta);
    }
    if (kf->del_bitmap) free(kf->del_bitmap);
    memset(kf, 0, sizeof(KoreFile));
}

int kore_read_column_idx(KoreFile *kf, int col_idx, KoreColumn *out) {
    if (col_idx < 0 || col_idx >= kf->ncols) return -1;
    memset(out, 0, sizeof(KoreColumn));
    out->ktype = kf->schema[col_idx].ktype;

    /* Accumulate from chunks */
    uint64_t total = 0;
    for (uint32_t c = 0; c < kf->nchunks; c++) total += kf->chunk_nrows[c];

    if (out->ktype == KTYPE_INT || out->ktype == KTYPE_FLOAT) {
        int64_t *all = (int64_t *)malloc(total * sizeof(int64_t));
        uint64_t off = 0;
        for (uint32_t c = 0; c < kf->nchunks; c++) {
            KoreColMeta *cm = &kf->col_meta[c][col_idx];
            uint64_t nr = kf->chunk_nrows[c];
            size_t comp_off = cm->file_offset;
            /* Skip CRC + comp_len header */
            uint32_t stored_crc = rd32(kf->data + comp_off);
            uint32_t comp_len = rd32(kf->data + comp_off + 4);
            uint8_t *compressed = kf->data + comp_off + 8;
            uint32_t actual_crc = kore_crc32(compressed, comp_len);
            if (actual_crc != stored_crc) { free(all); return -3; }
            size_t raw_len;
            uint8_t *raw = decompress_block(compressed, comp_len, &raw_len);

            /* Float scale prefix */
            double scale = 1.0;
            uint8_t *codec_data = raw; size_t codec_len = raw_len;
            if (out->ktype == KTYPE_FLOAT && raw_len >= 2 && raw[0] == 0xFE) {
                uint8_t se = raw[1]; scale = 1.0;
                double muls[] = {1,10,100,1000,10000};
                scale = (se <= 4) ? muls[se] : 10000;
                codec_data = raw + 2; codec_len = raw_len - 2;
            } else if (out->ktype == KTYPE_FLOAT) {
                scale = 10000.0;
            }

            int64_t *chunk_vals = NULL;
            switch (cm->codec) {
                case KCODEC_CDELTA: chunk_vals = decode_cdelta(codec_data, codec_len, nr); break;
                case KCODEC_DELTA:  chunk_vals = decode_delta_int(codec_data, codec_len, nr); break;
                case KCODEC_RLE:    chunk_vals = decode_rle_int(codec_data, codec_len, nr); break;
                case KCODEC_FOR:    chunk_vals = decode_for(codec_data, codec_len, nr); break;
                default:            chunk_vals = decode_delta_int(codec_data, codec_len, nr); break;
            }
            memcpy(all + off, chunk_vals, nr * sizeof(int64_t));
            off += nr;
            free(chunk_vals); free(raw);
        }

        /* Filter deleted + convert */
        if (out->ktype == KTYPE_FLOAT) {
            /* Determine scale from first chunk */
            double scale = 10000.0;
            if (kf->nchunks > 0) {
                KoreColMeta *cm0 = &kf->col_meta[0][col_idx];
                size_t co = cm0->file_offset;
                uint32_t cl = rd32(kf->data + co + 4);
                size_t rl;
                uint8_t *r = decompress_block(kf->data + co + 8, cl, &rl);
                if (rl >= 2 && r[0] == 0xFE) {
                    uint8_t se = r[1];
                    double muls[] = {1,10,100,1000,10000};
                    scale = (se <= 4) ? muls[se] : 10000;
                }
                free(r);
            }
            double *fd = (double *)malloc(total * sizeof(double));
            uint64_t fi = 0;
            for (uint64_t i = 0; i < total; i++) {
                if (!kore_is_deleted(kf, i))
                    fd[fi++] = (double)all[i] / scale;
            }
            free(all);
            out->floats = fd;
            out->len = fi;
        } else {
            int64_t *fd = (int64_t *)malloc(total * sizeof(int64_t));
            uint64_t fi = 0;
            for (uint64_t i = 0; i < total; i++) {
                if (!kore_is_deleted(kf, i)) fd[fi++] = all[i];
            }
            free(all);
            out->ints = fd;
            out->len = fi;
        }
    } else if (out->ktype == KTYPE_BOOL) {
        uint8_t *all = (uint8_t *)calloc(total, 1);
        uint64_t off = 0;
        for (uint32_t c = 0; c < kf->nchunks; c++) {
            KoreColMeta *cm = &kf->col_meta[c][col_idx];
            uint64_t nr = kf->chunk_nrows[c];
            size_t comp_off = cm->file_offset;
            uint32_t comp_len = rd32(kf->data + comp_off + 4);
            size_t raw_len;
            uint8_t *raw = decompress_block(kf->data + comp_off + 8, comp_len, &raw_len);
            uint8_t *bools = (cm->codec == KCODEC_BITPACK) ?
                decode_bitpack(raw, raw_len, nr) : decode_bitpack(raw, raw_len, nr);
            memcpy(all + off, bools, nr); off += nr;
            free(bools); free(raw);
        }
        uint8_t *fd = (uint8_t *)calloc(total, 1);
        uint64_t fi = 0;
        for (uint64_t i = 0; i < total; i++)
            if (!kore_is_deleted(kf, i)) fd[fi++] = all[i];
        free(all);
        out->bools = fd; out->len = fi;
    } else { /* STR */
        char **all = (char **)calloc(total, sizeof(char *));
        uint64_t off = 0;
        for (uint32_t c = 0; c < kf->nchunks; c++) {
            KoreColMeta *cm = &kf->col_meta[c][col_idx];
            uint64_t nr = kf->chunk_nrows[c];
            size_t comp_off = cm->file_offset;
            uint32_t comp_len = rd32(kf->data + comp_off + 4);
            size_t raw_len;
            uint8_t *raw = decompress_block(kf->data + comp_off + 8, comp_len, &raw_len);
            StrVec sv;
            switch (cm->codec) {
                case KCODEC_RLE:    sv = decode_rle_str(raw, raw_len, nr); break;
                case KCODEC_BDICT:  sv = decode_bdict(raw, raw_len, nr); break;
                case KCODEC_HUFFDICT: sv = decode_bdict(raw, raw_len, nr); break;
                default:            sv = decode_raw_str(raw, raw_len, nr); break;
            }
            for (uint64_t i = 0; i < nr; i++) all[off + i] = sv.strs[i];
            free(sv.strs); free(raw);
            off += nr;
        }
        char **fd = (char **)calloc(total, sizeof(char *));
        uint64_t fi = 0;
        for (uint64_t i = 0; i < total; i++) {
            if (!kore_is_deleted(kf, i)) fd[fi++] = all[i];
            else free(all[i]);
        }
        free(all);
        out->strings = fd; out->len = fi;
    }
    return 0;
}

int kore_read_column(KoreFile *kf, const char *col_name, KoreColumn *out) {
    int ci = kore_col_index(kf, col_name);
    if (ci < 0) return -1;
    return kore_read_column_idx(kf, ci, out);
}

void kore_free_column(KoreColumn *col) {
    if (col->ktype == KTYPE_STR || col->ktype == KTYPE_BYTES) {
        for (uint64_t i = 0; i < col->len; i++) free(col->strings[i]);
        free(col->strings);
    } else if (col->ktype == KTYPE_FLOAT) {
        free(col->floats);
    } else if (col->ktype == KTYPE_BOOL) {
        free(col->bools);
    } else {
        free(col->ints);
    }
    memset(col, 0, sizeof(KoreColumn));
}

#endif /* KORE_IMPLEMENTATION */
#endif /* KORE_READER_H */
