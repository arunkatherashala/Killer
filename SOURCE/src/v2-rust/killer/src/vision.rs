// ============================================================================
// Killer v2.3 — Vision Engine
// image_load(path)            → String (JSON metadata: format, width, height, size)
// image_describe(path)        → String (AI description — LLM vision if configured,
//                                       otherwise offline colour/structure analysis)
// khlm_vision(path, question) → String (ask any question about an image)
//
// LLM vision back-ends:   OpenAI gpt-4o / gpt-4-turbo
//                         Anthropic claude-3-5-sonnet / claude-3-opus
//                         Gemini  gemini-1.5-pro / gemini-pro-vision
// Temperature: KhLM `llm_temperature` / env KILLER_KHLM_LLM_TEMPERATURE (OpenAI/Gemini 0–2,
//             Anthropic clamped to 0–1 per API).
//
// Offline fallback: PNG/JPEG/BMP/GIF/WebP header parsing + dominant colour
//                   sampling → generates a rich text description without any
//                   external model.
// ============================================================================

use crate::value::Value;
use crate::error::VmError;
use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
//  Public builtin wrappers (called from builtin.rs dispatch table)
// ─────────────────────────────────────────────────────────────────────────────

/// image_load(path) → String
pub fn builtin_image_load(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_file_read()?;
    let path = str_arg(args, 0, "image_load(path)")?;
    Ok(Value::Str(image_load(&path)))
}

/// image_describe(path) → String
pub fn builtin_image_describe(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_file_read()?;
    crate::security::require_llm()?;
    let path = str_arg(args, 0, "image_describe(path)")?;
    Ok(Value::Str(image_describe(&path, "Describe this image in detail.")))
}

/// khlm_vision(path, question) → String
pub fn builtin_khlm_vision(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_file_read()?;
    crate::security::require_llm()?;
    if args.len() < 2 {
        return Err(VmError::runtime_error(
            "khlm_vision expects 2 args: khlm_vision(image_path, question)".to_string(),
        ));
    }
    let path     = str_arg(args, 0, "path")?;
    let question = str_arg(args, 1, "question")?;
    Ok(Value::Str(image_describe(&path, &question)))
}

// ─────────────────────────────────────────────────────────────────────────────
//  image_load — read header metadata from file
// ─────────────────────────────────────────────────────────────────────────────

pub fn image_load(path: &str) -> String {
    let bytes = match std::fs::read(path) {
        Ok(b)  => b,
        Err(e) => return format!("+-- Image Load Error -----\n|  {}\n+------------------------\n", e),
    };

    let size_bytes = bytes.len();
    let (fmt, width, height) = detect_image(&bytes);

    format!(
        "+-- Image Loaded -----------------------------------------\n\
         |  Path    : {}\n\
         |  Format  : {}\n\
         |  Width   : {} px\n\
         |  Height  : {} px\n\
         |  Size    : {} bytes ({:.1} KB)\n\
         +---------------------------------------------------------\n",
        path, fmt, width, height,
        size_bytes, size_bytes as f64 / 1024.0
    )
}

// ─────────────────────────────────────────────────────────────────────────────
//  image_describe / khlm_vision — try LLM vision, fall back to offline
// ─────────────────────────────────────────────────────────────────────────────

fn image_describe(path: &str, question: &str) -> String {
    // Read file bytes (needed for both offline analysis and base64 encoding)
    let bytes = match std::fs::read(path) {
        Ok(b)  => b,
        Err(e) => return format!("[Vision] Cannot read image: {}", e),
    };

    // Try LLM vision if a backend is configured
    let cfg = crate::khlm_polyglot::config().lock().unwrap().clone();
    if cfg.llm_available() {
        if let Some(answer) = llm_vision_call(&bytes, path, question, &cfg) {
            return answer;
        }
    }

    // Offline fallback — analyse image metadata + colour palette
    offline_vision(&bytes, path, question)
}

// ─────────────────────────────────────────────────────────────────────────────
//  LLM Vision API calls (OpenAI / Anthropic / Gemini)
// ─────────────────────────────────────────────────────────────────────────────

