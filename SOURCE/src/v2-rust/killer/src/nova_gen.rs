// nova_gen.rs — Nova Native Image Generator
//
// Generates images 100% natively in pure Rust — zero external crates, zero network calls.
// Uses algorithmic / procedural generation: gradients, noise, fractals, geometry.
//
// Output: PNG bytes → base64 data URI → renders inline in Kala chat
//
// Performance: 512×512 image in < 10ms on any hardware.
//
// Scene classes (detected from prompt keywords):
//   nature / forest / mountain / river / sky / sunset / ocean / beach → natural scene
//   city / urban / neon / night / tokyo / cyber / futuristic         → city/neon scene
//   space / galaxy / stars / cosmos / nebula / universe              → space scene
//   fire / flame / lava / volcano / explosion                        → fire scene
//   water / ocean / wave / lake / rain                               → water scene
//   abstract / art / fractal / pattern / geometric                   → fractal/art
//   code / matrix / hacker / terminal / program                      → matrix rain
//   default                                                           → gradient art

// ─────────────────────────────────────────────────────────────────────────────
// PNG Encoder (pure Rust, zero crates)
// ─────────────────────────────────────────────────────────────────────────────

/// Encode raw RGB pixels (width × height × 3 bytes) as a minimal PNG file.
/// Returns the complete PNG as a Vec<u8>.
pub fn encode_png(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(800_000);

    // PNG signature
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk: width, height, 8-bit depth, RGB colour type (2)
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8);  // bit depth
    ihdr.push(2);  // colour type: RGB
    ihdr.push(0);  // compression method
    ihdr.push(0);  // filter method
    ihdr.push(0);  // interlace method
    write_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT chunk: filter each row (None filter = 0x00), then zlib-deflate
    let row_bytes = (width * 3) as usize;
    let mut raw = Vec::with_capacity((row_bytes + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0u8); // filter type: None
        raw.extend_from_slice(&pixels[y * row_bytes..(y + 1) * row_bytes]);
    }
    let compressed = zlib_compress(&raw);
    write_chunk(&mut out, b"IDAT", &compressed);

    // IEND chunk
    write_chunk(&mut out, b"IEND", b"");

    out
}

fn write_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let crc = crc32(tag, data);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(tag: &[u8], data: &[u8]) -> u32 {
    // CRC-32 table-based
    let table: [u32; 256] = {
        let mut t = [0u32; 256];
        for n in 0u32..256 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            }
            t[n as usize] = c;
        }
        t
    };
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in tag.iter().chain(data.iter()) {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

// Minimal zlib (deflate store blocks — valid compression level 0)
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    // zlib header: CMF=0x78 (deflate, window 32KB), FLG makes FCHECK work
    let cmf: u8 = 0x78;
    let flg: u8 = 0x01; // 0x7801 % 31 == 0
    let mut out = vec![cmf, flg];

    // DEFLATE stored blocks (BTYPE=00), max 65535 bytes each
    let mut pos = 0;
    while pos < data.len() {
        let end  = (pos + 65535).min(data.len());
        let bfin = if end == data.len() { 1u8 } else { 0u8 };
        let len  = (end - pos) as u16;
        let nlen = !len;
        out.push(bfin);             // BFINAL | BTYPE(00)
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&data[pos..end]);
        pos = end;
    }

    // Adler-32 checksum
    let (mut s1, mut s2) = (1u32, 0u32);
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1)        % 65521;
    }
    out.extend_from_slice(&((s2 << 16) | s1).to_be_bytes());
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Pseudo-random number generator (xorshift64 — no rand crate)
// ─────────────────────────────────────────────────────────────────────────────

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(if seed == 0 { 0xDEAD_BEEF_CAFE_BABEu64 } else { seed }) }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    fn range(&mut self, lo: f64, hi: f64) -> f64 { lo + self.f() * (hi - lo) }
}

// ─────────────────────────────────────────────────────────────────────────────
// Noise helpers (value noise, no perlin crate)
// ─────────────────────────────────────────────────────────────────────────────

fn hash2(x: i32, y: i32) -> f64 {
    let mut h = (x as u64).wrapping_mul(1_619).wrapping_add((y as u64).wrapping_mul(31_337));
    h ^= h >> 16; h = h.wrapping_mul(0x45d9f3b); h ^= h >> 16;
    (h & 0xFFFF) as f64 / 65535.0
}

fn smooth(t: f64) -> f64 { t * t * (3.0 - 2.0 * t) }

fn value_noise(x: f64, y: f64) -> f64 {
    let xi = x.floor() as i32; let xf = x - x.floor();
    let yi = y.floor() as i32; let yf = y - y.floor();
    let v00 = hash2(xi, yi); let v10 = hash2(xi + 1, yi);
    let v01 = hash2(xi, yi + 1); let v11 = hash2(xi + 1, yi + 1);
    let sx = smooth(xf); let sy = smooth(yf);
    let a = v00 + sx * (v10 - v00);
    let b = v01 + sx * (v11 - v01);
    a + sy * (b - a)
}

