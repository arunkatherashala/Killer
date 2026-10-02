#![allow(unsafe_code)]
//! Minimal C FFI: load a shared library, look up a symbol, call it.
//!
//! Signature strings: `<args>><ret>`. Arg codes: `i`/`l` integer (passed as i64), `p` pointer (integer address),
//! `d` double, `s` C string. Return codes: `i` (i32), `l` (i64), `d`, `s`, `v` (void).
//! Mixing integer and double args is rejected: without libffi the call ABI differs
//! per platform, so each call must be all-integer-class (<=6 args) or all-double (<=4).

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Mutex;

#[cfg(windows)]
mod sys {
    use std::ffi::{c_char, c_void};
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(h: *mut c_void, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(h: *mut c_void) -> i32;
    }
    pub unsafe fn open(path: *const c_char) -> *mut c_void { LoadLibraryA(path) }
    pub unsafe fn sym(h: *mut c_void, name: *const c_char) -> *mut c_void { GetProcAddress(h, name) }
    pub unsafe fn close(h: *mut c_void) { FreeLibrary(h); }
}

#[cfg(unix)]
mod sys {
    use std::ffi::{c_char, c_void};
    pub unsafe fn open(path: *const c_char) -> *mut c_void { libc::dlopen(path, libc::RTLD_NOW) }
    pub unsafe fn sym(h: *mut c_void, name: *const c_char) -> *mut c_void { libc::dlsym(h, name) }
    pub unsafe fn close(h: *mut c_void) { libc::dlclose(h); }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FfiArg {
    Int(i64),
    Double(f64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FfiRet {
    Int(i64),
    Double(f64),
    Str(String),
    Void,
}

static LIBS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Open a library; returns a handle id (index + 1).
pub fn open(path: &str) -> Result<usize, String> {
    let c = CString::new(path).map_err(|_| "ffi_open: path contains NUL".to_string())?;
    let h = unsafe { sys::open(c.as_ptr()) };
    if h.is_null() {
        return Err(format!("ffi_open: cannot load '{}'", path));
    }
    let mut libs = LIBS.lock().unwrap();
    libs.push(h as usize);
    Ok(libs.len())
}

pub fn close(id: usize) -> Result<(), String> {
    let mut libs = LIBS.lock().unwrap();
    match libs.get_mut(id.wrapping_sub(1)) {
        Some(h) if *h != 0 => {
            unsafe { sys::close(*h as *mut c_void) };
            *h = 0;
            Ok(())
        }
        _ => Err(format!("ffi_close: invalid handle {}", id)),
    }
}

fn parse_sig(sig: &str) -> Result<(Vec<char>, char), String> {
    let (a, r) = sig.split_once('>').ok_or("ffi_call: signature must look like \"dd>d\"")?;
    let args: Vec<char> = a.chars().collect();
    if args.iter().any(|c| !matches!(c, 'i' | 'l' | 'p' | 'd' | 's')) {
        return Err("ffi_call: arg codes are i, l, p, d, s".into());
    }
    let mut rc = r.chars();
    let ret = rc.next().ok_or("ffi_call: missing return code")?;
    if rc.next().is_some() || !matches!(ret, 'i' | 'l' | 'p' | 'd' | 's' | 'v') {
        return Err("ffi_call: return code is one of i, l, p, d, s, v".into());
    }
    Ok((args, ret))
}

macro_rules! call_ints {
    ($f:expr, $a:expr, $ret_double:expr; $($i:tt),*) => {{
        if $ret_double {
            let f: extern "C" fn($(call_ints!(@t $i)),*) -> f64 = std::mem::transmute($f);
            FfiRet::Double(f($($a[$i]),*))
        } else {
            let f: extern "C" fn($(call_ints!(@t $i)),*) -> i64 = std::mem::transmute($f);
            FfiRet::Int(f($($a[$i]),*))
        }
    }};
    (@t $i:tt) => { i64 };
}

macro_rules! call_doubles {
    ($f:expr, $a:expr, $ret_int:expr; $($i:tt),*) => {{
        if $ret_int {
            let f: extern "C" fn($(call_doubles!(@t $i)),*) -> i64 = std::mem::transmute($f);
            FfiRet::Int(f($($a[$i]),*))
        } else {
            let f: extern "C" fn($(call_doubles!(@t $i)),*) -> f64 = std::mem::transmute($f);
            FfiRet::Double(f($($a[$i]),*))
        }
    }};
    (@t $i:tt) => { f64 };
}

pub fn call(id: usize, symbol: &str, sig: &str, args: &[FfiArg]) -> Result<FfiRet, String> {
    let (codes, ret) = parse_sig(sig)?;
    if codes.len() != args.len() {
        return Err(format!("ffi_call: signature has {} args, got {}", codes.len(), args.len()));
    }
    let handle = {
        let libs = LIBS.lock().unwrap();
        match libs.get(id.wrapping_sub(1)) {
            Some(h) if *h != 0 => *h,
            _ => return Err(format!("ffi_call: invalid handle {}", id)),
        }
    };
    let csym = CString::new(symbol).map_err(|_| "ffi_call: symbol contains NUL".to_string())?;
    let f = unsafe { sys::sym(handle as *mut c_void, csym.as_ptr()) };
    if f.is_null() {
        return Err(format!("ffi_call: symbol '{}' not found", symbol));
    }

    let any_double = codes.contains(&'d');
    let any_int = codes.iter().any(|c| *c != 'd');
    if any_double && any_int {
        return Err("ffi_call: mixing double and integer/string args is not supported".into());
    }

    let mut keep: Vec<CString> = Vec::new();
    let mut ints: Vec<i64> = Vec::new();
    let mut dbls: Vec<f64> = Vec::new();
    for (c, a) in codes.iter().zip(args) {
        match (c, a) {
            ('i' | 'l' | 'p', FfiArg::Int(v)) => ints.push(*v),
            ('i' | 'l' | 'p', FfiArg::Double(v)) if v.fract() == 0.0 => ints.push(*v as i64),
            ('d', FfiArg::Double(v)) => dbls.push(*v),
            ('d', FfiArg::Int(v)) => dbls.push(*v as f64),
            ('s', FfiArg::Str(s)) => {
                let cs = CString::new(s.as_str()).map_err(|_| "ffi_call: string arg contains NUL".to_string())?;
                ints.push(cs.as_ptr() as i64);
                keep.push(cs);
            }
            _ => return Err(format!("ffi_call: argument does not match code '{}'", c)),
        }
    }

    let mut out = unsafe {
        if any_double {
            let ret_int = matches!(ret, 'i' | 'l' | 'p' | 's');
            match dbls.len() {
                1 => call_doubles!(f, dbls, ret_int; 0),
                2 => call_doubles!(f, dbls, ret_int; 0, 1),
                3 => call_doubles!(f, dbls, ret_int; 0, 1, 2),
                4 => call_doubles!(f, dbls, ret_int; 0, 1, 2, 3),
                _ => return Err("ffi_call: at most 4 double args".into()),
            }
        } else {
            let rd = ret == 'd';
            match ints.len() {
                0 => call_ints!(f, ints, rd;),
                1 => call_ints!(f, ints, rd; 0),
                2 => call_ints!(f, ints, rd; 0, 1),
                3 => call_ints!(f, ints, rd; 0, 1, 2),
                4 => call_ints!(f, ints, rd; 0, 1, 2, 3),
                5 => call_ints!(f, ints, rd; 0, 1, 2, 3, 4),
                6 => call_ints!(f, ints, rd; 0, 1, 2, 3, 4, 5),
                _ => return Err("ffi_call: at most 6 integer/string args".into()),
            }
        }
    };
    drop(keep);

    out = match (ret, out) {
        ('v', _) => FfiRet::Void,
        ('i', FfiRet::Int(v)) => FfiRet::Int(v as i32 as i64),
        ('s', FfiRet::Int(p)) => {
            if p == 0 {
                FfiRet::Str(String::new())
            } else {
                let s = unsafe { CStr::from_ptr(p as *const c_char) };
                FfiRet::Str(s.to_string_lossy().into_owned())
            }
        }
        (_, other) => other,
    };
    Ok(out)
}

// ---- Bounds-checked native memory (for structs / out-params / buffers) ----
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::collections::HashMap;

static BLOCKS: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);

pub fn mem_alloc(size: usize) -> Result<usize, String> {
    if size == 0 || size > (1 << 30) {
        return Err("ffi_alloc: size must be 1..=1GiB".into());
    }
    let layout = Layout::from_size_align(size, 16).map_err(|e| e.to_string())?;
    let p = unsafe { alloc_zeroed(layout) };
    if p.is_null() {
        return Err("ffi_alloc: out of memory".into());
    }
    BLOCKS.lock().unwrap().get_or_insert_with(HashMap::new).insert(p as usize, size);
    Ok(p as usize)
}

pub fn mem_free(ptr: usize) -> Result<(), String> {
    let size = BLOCKS.lock().unwrap().as_mut().and_then(|m| m.remove(&ptr))
        .ok_or_else(|| "ffi_free: not a pointer from ffi_alloc (or already freed)".to_string())?;
    unsafe { dealloc(ptr as *mut u8, Layout::from_size_align(size, 16).unwrap()) };
    Ok(())
}

fn check(ptr: usize, off: usize, len: usize) -> Result<usize, String> {
    let guard = BLOCKS.lock().unwrap();
    let size = guard.as_ref().and_then(|m| m.get(&ptr)).ok_or("ffi: pointer was not allocated by ffi_alloc")?;
    match off.checked_add(len) {
        Some(end) if end <= *size => Ok(ptr + off),
        _ => Err(format!("ffi: access out of bounds (offset {} len {} of {})", off, len, size)),
    }
}

fn kind_len(kind: &str) -> Result<usize, String> {
    Ok(match kind { "u8" | "i8" => 1, "i16" => 2, "i32" => 4, "i64" | "f64" => 8, "f32" => 4,
        _ => return Err(format!("ffi: unknown kind '{}' (u8 i8 i16 i32 i64 f32 f64 str)", kind)) })
}

pub fn poke(ptr: usize, off: usize, kind: &str, v: &FfiArg) -> Result<(), String> {
    if kind == "str" {
        let FfiArg::Str(s) = v else { return Err("ffi_poke: 'str' needs a string".into()) };
        let bytes = s.as_bytes();
        if bytes.contains(&0) { return Err("ffi_poke: string contains NUL".into()); }
        let dst = check(ptr, off, bytes.len() + 1)? as *mut u8;
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len()); *dst.add(bytes.len()) = 0; }
        return Ok(());
    }
    let n = kind_len(kind)?;
    let dst = check(ptr, off, n)?;
    let (i, d) = match v {
        FfiArg::Int(i) => (*i, *i as f64),
        FfiArg::Double(d) => (*d as i64, *d),
        FfiArg::Str(_) => return Err("ffi_poke: numeric kind needs a number".into()),
    };
    unsafe {
        match kind {
            "u8" | "i8" => (dst as *mut u8).write_unaligned(i as u8),
            "i16" => (dst as *mut i16).write_unaligned(i as i16),
            "i32" => (dst as *mut i32).write_unaligned(i as i32),
            "i64" => (dst as *mut i64).write_unaligned(i),
            "f32" => (dst as *mut f32).write_unaligned(d as f32),
            _ => (dst as *mut f64).write_unaligned(d),
        }
    }
    Ok(())
}

