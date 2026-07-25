// nova_audio.rs — Native WAV audio synthesis for Kala
// Pure Rust, zero crates, zero network.
//
// 5 audio scenes: ambient · nature · space · beat · ocean
// Format: 22050 Hz · 16-bit PCM · Mono · ~3 seconds
// Output: "data:audio/wav;base64,..." ready for <audio> tag

const SAMPLE_RATE: u32 = 22_050;
const CHANNELS:    u16 = 1;
const BITS:        u16 = 16;
const DURATION:    f32 = 3.0;   // seconds

const PI2: f32 = std::f32::consts::PI * 2.0;

// ── WAV header (44 bytes) ─────────────────────────────────────
fn wav_header(num_samples: u32) -> Vec<u8> {
    let data_len  = num_samples * (BITS / 8) as u32 * CHANNELS as u32;
    let byte_rate = SAMPLE_RATE * CHANNELS as u32 * (BITS / 8) as u32;
    let block_aln = CHANNELS * BITS / 8;
    let mut h: Vec<u8> = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes());          // PCM
    h.extend_from_slice(&CHANNELS.to_le_bytes());
    h.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_aln.to_le_bytes());
    h.extend_from_slice(&BITS.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    h
}

// ── xorshift64 RNG ────────────────────────────────────────────
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 { (self.next() & 0xFFFF) as f32 / 65535.0 }
    fn sf(&mut self) -> f32 { self.f() * 2.0 - 1.0 }  // -1..1
}

// ── Oscillators ───────────────────────────────────────────────
#[inline] fn sine(t: f32, freq: f32) -> f32 { (PI2 * freq * t).sin() }
#[inline] fn env_adsr(t: f32, total: f32, a: f32, d: f32, sus: f32, r: f32) -> f32 {
    let fade = total - r;
    if t < a                { t / a }
    else if t < a + d       { 1.0 - (1.0 - sus) * (t - a) / d }
    else if t < fade        { sus }
    else if t < total       { sus * (1.0 - (t - fade) / r) }
    else                    { 0.0 }
}

// ── Scene: deep ambient pad ───────────────────────────────────
fn scene_ambient(seed: u64) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut rng = Rng(seed | 1);
    let root = 55.0 + rng.f() * 25.0;             // 55–80 Hz fundamental
    let harmos = [1.0f32, 1.498, 2.0, 2.997];     // roots, perfect fifths, octaves
    (0..n).map(|i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = env_adsr(t, DURATION, 0.5, 0.3, 0.75, 0.7);
        let lfo  = 1.0 + 0.12 * sine(t, 0.35);    // slow tremolo
        let s: f32 = harmos.iter().zip([0.40f32, 0.28, 0.18, 0.10])
            .map(|(&h, a)| sine(t, root * h) * a)
            .sum();
        ((s * env * lfo).clamp(-1.0, 1.0) * 28_000.0) as i16
    }).collect()
}

// ── Scene: nature sounds (rain + birds) ──────────────────────
fn scene_nature(seed: u64) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut rng = Rng(seed | 1);
    let bf1  = 1_900.0 + rng.f() * 600.0;
    let bf2  = 1_600.0 + rng.f() * 700.0;
    let bird1_start = (SAMPLE_RATE as f32 * 0.35) as usize;
    let bird2_start = (SAMPLE_RATE as f32 * 1.90) as usize;
    let mut lpf = 0.0f32;
    (0..n).map(|i| {
        let raw    = rng.sf();
        lpf        = lpf * 0.93 + raw * 0.07;
        let rain   = lpf * 0.28;
        let b1     = if i >= bird1_start {
            let bt  = (i - bird1_start) as f32 / SAMPLE_RATE as f32;
            let env = env_adsr(bt, 0.45, 0.02, 0.06, 0.55, 0.12);
            let fc  = bf1 + 280.0 * sine(bt, 9.0);
            sine(bt, fc) * env * 0.38
        } else { 0.0 };
        let b2     = if i >= bird2_start {
            let bt  = (i - bird2_start) as f32 / SAMPLE_RATE as f32;
            let env = env_adsr(bt, 0.38, 0.01, 0.04, 0.50, 0.10);
            let fc  = bf2 + 180.0 * sine(bt, 11.0);
            sine(bt, fc) * env * 0.32
        } else { 0.0 };
        ((rain + b1 + b2).clamp(-1.0, 1.0) * 28_000.0) as i16
    }).collect()
}

// ── Scene: space drone ────────────────────────────────────────
fn scene_space(seed: u64) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut rng = Rng(seed | 1);
    let base = 38.0 + rng.f() * 18.0;
    (0..n).map(|i| {
        let t   = i as f32 / SAMPLE_RATE as f32;
        let env = env_adsr(t, DURATION, 0.9, 0.4, 0.65, 0.9);
        let d   = sine(t, base) * 0.50 + sine(t, base * 2.0) * 0.28 + sine(t, base * 3.0) * 0.14;
        let shim_f = 760.0 + 180.0 * sine(t, 0.13);
        let shim   = sine(t, shim_f) * 0.10;
        ((d * env + shim * env * 0.5).clamp(-1.0, 1.0) * 28_000.0) as i16
    }).collect()
}