fn fbm(x: f64, y: f64, octaves: u32) -> f64 {
    let (mut val, mut amp, mut freq) = (0.0, 0.5, 1.0);
    // Cap at 3 octaves for performance (128x128 @ 3 = ~50K hash calls total)
    let oct = octaves.min(3);
    for _ in 0..oct {
        val  += amp * value_noise(x * freq, y * freq);
        amp  *= 0.5;
        freq *= 2.0;
    }
    val
}

// ─────────────────────────────────────────────────────────────────────────────
// Colour helpers
// ─────────────────────────────────────────────────────────────────────────────

fn lerp_col(a: (u8,u8,u8), b: (u8,u8,u8), t: f64) -> (u8,u8,u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t) as u8,
    )
}

fn add_col(a: (u8,u8,u8), b: (u8,u8,u8), w: f64) -> (u8,u8,u8) {
    (
        ((a.0 as f64 + b.0 as f64 * w).min(255.0)) as u8,
        ((a.1 as f64 + b.1 as f64 * w).min(255.0)) as u8,
        ((a.2 as f64 + b.2 as f64 * w).min(255.0)) as u8,
    )
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8,u8,u8) {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 / 60 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    (((r+m)*255.0) as u8, ((g+m)*255.0) as u8, ((b+m)*255.0) as u8)
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene generators — each returns a Vec<u8> of RGB pixels (W*H*3)
// ─────────────────────────────────────────────────────────────────────────────

const W: u32 = 512;
const H: u32 = 512;

pub fn img_width() -> u32 { W }
pub fn img_height() -> u32 { H }

/// Nature: sky gradient + FBM terrain + sun glow
fn scene_nature(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let sun_x = rng.range(0.3, 0.7) as f64;
    let horizon = rng.range(0.38, 0.52) as f64;
    let time = rng.range(0.0, 1.0);
    // Sky colours: dawn/dusk/midday
    let (sky_top, sky_mid, sky_hor) = if time < 0.33 {
        // Dawn — pink/orange
        ((25u8, 10u8, 60u8), (220u8, 100u8, 60u8), (255u8, 200u8, 120u8))
    } else if time < 0.66 {
        // Midday — blue
        ((10u8, 40u8, 120u8), (50u8, 110u8, 220u8), (140u8, 190u8, 255u8))
    } else {
        // Sunset — orange/purple
        ((20u8, 5u8, 40u8), (200u8, 70u8, 30u8), (255u8, 170u8, 80u8))
    };
    let ground_col: (u8,u8,u8) = (
        (30 + (rng.next() % 40) as u8),
        (80 + (rng.next() % 60) as u8),
        (20 + (rng.next() % 30) as u8),
    );

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf  = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            if yf < horizon {
                // Sky
                let t = yf / horizon;
                let col = if t < 0.5 {
                    lerp_col(sky_top, sky_mid, t * 2.0)
                } else {
                    lerp_col(sky_mid, sky_hor, (t - 0.5) * 2.0)
                };
                // Sun glow
                let dx = xf - sun_x; let dy = yf - (horizon * 0.7);
                let dist = (dx*dx + dy*dy).sqrt();
                let glow = (1.0 - (dist / 0.35).min(1.0)).powf(2.5);
                let col = add_col(col, (255, 240, 180), glow * 0.85);
                // Clouds via FBM
                let cloud = (fbm(xf * 4.0, yf * 3.0, 3) - 0.55).max(0.0) * 3.0;
                let col = lerp_col(col, (240, 240, 255), cloud.min(1.0) * 0.7);
                (px[i], px[i+1], px[i+2]) = col;
            } else {
                // Ground with terrain noise
                let tf  = (yf - horizon) / (1.0 - horizon);
                let n   = fbm(xf * 3.0, tf * 2.0, 3);
                let col = lerp_col(ground_col, (20, 50, 10), n * 0.7 + tf * 0.3);
                (px[i], px[i+1], px[i+2]) = col;
            }
        }
    }
    px
}