pub fn peek(ptr: usize, off: usize, kind: &str) -> Result<FfiRet, String> {
    if kind == "str" {
        let size = BLOCKS.lock().unwrap().as_ref().and_then(|m| m.get(&ptr).copied())
            .ok_or("ffi: pointer was not allocated by ffi_alloc")?;
        if off >= size { return Err("ffi_peek: offset out of bounds".into()); }
        let base = (ptr + off) as *const u8;
        let max = size - off;
        let bytes: Vec<u8> = (0..max).map(|i| unsafe { *base.add(i) }).take_while(|b| *b != 0).collect();
        return Ok(FfiRet::Str(String::from_utf8_lossy(&bytes).into_owned()));
    }
    let n = kind_len(kind)?;
    let src = check(ptr, off, n)?;
    unsafe {
        Ok(match kind {
            "u8" => FfiRet::Int((src as *const u8).read_unaligned() as i64),
            "i8" => FfiRet::Int((src as *const i8).read_unaligned() as i64),
            "i16" => FfiRet::Int((src as *const i16).read_unaligned() as i64),
            "i32" => FfiRet::Int((src as *const i32).read_unaligned() as i64),
            "i64" => FfiRet::Int((src as *const i64).read_unaligned()),
            "f32" => FfiRet::Double((src as *const f32).read_unaligned() as f64),
            _ => FfiRet::Double((src as *const f64).read_unaligned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    const LIBC: &str = "msvcrt.dll";
    #[cfg(target_os = "macos")]
    const LIBC: &str = "libc.dylib";
    #[cfg(all(unix, not(target_os = "macos")))]
    const LIBC: &str = "libc.so.6";

    #[cfg(windows)]
    const MATH: &str = "msvcrt.dll";
    #[cfg(target_os = "macos")]
    const MATH: &str = "libm.dylib";
    #[cfg(all(unix, not(target_os = "macos")))]
    const MATH: &str = "libm.so.6";

    #[test]
    fn calls_pow() {
        let h = open(MATH).unwrap();
        let r = call(h, "pow", "dd>d", &[FfiArg::Double(2.0), FfiArg::Double(10.0)]).unwrap();
        assert_eq!(r, FfiRet::Double(1024.0));
        close(h).unwrap();
    }

    #[test]
    fn calls_strlen_and_labs() {
        let h = open(LIBC).unwrap();
        let r = call(h, "strlen", "s>l", &[FfiArg::Str("killer".into())]).unwrap();
        assert_eq!(r, FfiRet::Int(6));
        let r = call(h, "labs", "l>l", &[FfiArg::Int(-42)]).unwrap();
        assert_eq!(r, FfiRet::Int(42));
        close(h).unwrap();
    }

    #[test]
    fn rejects_bad_input() {
        assert!(open("definitely_not_a_real_library_xyz").is_err());
        let h = open(LIBC).unwrap();
        assert!(call(h, "no_such_symbol_xyz", ">v", &[]).is_err());
        assert!(call(h, "strlen", "sd>l", &[FfiArg::Str("a".into()), FfiArg::Double(1.0)]).is_err());
        assert!(call(h, "strlen", "s>l", &[]).is_err());
        close(h).unwrap();
    }

    #[test]
    fn memory_roundtrip_and_bounds() {
        let p = mem_alloc(16).unwrap();
        poke(p, 0, "i32", &FfiArg::Int(-7)).unwrap();
        poke(p, 8, "f64", &FfiArg::Double(2.5)).unwrap();
        assert_eq!(peek(p, 0, "i32").unwrap(), FfiRet::Int(-7));
        assert_eq!(peek(p, 8, "f64").unwrap(), FfiRet::Double(2.5));
        assert!(poke(p, 12, "i64", &FfiArg::Int(1)).is_err());
        assert!(peek(p, 16, "u8").is_err());
        poke(p, 0, "str", &FfiArg::Str("hi".into())).unwrap();
        assert_eq!(peek(p, 0, "str").unwrap(), FfiRet::Str("hi".into()));
        mem_free(p).unwrap();
        assert!(mem_free(p).is_err());
        assert!(peek(p, 0, "u8").is_err());
    }

    #[test]
    fn c_function_fills_buffer() {
        let h = open(LIBC).unwrap();
        let buf = mem_alloc(8).unwrap();
        call(h, "memset", "pil>p", &[FfiArg::Int(buf as i64), FfiArg::Int(65), FfiArg::Int(3)]).unwrap();
        assert_eq!(peek(buf, 0, "str").unwrap(), FfiRet::Str("AAA".into()));
        let n = call(h, "strlen", "p>l", &[FfiArg::Int(buf as i64)]).unwrap();
        assert_eq!(n, FfiRet::Int(3));
        mem_free(buf).unwrap();
        close(h).unwrap();
    }
}