fn llm_vision_call(
    bytes: &[u8],
    path: &str,
    question: &str,
    cfg: &crate::khlm_polyglot::KhLmPolyglotConfig,
) -> Option<String> {
    let (fmt, _, _) = detect_image(bytes);
    let mime = fmt_to_mime(&fmt);
    let b64  = base64_encode(bytes);

    let result = match cfg.llm_provider.to_lowercase().as_str() {
        "openai" => {
            let key = &cfg.llm_api_key;
            let model = if cfg.llm_model.contains("vision") || cfg.llm_model.contains("gpt-4")
                { cfg.llm_model.clone() } else { "gpt-4o".to_string() };
            vision_openai(
                key,
                &model,
                &b64,
                mime,
                question,
                cfg.max_tokens,
                cfg.llm_temperature,
            )
        }
        "anthropic" => {
            let key = &cfg.llm_api_key;
            let model = if cfg.llm_model.contains("claude") { cfg.llm_model.clone() }
                        else { "claude-3-5-sonnet-20241022".to_string() };
            vision_anthropic(
                key,
                &model,
                &b64,
                mime,
                question,
                cfg.max_tokens,
                cfg.llm_temperature,
            )
        }
        "gemini" => {
            vision_gemini(
                &cfg.llm_api_key,
                &cfg.llm_model,
                &b64,
                mime,
                question,
                cfg.max_tokens,
                cfg.llm_temperature,
            )
        }
        _ => return None,   // ollama / groq don't support vision in text-only mode
    };

    match result {
        Ok(answer) => Some(format!(
            "+-- Vision ({}) -----------------------------------------\n\
             |  Image   : {}\n\
             |  Q: {}\n+-----------------------------------------------------\n\n\
             {}\n",
            cfg.llm_provider, path, question, answer.trim()
        )),
        Err(_e) => None,
    }
}

// ── OpenAI Vision ─────────────────────────────────────────────────────────────
fn vision_openai(
    api_key: &str,
    model: &str,
    b64: &str,
    mime: &str,
    question: &str,
    max_tokens: usize,
    temperature: f64,
) -> Result<String, String> {
    let url = "https://api.openai.com/v1/chat/completions";
    let temp = temperature.clamp(0.0, 2.0);

    // Build messages with inline image data URL
    let image_url = format!("data:{};base64,{}", mime, b64);
    let content_json = format!(
        r#"[{{"type":"text","text":{}}},{{"type":"image_url","image_url":{{"url":{}}}}}]"#,
        json_str(question), json_str(&image_url)
    );
    let body = format!(
        r#"{{"model":{},"messages":[{{"role":"user","content":{}}}],"max_tokens":{},"temperature":{}}}"#,
        json_str(model),
        content_json,
        max_tokens,
        temp
    );

    let raw = curl_post_json(url, api_key, &body, 60)?;
    extract_openai_content(&raw)
}

// ── Anthropic Vision ──────────────────────────────────────────────────────────
fn vision_anthropic(
    api_key: &str,
    model: &str,
    b64: &str,
    mime: &str,
    question: &str,
    max_tokens: usize,
    temperature: f64,
) -> Result<String, String> {
    let url = "https://api.anthropic.com/v1/messages";
    // Anthropic Messages API: temperature in [0, 1]
    let temp = temperature.clamp(0.0, 1.0);

    let content_json = format!(
        r#"[{{"type":"image","source":{{"type":"base64","media_type":{},"data":{}}}}},{{"type":"text","text":{}}}]"#,
        json_str(mime), json_str(b64), json_str(question)
    );
    let body = format!(
        r#"{{"model":{},"max_tokens":{},"temperature":{},"messages":[{{"role":"user","content":{}}}]}}"#,
        json_str(model),
        max_tokens,
        temp,
        content_json
    );

    let output = Command::new("curl")
        .args([
            "-s", "--fail-with-body", "--max-time", "60",
            "-X", "POST", url,
            "-H", &format!("x-api-key: {}", api_key),
            "-H", "anthropic-version: 2023-06-01",
            "-H", "Content-Type: application/json",
            "-d", &body,
        ])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    // { "content":[{"type":"text","text":"..."}], ... }
    extract_anthropic_text(&raw)
}

