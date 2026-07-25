// kore_profile.rs — Per-column size profiler for KORE v2 files
// Uses the footer metadata to accurately report per-column compressed sizes

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: kore_profile <kore_file>");
        std::process::exit(1);
    }
    let path = &args[1];
    let data = std::fs::read(path).expect("Cannot read file");
    let total_size = data.len();

    // Parse header
    let ncols = u16::from_le_bytes(data[6..8].try_into().unwrap()) as usize;
    let nrows = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
    let nchunks = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;

    // Parse schema for column names
    let mut pos = 64usize;
    let schema_comp_len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize;
    pos += 4;
    // Decompress schema to get names — just read names from the decompressed block
    let _schema_data = &data[pos..pos+schema_comp_len];
    pos += schema_comp_len;
    let dict_comp_len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize;
    pos += 4 + dict_comp_len;
    let header_overhead = pos;

    // Parse footer from end of file
    let ft_start = data.len() - 12;
    let footer_comp_len = u32::from_le_bytes(data[ft_start..ft_start+4].try_into().unwrap()) as usize;
    let footer_offset = u64::from_le_bytes(data[ft_start+4..ft_start+12].try_into().unwrap()) as usize;

    // Decompress footer
    let footer_raw = decompress_footer(&data[footer_offset..footer_offset+footer_comp_len]);

    // Parse footer: nchunks(u32) + ncols(u16) + chunk_nrows[nchunks] + per-chunk per-col metadata
    let mut fp = 0usize;
    let _ft_nchunks = u32::from_le_bytes(footer_raw[fp..fp+4].try_into().unwrap()) as usize;
    fp += 4;
    let _ft_ncols = u16::from_le_bytes(footer_raw[fp..fp+2].try_into().unwrap()) as usize;
    fp += 2;
    // Skip chunk_nrows
    fp += nchunks * 4;

    // Accumulate per-column compressed sizes
    let mut col_sizes: Vec<u64> = vec![0; ncols];
    let mut col_codecs: Vec<[u32; 16]> = vec![[0u32; 16]; ncols]; // codec histogram

    for _chunk in 0..nchunks {
        for ci in 0..ncols {
            let _file_offset = u64::from_le_bytes(footer_raw[fp..fp+8].try_into().unwrap());
            fp += 8;
            let comp_len = u32::from_le_bytes(footer_raw[fp..fp+4].try_into().unwrap()) as u64;
            fp += 4;
            let codec = footer_raw[fp] as usize;
            fp += 1;

            col_sizes[ci] += comp_len + 8; // +8 for crc32(4) + comp_len_field(4)
            if codec < 16 { col_codecs[ci][codec] += 1; }

            // Skip stats: null_count(4) + min_zvar + max_zvar + min_str(varint+data) + max_str(varint+data)
            fp += 4; // null_count
            fp = skip_zvar(&footer_raw, fp);
            fp = skip_zvar(&footer_raw, fp);
            let (min_slen, np) = read_varint_simple(&footer_raw, fp); fp = np + min_slen as usize;
            let (max_slen, np) = read_varint_simple(&footer_raw, fp); fp = np + max_slen as usize;
            fp += 512; // bloom filter
        }
    }

    // Get column names from KoreReader
    let reader = killer_native::kore_v2::KoreReader::open(path).expect("Cannot parse KORE");

    println!("KORE v2 Profile: {}", path);
    println!("Total: {} bytes ({:.1} MB) | {} rows × {} cols | {} chunks",
             total_size, total_size as f64/(1024.0*1024.0), nrows, ncols, nchunks);
    println!();

    println!("OVERHEAD:");
    println!("  Header+Schema+Dict: {:>8} bytes ({:.1} KB)", header_overhead, header_overhead as f64/1024.0);
    println!("  Footer (compressed): {:>7} bytes ({:.1} KB)", footer_comp_len, footer_comp_len as f64/1024.0);
    println!("  Bloom in footer:    {:>8} bytes ({:.1} MB) [{} chunks × {} cols × 512B]",
             nchunks*ncols*512, (nchunks*ncols*512) as f64/(1024.0*1024.0), nchunks, ncols);
    println!();

    let codec_names = ["Raw","RLE","Delta","DictRLE","Bitpk","BDict","CDelta","FOR","HfDict"];

    println!("PER-COLUMN (sorted by size):");
    println!("{:>3} {:20} {:>10}  {:>6}  {:>6}  {:>10}  codec",
             "#", "COLUMN", "BYTES", "MB", "%", "bytes/row");
    let mut col_info: Vec<(usize, u64)> = (0..ncols).map(|i| (i, col_sizes[i])).collect();
    col_info.sort_by(|a, b| b.1.cmp(&a.1));

    let total_col: u64 = col_sizes.iter().sum();
    for (ci, size) in &col_info {
        let name = &reader.columns[*ci].name;
        let pct = *size as f64 / total_size as f64 * 100.0;
        let mb = *size as f64 / (1024.0*1024.0);
        let per_row = *size as f64 / nrows as f64;
        let dominant_codec = col_codecs[*ci].iter().enumerate()
            .max_by_key(|(_, &cnt)| cnt).map(|(idx, _)| idx).unwrap_or(0);
        let cname = if dominant_codec < codec_names.len() { codec_names[dominant_codec] } else { "?" };
        println!("{:>3} {:20} {:>10}  {:>5.1}  {:>5.1}%  {:>9.2}  {}",
                 ci, name, size, mb, pct, per_row, cname);
    }
    println!("{:>3} {:20} {:>10}  {:>5.1}  {:>5.1}%",
             "", "TOTAL DATA", total_col, total_col as f64/(1024.0*1024.0),
             total_col as f64/total_size as f64*100.0);
}