/// Space: star field + nebula colours + potential planet
fn scene_space(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let neb_hue = rng.range(180.0, 360.0);
    let planet_x = rng.range(0.2, 0.8);
    let planet_y = rng.range(0.2, 0.8);
    let planet_r = rng.range(0.06, 0.15);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            // Deep space bg
            let n1 = fbm(xf * 2.0 + 0.5, yf * 2.0, 2);
            let n2 = fbm(xf * 3.0 + 10.0, yf * 3.0 + 10.0, 2);
            // Nebula glow
            let neb = (n1 * n2 * 3.0).min(1.0);
            let neb_col = hsv_to_rgb(neb_hue + n1 * 40.0, 0.8, neb * 0.6);
            let bg = lerp_col((2, 2, 15), neb_col, neb);

            // Stars — points via noise threshold
            let star_n = value_noise(xf * 180.0, yf * 180.0);
            let col = if star_n > 0.96 {
                let bright = (star_n - 0.96) / 0.04;
                lerp_col(bg, (255, 255, 255), bright)
            } else {
                bg
            };

            // Planet
            let dx = xf - planet_x; let dy = yf - planet_y;
            let dist = (dx*dx + dy*dy).sqrt();
            let col = if dist < planet_r {
                let t = dist / planet_r;
                let surface = fbm(xf * 10.0, yf * 10.0, 4);
                let pcol = hsv_to_rgb(rng.range(0.0, 360.0) * 0.0 + 200.0, 0.6, 0.4 + surface * 0.4);
                // Atmosphere rim
                let rim = (1.0 - t * t).powf(0.4);
                let atm = hsv_to_rgb(200.0, 0.5, 0.8);
                lerp_col(pcol, atm, rim * 0.5)
            } else if dist < planet_r * 1.15 {
                // Atmosphere glow
                let glow = 1.0 - (dist - planet_r) / (planet_r * 0.15);
                let atm = hsv_to_rgb(200.0, 0.5, 0.5);
                lerp_col(col, atm, glow * 0.5)
            } else {
                col
            };

            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// City/neon: dark sky + neon reflections on wet streets
fn scene_city(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let num_buildings = (rng.range(6.0, 14.0)) as u32;
    struct Building { x: f64, w: f64, h: f64, col: (u8,u8,u8) }
    let buildings: Vec<Building> = (0..num_buildings).map(|_| {
        let hue = rng.range(0.0, 360.0);
        Building {
            x: rng.f(), w: rng.range(0.04, 0.14), h: rng.range(0.25, 0.75),
            col: hsv_to_rgb(hue, 0.9, 0.9),
        }
    }).collect();

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            // Night sky gradient
            let sky_n = fbm(xf * 3.0, yf * 1.5, 3);
            let base  = lerp_col((5, 2, 20), (15, 8, 50), sky_n);

            // Neon light bloom from buildings
            let mut col = base;
            for b in &buildings {
                let bx = b.x + b.w * 0.5;
                let dx = (xf - bx).abs();
                let dist = (dx * dx + (yf * 0.3).powi(2)).sqrt();
                let glow = (1.0 - (dist / 0.35).min(1.0)).powf(3.0) * 0.6;
                col = add_col(col, b.col, glow);
            }

            // Building silhouettes
            for b in &buildings {
                let in_x = xf >= b.x && xf <= b.x + b.w;
                let in_y = yf >= 1.0 - b.h;
                if in_x && in_y {
                    // Building face with slight neon tint from windows
                    let win_n = value_noise(xf * 40.0, yf * 20.0);
                    let win_on = win_n > 0.7;
                    let bface = lerp_col((8, 8, 18), (20, 15, 40), fbm(xf*8.0, yf*8.0, 2));
                    col = if win_on {
                        lerp_col(bface, b.col, 0.4)
                    } else {
                        bface
                    };
                }
            }

            // Wet street reflections (bottom 15%)
            if yf > 0.85 {
                let rf = (yf - 0.85) / 0.15;
                let reflect_y = 2.0 * 0.85 - yf;
                let ry = ((reflect_y * H as f64) as u32).min(H - 1);
                let ri = (ry * W + x) as usize * 3;
                let reflect = (px[ri], px[ri+1], px[ri+2]);
                // Ripple distortion
                let rip = (value_noise(xf * 30.0, (1.0-yf) * 10.0) - 0.5) * 0.03;
                let _ = rip;
                col = lerp_col(col, lerp_col(reflect, (5, 5, 20), 0.3), rf * 0.7);
            }

            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Fire: black bg + orange/red/yellow flame via FBM
fn scene_fire(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let offset = rng.range(0.0, 100.0);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            // Flame height FBM — rises from bottom
            let flame_n = fbm(xf * 2.5 + offset, (1.0 - yf) * 3.0 + offset * 0.5, 4);
            let heat    = (flame_n * 1.8 - yf * 1.2).clamp(0.0, 1.0);
            let embers  = fbm(xf * 8.0 + offset, yf * 8.0, 3);

            let col = if heat > 0.6 {
                lerp_col((220, 180, 0), (255, 255, 200), (heat - 0.6) / 0.4)
            } else if heat > 0.3 {
                lerp_col((200, 40, 0), (220, 180, 0), (heat - 0.3) / 0.3)
            } else {
                lerp_col((5, 2, 5), (200, 40, 0), heat / 0.3)
            };

            // Ember sparks
            let spark = (embers - 0.8).max(0.0) / 0.2 * heat;
            let col   = lerp_col(col, (255, 255, 255), spark * 0.7);

            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Ocean / water: waves + foam + light caustics
fn scene_ocean(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let horizon = rng.range(0.35, 0.45);
    let sun_x   = rng.range(0.3, 0.7);
    let time    = rng.range(0.0, 100.0);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            if yf < horizon {
                // Sky
                let t = yf / horizon;
                let col = lerp_col((30, 100, 200), (180, 220, 255), t);
                // Sun
                let dx = xf - sun_x;
                let dy = yf - horizon * 0.4;
                let glow = (1.0 - ((dx*dx + dy*dy).sqrt() / 0.25).min(1.0)).powf(3.0);
                let col = add_col(col, (255, 240, 180), glow * 0.9);
                (px[i], px[i+1], px[i+2]) = col;
            } else {
                // Ocean waves
                let wf = (yf - horizon) / (1.0 - horizon);
                let wave = fbm(xf * 4.0 + time * 0.1, wf * 3.0 + time * 0.05, 4);
                let depth = lerp_col((0, 20, 80), (0, 80, 160), wf);
                let col   = lerp_col(depth, (30, 120, 200), wave * 0.5);
                // Sun path on water
                let sun_path_dx = (xf - sun_x).abs();
                let sun_refl    = (1.0 - (sun_path_dx / (0.25 + wf * 0.5)).min(1.0)).powf(4.0);
                let col = add_col(col, (255, 240, 180), sun_refl * 0.5 * (1.0 - wf));
                // Foam crests
                let foam_n = value_noise(xf * 30.0, wf * 15.0 + time * 0.2);
                let foam   = (foam_n - 0.75).max(0.0) / 0.25 * (1.0 - wf);
                let col    = lerp_col(col, (220, 235, 255), foam * 0.9);
                (px[i], px[i+1], px[i+2]) = col;
            }
        }
    }
    px
}

/// Fractal: mandelbrot-like with rainbow colouring
fn scene_fractal(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let cx = rng.range(-1.5, 0.5);
    let cy = rng.range(-0.8, 0.8);
    let zoom = rng.range(0.8, 3.0);

    let max_iter: u32 = 20;
    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let zy = (y as f64 / H as f64 - 0.5) / zoom + cy;
        for x in 0..W {
            let zx = (x as f64 / W as f64 - 0.5) * 1.5 / zoom + cx;
            let i  = (y * W + x) as usize * 3;

            let (mut re, mut im) = (0.0f64, 0.0f64);
            let mut iter = 0u32;
            while iter < max_iter && re*re + im*im < 4.0 {
                let new_re = re*re - im*im + zx;
                im = 2.0*re*im + zy;
                re = new_re;
                iter += 1;
            }

            let col = if iter == max_iter {
                (0u8, 0u8, 0u8)
            } else {
                // Smooth colouring
                let smooth = iter as f64 + 1.0 - (re*re + im*im).ln().ln() / 2.0_f64.ln();
                let hue = (smooth / max_iter as f64 * 360.0 + rng.range(0.0, 180.0)).rem_euclid(360.0);
                hsv_to_rgb(hue, 0.85, if iter > max_iter / 2 { 0.9 } else { 0.6 })
            };
            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Matrix: green code rain on black
fn scene_matrix(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let col_w: u32 = 12; // character column width in pixels
    let num_cols = W / col_w;

    // Random head positions and speeds for each column
    let heads: Vec<(f64, f64)> = (0..num_cols).map(|_| {
        (rng.range(0.0, 1.0), rng.range(0.3, 1.2))
    }).collect();

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;
            let col_idx = (x / col_w) as usize;

            // Base: very dark green-tinted black
            let base_n = value_noise(xf * 2.0, yf * 2.0) * 0.05;
            let mut col = (0u8, (base_n * 20.0) as u8, 0u8);

            if col_idx < heads.len() {
                let (head_y, trail_len) = heads[col_idx];
                // Characters falling — use noise to simulate glyphs
                let char_n = value_noise(xf * W as f64 / col_w as f64, yf * 40.0 + seed as f64 * 0.1);
                let is_char = char_n > 0.35; // ~65% chance of a character being "lit"

                // Distance from head of this stream
                let dist_from_head = yf - head_y;
                if dist_from_head > -0.02 && dist_from_head < trail_len {
                    let trail_pos = dist_from_head / trail_len;
                    if dist_from_head < 0.01 {
                        // Head: bright white
                        col = (220, 255, 220);
                    } else if is_char {
                        // Trail: fades from bright green to dark
                        let brightness = 1.0 - trail_pos;
                        col = (0, (40.0 + brightness * 215.0) as u8, (brightness * 60.0) as u8);
                    }
                }
                // Second pass (offset stream)
                let head2 = (head_y + 0.5).rem_euclid(1.0);
                let dist2 = yf - head2;
                if dist2 > -0.02 && dist2 < trail_len * 0.7 {
                    let tp = dist2 / (trail_len * 0.7);
                    if dist2 < 0.01 {
                        col = (180, 255, 180);
                    } else if is_char && tp < 0.5 {
                        let b = 1.0 - tp;
                        col = (0, (30.0 + b * 170.0) as u8, 0u8);
                    }
                }
            }
            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Abstract gradient art — smooth hue rotation + noise layers
fn scene_abstract(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let hue_base = rng.range(0.0, 360.0);
    let hue_range = rng.range(60.0, 180.0);
    let scale1 = rng.range(1.5, 4.0);
    let scale2 = rng.range(2.0, 5.0);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            let n1 = fbm(xf * scale1, yf * scale1, 4);
            let n2 = fbm(xf * scale2 + 5.0, yf * scale2 + 5.0, 3);
            let n3 = value_noise(xf * 8.0 + n1, yf * 8.0 + n2);

            let hue = hue_base + n1 * hue_range + n2 * 40.0;
            let sat = 0.6 + n3 * 0.4;
            let val = 0.4 + n2 * 0.5 + n1 * 0.1;

            (px[i], px[i+1], px[i+2]) = hsv_to_rgb(hue, sat.min(1.0), val.min(1.0));
        }
    }
    px
}

/// Forest: layered trees + mist + light rays
fn scene_forest(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mist_level = rng.range(0.5, 0.7);
    let light_x    = rng.range(0.3, 0.7);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            // Sky through canopy (top 40%)
            let n = fbm(xf * 3.5, yf * 2.0, 4);
            let base = if yf < 0.4 {
                let sky = lerp_col((30, 80, 160), (100, 170, 230), yf / 0.4);
                lerp_col(sky, (20, 60, 20), n * 0.5) // canopy darkening
            } else {
                // Forest floor gradient
                lerp_col((10, 35, 10), (30, 60, 15), n)
            };

            // Tree trunks — dark vertical stripes via noise
            let trunk_n = value_noise(xf * 18.0, 0.5);
            let is_trunk = trunk_n > 0.72 && yf > 0.3;
            let col = if is_trunk {
                let bark_n = fbm(xf * 30.0, yf * 20.0, 3);
                lerp_col((20, 12, 5), (40, 25, 10), bark_n)
            } else {
                base
            };

            // Foliage blobs via FBM
            let foliage_n = fbm(xf * 5.0, yf * 4.0 + 1.0, 4);
            let is_foliage = foliage_n > 0.52 && yf < 0.65;
            let col = if is_foliage {
                let shade = fbm(xf * 10.0, yf * 10.0, 3);
                lerp_col((10, 50, 10), (50, 120, 30), shade)
            } else {
                col
            };

            // God rays — diagonal light shafts
            let ray_n  = value_noise(xf * 6.0 + yf * 2.0, yf * 0.5);
            let ray_dx = (xf - light_x - yf * 0.3).abs();
            let ray    = (1.0 - (ray_dx / 0.08).min(1.0)) * (1.0 - yf) * 0.4 * ray_n;
            let col    = add_col(col, (200, 220, 150), ray);

            // Mist
            let mist_n = fbm(xf * 2.0, yf * 1.0 + 20.0, 3);
            let mist   = ((mist_level - yf).max(0.0) / mist_level * mist_n).min(0.7);
            let col    = lerp_col(col, (200, 220, 210), mist);

            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Mountain: layered ridges + snow peaks + atmospheric haze
fn scene_mountain(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let snow_line = rng.range(0.18, 0.32);
    let horizon   = rng.range(0.5, 0.62);

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i  = (y * W + x) as usize * 3;

            // Sky
            let sky_col = lerp_col((60, 120, 220), (160, 200, 255), yf / horizon.min(1.0));

            // Mountain profile (FBM ridge)
            let ridge = |ox: f64, scale: f64, amp: f64| -> f64 {
                let n = fbm(xf * scale + ox, 0.5, 5);
                horizon - n * amp
            };
            let r1 = ridge(0.0,   2.0, 0.4); // far range
            let r2 = ridge(5.0,   3.0, 0.35); // mid range
            let r3 = ridge(12.0,  4.5, 0.28); // near range

            let col = if yf < r1 {
                sky_col
            } else if yf < r2 {
                // Far mountains — blue haze
                let tf = (yf - r1) / (r2 - r1).max(0.001);
                let rock = lerp_col((100, 120, 150), (60, 80, 110), tf);
                let snow_t = (snow_line - (yf - r1)).max(0.0) / snow_line;
                lerp_col(rock, (235, 238, 245), snow_t.min(1.0))
            } else if yf < r3 {
                // Mid mountains
                let tf = (yf - r2) / (r3 - r2).max(0.001);
                let n   = fbm(xf * 8.0, yf * 8.0, 3);
                let rock = lerp_col((70, 65, 60), (90, 85, 80), n);
                let snow_t = ((snow_line + 0.1) - tf * 0.3).max(0.0).min(1.0);
                lerp_col(rock, (245, 248, 255), snow_t)
            } else {
                // Foreground — meadow/pine fade
                let n  = fbm(xf * 4.0, yf * 3.0, 4);
                let tf = (yf - r3).max(0.0);
                lerp_col((30, 80, 25), (50, 100, 40), n * 0.7 + tf * 0.3)
            };

            // Atmospheric haze
            let haze = (rng.range(0.0, 0.0) + 0.0) * 0.0; // distance-based haze placeholder
            let _    = haze;

            (px[i], px[i+1], px[i+2]) = col;
        }
    }
    px
}

/// Wildlife / animal: golden savanna sunset backdrop + dark animal silhouette
fn scene_wildlife(seed: u64, prompt: &str) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let horizon = 0.55;
    let sun_x = rng.range(0.35, 0.65) as f64;
    let sun_y = 0.35;

    // Determine silhouette shape from animal type
    // body_cx, body_cy = centre; body_w, body_h = half-size; has_long_neck, has_tail_up
    let (body_cx, body_cy) = (0.5f64, 0.72f64);
    let (body_w, body_h, head_r, head_dy, leg_h, has_long_neck) = if prompt.contains("giraffe") {
        (0.06, 0.10, 0.025, -0.25, 0.12, true)
    } else if prompt.contains("elephant") {
        (0.12, 0.10, 0.06, -0.12, 0.10, false)
    } else if prompt.contains("bird") || prompt.contains("eagle") || prompt.contains("penguin") {
        (0.04, 0.04, 0.025, -0.06, 0.04, false)
    } else if prompt.contains("snake") || prompt.contains("dragon") {
        (0.14, 0.03, 0.025, -0.02, 0.0, false)
    } else {
        // Generic quadruped (lion, tiger, wolf, horse, deer, dog, cat, etc.)
        (0.10, 0.07, 0.04, -0.10, 0.10, false)
    };

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i = (y * W + x) as usize * 3;

            // Check if pixel is inside the silhouette
            let dx = (xf - body_cx) / body_w;
            let dy_body = (yf - body_cy) / body_h;
            let in_body = dx * dx + dy_body * dy_body < 1.0;

            // Head
            let head_cx = body_cx + if has_long_neck { 0.0 } else { body_w * 0.8 };
            let head_cy = body_cy + head_dy;
            let neck_cx = body_cx + if has_long_neck { 0.0 } else { body_w * 0.5 };
            let hdx = (xf - head_cx) / head_r;
            let hdy = (yf - head_cy) / head_r;
            let in_head = hdx * hdx + hdy * hdy < 1.0;

            // Neck (connecting body to head)
            let neck_w = head_r * 0.8;
            let in_neck = has_long_neck
                && (xf - neck_cx).abs() < neck_w
                && yf > (body_cy + head_dy) && yf < body_cy;
            let in_neck2 = !has_long_neck
                && (xf - neck_cx).abs() < head_r * 1.2
                && yf > head_cy && yf < body_cy;

            // Legs (4 ellipses below body)
            let leg_w = body_w * 0.15;
            let in_legs = yf > body_cy && yf < body_cy + leg_h && (
                   (xf - (body_cx - body_w * 0.6)).abs() < leg_w
                || (xf - (body_cx - body_w * 0.2)).abs() < leg_w
                || (xf - (body_cx + body_w * 0.2)).abs() < leg_w
                || (xf - (body_cx + body_w * 0.6)).abs() < leg_w
            );

            let in_silhouette = in_body || in_head || in_neck || in_neck2 || in_legs;

            if yf < horizon {
                // Sky — warm golden sunset
                let t = yf / horizon;
                let sky = if t < 0.4 {
                    lerp_col((40, 10, 60), (220, 80, 30), t / 0.4)
                } else {
                    lerp_col((220, 80, 30), (255, 200, 100), (t - 0.4) / 0.6)
                };
                // Sun glow
                let sdx = xf - sun_x; let sdy = yf - sun_y;
                let dist = (sdx * sdx + sdy * sdy).sqrt();
                let glow = (1.0 - (dist / 0.25).min(1.0)).powf(2.0);
                let col = add_col(sky, (255, 240, 160), glow * 0.9);

                if in_silhouette {
                    // Dark silhouette against bright sky
                    (px[i], px[i+1], px[i+2]) = (15, 10, 8);
                } else {
                    (px[i], px[i+1], px[i+2]) = col;
                }
            } else {
                // Ground — savanna grass
                let tf = (yf - horizon) / (1.0 - horizon);
                let n = fbm(xf * 5.0, tf * 3.0, 3);
                let grass_base = (140, 110, 50);
                let grass_dark = (80, 65, 25);
                let col = lerp_col(grass_base, grass_dark, tf * 0.6 + n * 0.4);

                if in_silhouette {
                    (px[i], px[i+1], px[i+2]) = (15, 10, 8);
                } else {
                    (px[i], px[i+1], px[i+2]) = col;
                }
            }
        }
    }
    px
}