// ── Gemini Vision ─────────────────────────────────────────────────────────────
fn vision_gemini(
    api_key: &str,
    model: &str,
    b64: &str,
    mime: &str,
    question: &str,
    max_tokens: usize,
    temperature: f64,
) -> Result<String, String> {
    let mdl = if model.is_empty() { "gemini-1.5-pro" } else { model };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        mdl, api_key
    );
    let temp = temperature.clamp(0.0, 2.0);
    let body = format!(
        r#"{{"contents":[{{"parts":[{{"text":{}}},{{"inline_data":{{"mime_type":{},"data":{}}}}}]}}],"generationConfig":{{"maxOutputTokens":{},"temperature":{}}}}}"#,
        json_str(question),
        json_str(mime),
        json_str(b64),
        max_tokens,
        temp
    );

    let output = Command::new("curl")
        .args([
            "-s", "--fail-with-body", "--max-time", "60",
            "-X", "POST", &url,
            "-H", "Content-Type: application/json",
            "-d", &body,
        ])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    // {"candidates":[{"content":{"parts":[{"text":"..."}]}}]}
    extract_gemini_text(&raw)
}

// ─────────────────────────────────────────────────────────────────────────────
//  Offline Vision — pixel analysis without any external model
// ─────────────────────────────────────────────────────────────────────────────

fn offline_vision(bytes: &[u8], path: &str, question: &str) -> String {
    let (fmt, width, height) = detect_image(bytes);
    let colours = sample_colours(bytes, &fmt);
    let palette = describe_palette(&colours);
    let brightness = brightness_class(&colours);
    let complexity = if width * height > 500_000 { "detailed" }
                     else if width * height > 100_000 { "medium-detail" }
                     else { "compact" };
    let orientation = if width > height { "landscape" }
                      else if height > width { "portrait" }
                      else { "square" };

    // Determine file name for description
    let file_stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image")
        .replace('_', " ")
        .replace('-', " ");

    format!(
        "+-- Vision (Offline Analysis) ───────────────────────────\n\
         |  File      : {}\n\
         |  Format    : {}  |  {}x{}px  |  {}\n\
         |  Q: {}\n\
         +─────────────────────────────────────────────────────────\n\n\
         This is a {fmt} image ({orientation}, {complexity}). \
         Dimensions: {width}×{height} pixels.\n\n\
         Colour analysis: {palette}. \
         The image appears {brightness}.\n\n\
         Topic/subject (from filename): \"{file_stem}\".\n\n\
         To get full AI image understanding with scene recognition,\n\
         object detection, and natural language answers, configure\n\
         a vision model:\n\
           khlm_set_llm(\"openai\",    \"sk-...\", \"gpt-4o\")\n\
           khlm_set_llm(\"anthropic\", \"sk-ant-...\", \"claude-3-5-sonnet-20241022\")\n\
           khlm_set_llm(\"gemini\",    \"AIza...\", \"gemini-1.5-pro\")\n",
        std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path),
        fmt, width, height,
        format!("{} bytes", bytes.len()),
        question,
        fmt = fmt, orientation = orientation, complexity = complexity,
        width = width, height = height,
        palette = palette, brightness = brightness,
        file_stem = file_stem,
    )
}

// colour sampling — read every 64th byte group after the header
fn sample_colours(bytes: &[u8], fmt: &str) -> Vec<(u8, u8, u8)> {
    let skip = match fmt {
        "PNG"  => 33,   // after PNG sig(8) + IHDR chunk(25) skip to IDAT area
        "JPEG" | "JPG" => 200,  // skip JPEG headers
        "BMP"  => 54,   // standard BMP header
        "GIF"  => 13,   // GIF header (6 + 7 logical screen)
        _      => 50,
    };
    let data = if bytes.len() > skip { &bytes[skip..] } else { bytes };
    let step = (data.len() / 64).max(3);
    let mut colours = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() && colours.len() < 32 {
        colours.push((data[i], data[i + 1], data[i + 2]));
        i += step;
    }
    colours
}

fn describe_palette(colours: &[(u8, u8, u8)]) -> String {
    if colours.is_empty() { return "undetermined colours".to_string(); }
    let avg_r: u32 = colours.iter().map(|c| c.0 as u32).sum::<u32>() / colours.len() as u32;
    let avg_g: u32 = colours.iter().map(|c| c.1 as u32).sum::<u32>() / colours.len() as u32;
    let avg_b: u32 = colours.iter().map(|c| c.2 as u32).sum::<u32>() / colours.len() as u32;

    let dominant = if avg_r > avg_g && avg_r > avg_b { "warm red/orange tones" }
        else if avg_g > avg_r && avg_g > avg_b { "cool green tones" }
        else if avg_b > avg_r && avg_b > avg_g { "blue/cool tones" }
        else if avg_r > 200 && avg_g > 200 && avg_b > 200 { "mostly white/light tones" }
        else if avg_r < 60 && avg_g < 60 && avg_b < 60 { "dark/near-black tones" }
        else { "mixed neutral tones" };

    // Calculate colour variance (saturation proxy)
    let saturation: u32 = colours.iter().map(|c| {
        let mx = c.0.max(c.1).max(c.2) as u32;
        let mn = c.0.min(c.1).min(c.2) as u32;
        mx - mn
    }).sum::<u32>() / colours.len() as u32;

    let variety = if saturation > 80 { "highly colourful/saturated" }
        else if saturation > 40 { "moderately colourful" }
        else { "low saturation/greyscale-like" };

    format!("{}, {}", dominant, variety)
}