fn skip_zvar(data: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < data.len() {
        if data[i] & 0x80 == 0 { return i + 1; }
        i += 1;
    }
    i
}

fn read_varint_simple(data: &[u8], pos: usize) -> (u64, usize) {
    let mut r = 0u64; let mut s = 0u32; let mut i = pos;
    while i < data.len() {
        let b = data[i] as u64; r |= (b & 0x7F) << s; i += 1;
        if b & 0x80 == 0 { break; } s += 7;
    }
    (r, i)
}

fn decompress_footer(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return Vec::new(); }
    match data[0] {
        0x01 => lz77_decompress(&huffman_decompress(&data[1..])),
        0x02 => data[1..].to_vec(),
        _    => lz77_decompress(&data[1..]),
    }
}

fn lz77_decompress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    let mut i = 0;
    while i < input.len() {
        if input[i] == 0xFF && i + 4 < input.len() {
            let off = u16::from_le_bytes([input[i+1], input[i+2]]) as usize;
            let len = u16::from_le_bytes([input[i+3], input[i+4]]) as usize;
            i += 5;
            if off == 0 && len == 1 { out.push(0xFF); }
            else {
                let base = out.len().saturating_sub(off);
                if off == 0 || base >= out.len() { continue; }
                if base + len <= out.len() {
                    let start = out.len();
                    out.resize(start + len, 0);
                    out.copy_within(base..base+len, start);
                } else {
                    for j in 0..len { let b = out[base + j]; out.push(b); }
                }
            }
        } else { out.push(input[i]); i += 1; }
    }
    out
}

fn huffman_decompress(input: &[u8]) -> Vec<u8> {
    if input.len() < 260 { return Vec::new(); }
    let mut code_lens = [0u8; 256];
    code_lens.copy_from_slice(&input[..256]);
    let orig_len = u32::from_le_bytes([input[256], input[257], input[258], input[259]]) as usize;
    let bitstream = &input[260..];
    let mut syms_by_len: Vec<(u8, u8)> = code_lens.iter().enumerate()
        .filter(|(_, &l)| l > 0).map(|(s, &l)| (l, s as u8)).collect();
    syms_by_len.sort();
    if syms_by_len.is_empty() { return Vec::new(); }
    let mut lookup = [(0u8, 0u8); 1 << 15];
    let mut code = 0u32; let mut prev_len = 0u8;
    for &(len, sym) in &syms_by_len {
        code <<= len - prev_len;
        let shift = 15 - len;
        let base = (code as usize) << shift;
        let count = 1usize << shift;
        for i in 0..count { if base + i < lookup.len() { lookup[base + i] = (sym, len); } }
        code += 1; prev_len = len;
    }
    let mut out = Vec::with_capacity(orig_len);
    let mut bitbuf: u64 = 0; let mut bits_avail = 0u32; let mut byte_pos = 0;
    while out.len() < orig_len {
        while bits_avail <= 48 && byte_pos < bitstream.len() {
            bitbuf |= (bitstream[byte_pos] as u64) << (56 - bits_avail);
            bits_avail += 8; byte_pos += 1;
        }
        if bits_avail == 0 { break; }
        let peek = (bitbuf >> 49) as usize & 0x7FFF;
        let (sym, len) = lookup[peek];
        if len == 0 { break; }
        out.push(sym); bitbuf <<= len; bits_avail -= len as u32;
    }
    out.truncate(orig_len);
    out
}
