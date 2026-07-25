// nova_video.rs — Native animated GIF generator for Kala
// Pure Rust, zero crates, zero network.
//
// Generates 10-frame animated GIFs using nova_gen scene renderers.
// Format: GIF89a · 64×64 · 64-color palette · ~6fps · ~0.5MB base64
// Output: "data:image/gif;base64,..." renders as <img> in any browser.

const VW: usize = 64;
const VH: usize = 64;
pub const N_FRAMES: usize = 10;
const FRAME_DELAY_CS: u16 = 7;   // 70ms per frame → ~14fps

// ── Bit writer for GIF LZW ────────────────────────────────────
struct BitWriter { bytes: Vec<u8>, buf: u32, cnt: u32 }
impl BitWriter {
    fn new() -> Self { BitWriter { bytes: Vec::new(), buf: 0, cnt: 0 } }
    fn write(&mut self, code: u16, bits: u32) {
        self.buf |= (code as u32) << self.cnt;
        self.cnt += bits;
        while self.cnt >= 8 {
            self.bytes.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.cnt -= 8;
        }
    }
    fn flush(mut self) -> Vec<u8> {
        if self.cnt > 0 { self.bytes.push(self.buf as u8); }
        self.bytes
    }
}

// ── GIF LZW encoder ───────────────────────────────────────────
// min_code_size = 6 for 64-color palette (2^6 = 64 symbols)
fn lzw_encode(indices: &[u8], min_code_size: u8) -> Vec<u8> {
    use std::collections::HashMap;
    let mcs        = min_code_size as u32;
    let clear_code = 1u16 << mcs;
    let eoi_code   = clear_code + 1;

    let mut bw        = BitWriter::new();
    let mut code_size = mcs + 1;
    let mut table: HashMap<(u16, u8), u16> = HashMap::with_capacity(4096);
    let mut next_code: u16 = eoi_code + 1;

    bw.write(clear_code, code_size);

    if indices.is_empty() {
        bw.write(eoi_code, code_size);
        return pack_sub_blocks(&bw.flush());
    }

    let mut buf: u16 = indices[0] as u16;

    for &pixel in &indices[1..] {
        if let Some(&code) = table.get(&(buf, pixel)) {
            buf = code;
        } else {
            bw.write(buf, code_size);
            if next_code <= 4095 {
                table.insert((buf, pixel), next_code);
                next_code += 1;
                if next_code as u32 > (1 << code_size) && code_size < 12 {
                    code_size += 1;
                }
            } else {
                // Table full: reset
                bw.write(clear_code, code_size);
                table.clear();
                code_size = mcs + 1;
                next_code = eoi_code + 1;
            }
            buf = pixel as u16;
        }
    }

    bw.write(buf, code_size);
    bw.write(eoi_code, code_size);
    pack_sub_blocks(&bw.flush())
}

fn pack_sub_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(255) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out.push(0); // block terminator
    out
}

// ── Scene-tuned 64-colour palettes ────────────────────────────
fn make_palette(prompt: &str) -> Vec<[u8; 3]> {
    let p  = prompt.to_lowercase();
    let mut pal: Vec<[u8; 3]> = Vec::with_capacity(64);

    if p.contains("matrix") || p.contains("code") || p.contains("hacker") || p.contains("digital") {
        for i in 0..64usize { pal.push([0, (i * 4).min(255) as u8, (i / 4) as u8]); }
    } else if p.contains("fire") || p.contains("flame") || p.contains("lava") {
        for i in 0..32 { pal.push([(i * 8).min(255) as u8, (i * 2) as u8, 0]); }
        for i in 0..32 { pal.push([220u8, (100 + i * 5).min(255) as u8, (i * 2) as u8]); }
    } else if p.contains("space") || p.contains("galaxy") || p.contains("cosmos") {
        for i in 0..32 { let v = (i * 6) as u8; pal.push([v / 4, v / 6, v.min(200)]); }
        for i in 0..16 { let v = (i * 14) as u8; pal.push([v / 3, 0, v.min(255)]); }
        for i in 0..16 { let v = (200 + i * 3).min(255) as u8; pal.push([v, v, v]); }
    } else if p.contains("ocean") || p.contains("sea") || p.contains("wave") || p.contains("water") {
        for i in 0..32 { pal.push([0, (i * 4) as u8, (100 + i * 5).min(255) as u8]); }
        for i in 0..32 { pal.push([(i * 2) as u8, (180 - i * 2).max(80) as u8, (200 - i) as u8]); }
    } else if p.contains("forest") || p.contains("nature") || p.contains("tree") || p.contains("jungle") {
        for i in 0..32 { pal.push([(i * 2) as u8, (60 + i * 6).min(255) as u8, (i * 2) as u8]); }
        for i in 0..16 { let s = (80 + i * 8) as u8; pal.push([s, s/2, s/4]); }
        for i in 0..16 { pal.push([100u8, (200 - i * 5) as u8, (150 - i * 4) as u8]); }
    } else {
        // Generic rainbow (nature scenes, mountains, city)
        for i in 0..64usize {
            let h = i as f32 / 64.0;
            let (r, g, b) = hsv(h, 0.72, 0.88);
            pal.push([r, g, b]);
        }
    }

    while pal.len() < 64 { pal.push([0, 0, 0]); }
    pal.truncate(64);
    pal
}