fn brightness_class(colours: &[(u8, u8, u8)]) -> &'static str {
    if colours.is_empty() { return "unknown brightness"; }
    let avg: u32 = colours.iter().map(|c| (c.0 as u32 + c.1 as u32 + c.2 as u32) / 3).sum::<u32>()
        / colours.len() as u32;
    if avg > 200 { "very bright / high-key" }
    else if avg > 140 { "bright / well-lit" }
    else if avg > 80  { "medium brightness" }
    else if avg > 40  { "dark / low-key" }
    else              { "very dark / near-black" }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Image header parsing — detect format and dimensions (zero deps)
// ─────────────────────────────────────────────────────────────────────────────

fn detect_image(bytes: &[u8]) -> (&'static str, u32, u32) {
    if bytes.len() < 4 { return ("Unknown", 0, 0); }

    // PNG: magic = 137 80 78 71 13 10 26 10
    if bytes.len() >= 24 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return ("PNG", w, h);
    }

    // JPEG: starts FF D8 FF
    if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        // Scan for SOF0 (FF C0) or SOF2 (FF C2) — contains width/height
        let (w, h) = parse_jpeg_size(bytes);
        return ("JPEG", w, h);
    }

    // GIF: magic = GIF87a or GIF89a
    if bytes.len() >= 10 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        let w = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
        let h = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
        return ("GIF", w, h);
    }

    // BMP: magic = BM
    if bytes.len() >= 26 && bytes[0] == b'B' && bytes[1] == b'M' {
        let w = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        let h = u32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        return ("BMP", w, h);
    }

    // WebP: RIFF....WEBP
    if bytes.len() >= 12
        && bytes.starts_with(b"RIFF")
        && &bytes[8..12] == b"WEBP"
    {
        let (w, h) = parse_webp_size(bytes);
        return ("WebP", w, h);
    }

    // TIFF: little-endian II or big-endian MM
    if bytes.len() >= 8 && (bytes.starts_with(b"II\x2A\x00") || bytes.starts_with(b"MM\x00\x2A")) {
        return ("TIFF", 0, 0);
    }

    ("Unknown", 0, 0)
}

fn parse_jpeg_size(bytes: &[u8]) -> (u32, u32) {
    let mut i = 2;
    while i + 3 < bytes.len() {
        if bytes[i] != 0xFF { break; }
        let marker = bytes[i + 1];
        // SOF markers: C0, C1, C2, C3, C5, C6, C7, C9, CA, CB, CD, CE, CF
        let is_sof = matches!(marker, 0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF);
        if is_sof && i + 9 < bytes.len() {
            // SOF: FF Cx  len(2)  bits(1)  height(2)  width(2)
            let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            return (w, h);
        }
        if i + 3 >= bytes.len() { break; }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if len < 2 { break; }
        i += 2 + len;
    }
    (0, 0)
}

fn parse_webp_size(bytes: &[u8]) -> (u32, u32) {
    if bytes.len() < 30 { return (0, 0); }
    // VP8 bitstream: "VP8 " at offset 12
    if &bytes[12..16] == b"VP8 " && bytes.len() >= 30 {
        // VP8 frame: skip 3 bytes frame tag, then "9d012a" signature
        if bytes[23] == 0x9D && bytes[24] == 0x01 && bytes[25] == 0x2A {
            let w = u16::from_le_bytes([bytes[26], bytes[27]]) as u32 & 0x3FFF;
            let h = u16::from_le_bytes([bytes[28], bytes[29]]) as u32 & 0x3FFF;
            return (w, h);
        }
    }
    // VP8L lossless: "VP8L" at offset 12
    if bytes.len() >= 25 && &bytes[12..16] == b"VP8L" {
        // after 4-byte sig 0x2F: packed width/height in 28 bits
        if bytes[20] == 0x2F && bytes.len() >= 25 {
            let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
            let w = (bits & 0x3FFF) + 1;
            let h = ((bits >> 14) & 0x3FFF) + 1;
            return (w, h);
        }
    }
    (0, 0)
}