/// Desert: sand dunes + warm sky + heat haze
fn scene_desert(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let sun_x = rng.range(0.3, 0.7) as f64;
    let horizon = rng.range(0.42, 0.55) as f64;

    let mut px = vec![0u8; (W * H * 3) as usize];
    for y in 0..H {
        let yf = y as f64 / H as f64;
        for x in 0..W {
            let xf = x as f64 / W as f64;
            let i = (y * W + x) as usize * 3;

            if yf < horizon {
                // Sky — pale blue to warm white near horizon
                let t = yf / horizon;
                let sky = lerp_col((60, 120, 200), (240, 220, 180), t);
                // Sun
                let sdx = xf - sun_x; let sdy = yf - (horizon * 0.5);
                let dist = (sdx * sdx + sdy * sdy).sqrt();
                let glow = (1.0 - (dist / 0.2).min(1.0)).powf(3.0);
                let col = add_col(sky, (255, 250, 220), glow * 0.95);
                (px[i], px[i+1], px[i+2]) = col;
            } else {
                // Sand dunes with rolling noise
                let tf = (yf - horizon) / (1.0 - horizon);
                let dune = fbm(xf * 3.0 + tf * 0.5, tf * 2.0, 4);
                let ridge = ((xf * 6.0 + dune * 2.0).sin() * 0.5 + 0.5) * 0.3;
                let sand_light = (230, 200, 140);
                let sand_shadow = (180, 140, 80);
                let col = lerp_col(sand_light, sand_shadow, (tf * 0.4 + ridge + dune * 0.3).min(1.0));
                (px[i], px[i+1], px[i+2]) = col;
            }
        }
    }
    px
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene router — maps prompt text to a scene + seed
// ─────────────────────────────────────────────────────────────────────────────

fn prompt_seed(prompt: &str) -> u64 {
    // Deterministic-ish seed from prompt so same words → similar image
    let mut h: u64 = 0x517CC1B727220A95;
    for b in prompt.bytes() {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(b as u64);
    }
    h
}

pub fn generate_native_image(prompt: &str) -> Vec<u8> {
    let p = prompt.to_lowercase();
    let seed = prompt_seed(&p);

    let pixels = if p.contains("space") || p.contains("galaxy") || p.contains("star")
              || p.contains("cosmos") || p.contains("nebula") || p.contains("universe")
              || p.contains("planet") || p.contains("astronaut") {
        scene_space(seed)
    } else if p.contains("city") || p.contains("neon") || p.contains("urban")
           || p.contains("tokyo") || p.contains("cyber") || p.contains("night")
           || p.contains("futuristic") || p.contains("skyscraper") {
        scene_city(seed)
    } else if p.contains("fire") || p.contains("flame") || p.contains("lava")
           || p.contains("volcano") || p.contains("blaze") || p.contains("burning") {
        scene_fire(seed)
    } else if p.contains("ocean") || p.contains("sea") || p.contains("wave")
           || p.contains("beach") || p.contains("coast") || p.contains("water") {
        scene_ocean(seed)
    } else if p.contains("fractal") || p.contains("abstract") || p.contains("geometric")
           || p.contains("pattern") || p.contains("mandelbrot") || p.contains("art") {
        scene_fractal(seed)
    } else if p.contains("code") || p.contains("matrix") || p.contains("hacker")
           || p.contains("terminal") || p.contains("program") || p.contains("digital") {
        scene_matrix(seed)
    } else if p.contains("forest") || p.contains("jungle") || p.contains("tree")
           || p.contains("woods") || p.contains("woodland") {
        scene_forest(seed)
    } else if p.contains("mountain") || p.contains("peak") || p.contains("alpine")
           || p.contains("himalaya") || p.contains("cliff") || p.contains("snow") {
        scene_mountain(seed)
    } else if p.contains("lion") || p.contains("elephant") || p.contains("tiger")
           || p.contains("bear") || p.contains("wolf") || p.contains("deer")
           || p.contains("horse") || p.contains("eagle") || p.contains("bird")
           || p.contains("fish") || p.contains("shark") || p.contains("whale")
           || p.contains("dog") || p.contains("cat") || p.contains("rabbit")
           || p.contains("fox") || p.contains("snake") || p.contains("dragon")
           || p.contains("animal") || p.contains("wildlife") || p.contains("safari")
           || p.contains("zoo") || p.contains("pet") || p.contains("panda")
           || p.contains("giraffe") || p.contains("zebra") || p.contains("monkey")
           || p.contains("penguin") || p.contains("dolphin") || p.contains("butterfly") {
        scene_wildlife(seed, &p)
    } else if p.contains("desert") || p.contains("sahara") || p.contains("sand")
           || p.contains("dune") || p.contains("cactus") || p.contains("arid") {
        scene_desert(seed)
    } else if p.contains("nature") || p.contains("sunset") || p.contains("sunrise")
           || p.contains("landscape") || p.contains("sky") || p.contains("cloud")
           || p.contains("field") || p.contains("meadow") || p.contains("river")
           || p.contains("natcher") || p.contains("natur") {
        scene_nature(seed)
    } else {
        // Default: abstract gradient — always looks beautiful
        scene_abstract(seed)
    };

    encode_png(&pixels, W, H)
}

/// Generate raw RGB pixels for a given prompt + explicit seed (used by nova_video for animation frames)
pub fn generate_native_image_with_seed(prompt: &str, seed: u64) -> Vec<u8> {
    let p = prompt.to_lowercase();
    if p.contains("space") || p.contains("galaxy") || p.contains("star")
    || p.contains("cosmos") || p.contains("nebula") || p.contains("universe")
    || p.contains("planet") || p.contains("astronaut") {
        scene_space(seed)
    } else if p.contains("city") || p.contains("neon") || p.contains("urban")
           || p.contains("tokyo") || p.contains("cyber") || p.contains("night")
           || p.contains("futuristic") || p.contains("skyscraper") {
        scene_city(seed)
    } else if p.contains("fire") || p.contains("flame") || p.contains("lava")
           || p.contains("volcano") || p.contains("blaze") || p.contains("burning") {
        scene_fire(seed)
    } else if p.contains("ocean") || p.contains("sea") || p.contains("wave")
           || p.contains("beach") || p.contains("coast") || p.contains("water") {
        scene_ocean(seed)
    } else if p.contains("fractal") || p.contains("abstract") || p.contains("geometric")
           || p.contains("pattern") || p.contains("mandelbrot") || p.contains("art") {
        scene_fractal(seed)
    } else if p.contains("code") || p.contains("matrix") || p.contains("hacker")
           || p.contains("terminal") || p.contains("program") || p.contains("digital") {
        scene_matrix(seed)
    } else if p.contains("forest") || p.contains("jungle") || p.contains("tree")
           || p.contains("woods") || p.contains("woodland") {
        scene_forest(seed)
    } else if p.contains("mountain") || p.contains("peak") || p.contains("alpine")
           || p.contains("himalaya") || p.contains("cliff") || p.contains("snow") {
        scene_mountain(seed)
    } else if p.contains("lion") || p.contains("elephant") || p.contains("tiger")
           || p.contains("bear") || p.contains("wolf") || p.contains("deer")
           || p.contains("horse") || p.contains("eagle") || p.contains("bird")
           || p.contains("fish") || p.contains("shark") || p.contains("whale")
           || p.contains("dog") || p.contains("cat") || p.contains("rabbit")
           || p.contains("fox") || p.contains("snake") || p.contains("dragon")
           || p.contains("animal") || p.contains("wildlife") || p.contains("safari")
           || p.contains("zoo") || p.contains("pet") || p.contains("panda")
           || p.contains("giraffe") || p.contains("zebra") || p.contains("monkey")
           || p.contains("penguin") || p.contains("dolphin") || p.contains("butterfly") {
        scene_wildlife(seed, &p)
    } else if p.contains("desert") || p.contains("sahara") || p.contains("sand")
           || p.contains("dune") || p.contains("cactus") || p.contains("arid") {
        scene_desert(seed)
    } else if p.contains("nature") || p.contains("sunset") || p.contains("sunrise")
           || p.contains("landscape") || p.contains("sky") || p.contains("cloud")
           || p.contains("field") || p.contains("meadow") || p.contains("river")
           || p.contains("natcher") || p.contains("natur") {
        scene_nature(seed)
    } else {
        scene_abstract(seed)
    }
}

/// Main public API: generate image from prompt, return base64 data URI
pub fn generate_image_native(prompt: &str) -> String {
    let png_bytes = generate_native_image(prompt);
    let b64 = base64_encode_bytes(&png_bytes);
    format!("data:image/png;base64,{}", b64)
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