fn hsv(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let hi = (h * 6.0) as u32 % 6;
    let f  = h * 6.0 - (h * 6.0).floor();
    let p  = v * (1.0 - s);
    let q  = v * (1.0 - f * s);
    let t  = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match hi {
        0 => (v, t, p), 1 => (q, v, p), 2 => (p, v, t),
        3 => (p, q, v), 4 => (t, p, v), _ => (v, p, q),
    };
    ((r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8)
}

// Nearest palette colour — weighted perceptual distance
fn nearest(r: u8, g: u8, b: u8, pal: &[[u8; 3]]) -> u8 {
    pal.iter().enumerate()
        .map(|(i, &[pr, pg, pb])| {
            let dr = r as i32 - pr as i32;
            let dg = g as i32 - pg as i32;
            let db = b as i32 - pb as i32;
            (i, (dr*dr*2 + dg*dg*4 + db*db) as u32)
        })
        .min_by_key(|&(_, d)| d)
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}

// ── GIF89a multi-frame writer ─────────────────────────────────
fn write_u16(v: u16) -> [u8; 2] { [v as u8, (v >> 8) as u8] }

fn prompt_seed(prompt: &str) -> u64 {
    let mut h: u64 = 0x517CC1B727220A95;
    for b in prompt.bytes() { h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(b as u64); }
    h
}

/// Human-readable scene label for a prompt
pub fn scene_name(prompt: &str) -> &'static str {
    let p = prompt.to_lowercase();
    if p.contains("matrix") || p.contains("code") || p.contains("hacker") { "Matrix / code rain" }
    else if p.contains("fire") || p.contains("flame") || p.contains("lava") { "Fire / lava" }
    else if p.contains("space") || p.contains("galaxy") || p.contains("cosmos") { "Space / galaxy" }
    else if p.contains("ocean") || p.contains("sea") || p.contains("wave") { "Ocean / waves" }
    else if p.contains("forest") || p.contains("nature") || p.contains("tree") { "Forest / nature" }
    else { "Generative abstract" }
}

/// Generate raw GIF bytes for the given prompt
pub fn generate_native_video(prompt: &str) -> Vec<u8> {
    let pal        = make_palette(prompt);
    let min_cs:u8  = 6;   // ceil(log2(64)) = 6
    let base_seed  = prompt_seed(prompt);

    let mut gif = Vec::new();

    // ── Header ────────────────────────────────────────────────
    gif.extend_from_slice(b"GIF89a");

    // Logical Screen Descriptor
    gif.extend_from_slice(&write_u16(VW as u16));
    gif.extend_from_slice(&write_u16(VH as u16));
    // packed: GlobalCT=1, ColorRes-1=7 (bits 4-6), SortFlag=0, CTSize=5 (2^6=64)
    gif.push(0b11110101); // 0xF5
    gif.push(0);  // background color index
    gif.push(0);  // pixel aspect ratio

    // Global Color Table (64 × 3 = 192 bytes)
    for &[r, g, b] in &pal { gif.push(r); gif.push(g); gif.push(b); }
    for _ in pal.len()..64  { gif.push(0); gif.push(0); gif.push(0); }

    // Netscape 2.0 Application Extension (infinite loop)
    gif.push(0x21); gif.push(0xFF); gif.push(0x0B);
    gif.extend_from_slice(b"NETSCAPE2.0");
    gif.push(0x03); gif.push(0x01);
    gif.extend_from_slice(&write_u16(0)); // 0 = loop forever
    gif.push(0x00);

    // ── Frames ────────────────────────────────────────────────
    for frame in 0..N_FRAMES {
        // Vary seed each frame for animation
        let seed = base_seed.wrapping_add(frame as u64 * 0x9E3779B97F4A7C15);
        let rgb  = crate::nova_gen::generate_native_image_with_seed(prompt, seed);

        // Downscale source image → 64×64 via box average, then quantize
        let src_w = crate::nova_gen::img_width() as usize;
        let src_h = crate::nova_gen::img_height() as usize;
        let step_x = src_w / VW;
        let step_y = src_h / VH;
        let pixels_per_block = (step_x * step_y) as u32;
        let mut indices = Vec::with_capacity(VW * VH);
        for fy in 0..VH {
            for fx in 0..VW {
                let mut rs = 0u32; let mut gs = 0u32; let mut bs = 0u32;
                for dy in 0..step_y {
                    for dx in 0..step_x {
                        let px = (fx * step_x + dx).min(src_w - 1);
                        let py = (fy * step_y + dy).min(src_h - 1);
                        let p  = (py * src_w + px) * 3;
                        if p + 2 < rgb.len() {
                            rs += rgb[p] as u32;
                            gs += rgb[p+1] as u32;
                            bs += rgb[p+2] as u32;
                        }
                    }
                }
                let d = pixels_per_block.max(1);
                indices.push(nearest((rs/d) as u8, (gs/d) as u8, (bs/d) as u8, &pal));
            }
        }

        // Graphic Control Extension
        gif.push(0x21); gif.push(0xF9); gif.push(0x04);
        gif.push(0x00); // no disposal, no transparency
        gif.extend_from_slice(&write_u16(FRAME_DELAY_CS));
        gif.push(0);    // transparent color index (unused)
        gif.push(0x00); // block terminator

        // Image Descriptor
        gif.push(0x2C);
        gif.extend_from_slice(&write_u16(0));           // left
        gif.extend_from_slice(&write_u16(0));           // top
        gif.extend_from_slice(&write_u16(VW as u16));
        gif.extend_from_slice(&write_u16(VH as u16));
        gif.push(0x00); // no local color table, not interlaced

        // Image Data
        gif.push(min_cs);
        gif.extend_from_slice(&lzw_encode(&indices, min_cs));
    }

    // Trailer
    gif.push(0x3B);
    gif
}

/// Generate animated GIF as "data:image/gif;base64,..."
pub fn generate_video_native(prompt: &str) -> String {
    let bytes = generate_native_video(prompt);
    let b64   = base64_encode_bytes(&bytes);
    format!("data:image/gif;base64,{}", b64)
}

fn base64_encode_bytes(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}