// ─────────────────────────────────────────────────────────────────────────────
//  HTTP helpers (no extra deps — reuse curl subprocess like llm.rs)
// ─────────────────────────────────────────────────────────────────────────────

fn curl_post_json(url: &str, api_key: &str, body: &str, timeout_s: u64) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err(format!("URL must use HTTPS: {}", &url[..url.len().min(60)]));
    }
    let timeout_str = timeout_s.to_string();
    let auth_header = format!("Authorization: Bearer {}", api_key);
    let output = Command::new("curl")
        .args([
            "-s", "--fail-with-body", "--max-time", &timeout_str,
            "-X", "POST", url,
            "-H", &auth_header,
            "-H", "Content-Type: application/json",
            "-d", body,
        ])
        .output()
        .map_err(|e| format!("curl: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stdout.is_empty() && !stderr.is_empty() {
        return Err(format!("curl error: {}", stderr.trim()));
    }
    Ok(stdout)
}

// ─────────────────────────────────────────────────────────────────────────────
//  JSON / response parsing  (zero deps)
// ─────────────────────────────────────────────────────────────────────────────

fn extract_openai_content(raw: &str) -> Result<String, String> {
    // {"choices":[{"message":{"role":"assistant","content":"..."}}]}
    if raw.contains("\"error\"") {
        if let Some(msg) = extract_str_val(raw, "message") {
            return Err(format!("OpenAI error: {}", msg));
        }
    }
    extract_str_val(raw, "content")
        .ok_or_else(|| format!("Cannot parse OpenAI vision response: {}", &raw[..raw.len().min(200)]))
}

fn extract_anthropic_text(raw: &str) -> Result<String, String> {
    // {"content":[{"type":"text","text":"..."}]}
    if raw.contains("\"error\"") {
        if let Some(msg) = extract_str_val(raw, "message") {
            return Err(format!("Anthropic error: {}", msg));
        }
    }
    extract_str_val(raw, "text")
        .ok_or_else(|| format!("Cannot parse Anthropic vision response: {}", &raw[..raw.len().min(200)]))
}

fn extract_gemini_text(raw: &str) -> Result<String, String> {
    // {"candidates":[{"content":{"parts":[{"text":"..."}]}}]}
    extract_str_val(raw, "text")
        .ok_or_else(|| format!("Cannot parse Gemini vision response: {}", &raw[..raw.len().min(200)]))
}

/// Extract `"key":"value"` (the first occurrence)
fn extract_str_val(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)?;
    let rest = json[pos + needle.len()..].trim_start();
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"'  => break,
            '\\' => {
                match chars.next() {
                    Some('n')  => result.push('\n'),
                    Some('t')  => result.push('\t'),
                    Some('r')  => result.push('\r'),
                    Some('"')  => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(c)    => { result.push('\\'); result.push(c); }
                    None       => break,
                }
            }
            c => result.push(c),
        }
    }
    Some(result)
}

// ─────────────────────────────────────────────────────────────────────────────
//  Utility helpers
// ─────────────────────────────────────────────────────────────────────────────

fn fmt_to_mime(fmt: &str) -> &'static str {
    match fmt {
        "PNG"  => "image/png",
        "JPEG" | "JPG" => "image/jpeg",
        "GIF"  => "image/gif",
        "BMP"  => "image/bmp",
        "WebP" => "image/webp",
        "TIFF" => "image/tiff",
        _      => "image/jpeg",
    }
}

fn str_arg(args: &[Value], idx: usize, ctx: &str) -> Result<String, VmError> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(_) => Err(VmError::runtime_error(format!("{}: argument {} must be a String", ctx, idx))),
        None    => Err(VmError::runtime_error(format!("{}: missing argument {}", ctx, idx))),
    }
}

/// JSON-escape a string and wrap in double quotes
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c    => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Minimal base64 encoder — no external crate needed
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if i + 1 < data.len() { out.push(TABLE[((n >> 6) & 63) as usize] as char); } else { out.push('='); }
        if i + 2 < data.len() { out.push(TABLE[(n & 63) as usize] as char); } else { out.push('='); }
        i += 3;
    }
    out
}