// ── Scene: drum beat ─────────────────────────────────────────
fn scene_beat(seed: u64) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut rng = Rng(seed | 1);
    let bpm      = 120.0 + rng.f() * 20.0;
    let beat     = (SAMPLE_RATE as f32 * 60.0 / bpm) as usize;
    let half     = beat / 2;
    let eighth   = beat / 4;
    (0..n).map(|i| {
        let pb = i % beat;
        // Kick: frequency-swept sine, fast attack/decay
        let kick = if pb < 2_200 {
            let ke = (-(pb as f32) / 900.0).exp();
            let kf = 78.0 - 55.0 * (pb as f32 / 900.0).min(1.0);
            sine(pb as f32 / SAMPLE_RATE as f32, kf) * ke * 0.82
        } else { 0.0 };
        // Hi-hat: noise burst on every eighth note
        let hihat = if pb % eighth < 700 {
            let he = (-((pb % eighth) as f32) / 160.0).exp();
            rng.sf() * he * 0.14
        } else { 0.0 };
        // Snare: white noise + mid tone on beats 2 & 4
        let ph = i % half;
        let snare = if ph >= half.saturating_sub(2_600) && ph < half.saturating_sub(200) {
            let sp  = ph - half.saturating_sub(2_600);
            let se  = (-(sp as f32) / 650.0).exp();
            let sn  = rng.sf() * 0.65 + sine(sp as f32 / SAMPLE_RATE as f32, 220.0) * 0.25;
            sn * se
        } else { 0.0 };
        ((kick + hihat + snare).clamp(-1.0, 1.0) * 28_000.0) as i16
    }).collect()
}

// ── Scene: ocean waves ────────────────────────────────────────
fn scene_ocean(seed: u64) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut rng = Rng(seed | 1);
    let mut lp1 = 0.0f32;
    let mut lp2 = 0.0f32;
    (0..n).map(|i| {
        let t    = i as f32 / SAMPLE_RATE as f32;
        let raw  = rng.sf();
        lp1      = lp1 * 0.990 + raw * 0.010;  // heavy low-pass
        lp2      = lp2 * 0.978 + lp1 * 0.022;
        let swell = 0.45 + 0.55 * sine(t, 0.38);  // slow swell ~2.6s cycle
        let rumble = sine(t, 52.0) * 0.12 + sine(t, 78.0) * 0.07;
        ((lp2 * swell * 1.6 + rumble).clamp(-1.0, 1.0) * 28_000.0) as i16
    }).collect()
}

// ── Prompt → scene selection ──────────────────────────────────
fn prompt_seed(prompt: &str) -> u64 {
    let mut h: u64 = 0x517CC1B727220A95;
    for b in prompt.bytes() { h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(b as u64); }
    h | 1
}

fn select_scene(prompt: &str) -> u8 {
    let p = prompt.to_lowercase();
    if p.contains("space") || p.contains("galaxy") || p.contains("cosmos") || p.contains("alien") || p.contains("universe") { 2 }
    else if p.contains("beat") || p.contains("music") || p.contains("drum") || p.contains("dance")
         || p.contains("song") || p.contains("rhythm") || p.contains("hip") || p.contains("bass") { 3 }
    else if p.contains("ocean") || p.contains("wave") || p.contains("sea") || p.contains("beach")
         || p.contains("rain") || p.contains("water") { 4 }
    else if p.contains("nature") || p.contains("bird") || p.contains("forest") || p.contains("wind")
         || p.contains("tree") || p.contains("jungle") { 1 }
    else { 0 }   // ambient default
}

/// Human-readable scene label for a prompt
pub fn scene_name(prompt: &str) -> &'static str {
    match select_scene(prompt) {
        1 => "Nature (birdsong + rain)",
        2 => "Space drone",
        3 => "Beat / rhythm",
        4 => "Ocean waves",
        _ => "Ambient pad",
    }
}

/// Generate raw WAV bytes for the given prompt
pub fn generate_native_audio(prompt: &str) -> Vec<u8> {
    let seed    = prompt_seed(prompt);
    let samples = match select_scene(prompt) {
        1 => scene_nature(seed),
        2 => scene_space(seed),
        3 => scene_beat(seed),
        4 => scene_ocean(seed),
        _ => scene_ambient(seed),
    };
    let mut wav = wav_header(samples.len() as u32);
    for s in &samples { wav.extend_from_slice(&s.to_le_bytes()); }
    wav
}

/// Generate WAV as "data:audio/wav;base64,..." ready for <audio> tag
pub fn generate_audio_native(prompt: &str) -> String {
    let bytes = generate_native_audio(prompt);
    let b64   = base64_encode_bytes(&bytes);
    format!("data:audio/wav;base64,{}", b64)
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
