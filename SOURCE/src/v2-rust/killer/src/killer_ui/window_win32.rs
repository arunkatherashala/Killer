//! **Win32 native window** — creates a real OS window via raw Win32 API, blits a [`Framebuffer`],
//! and pumps mouse/keyboard events into [`super::events::EventDispatcher`].
//!
//! Zero external dependencies — pure `extern "system"` FFI to user32.dll + gdi32.dll + kernel32.dll.
//! Windows-only (`#[cfg(target_os = "windows")]`).

#![allow(unsafe_code)]

#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "C" {}

#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
extern "C" {}

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use super::framebuffer::Framebuffer;

// ── Win32 type aliases ───────────────────────────────────────────────────────

type HWND = *mut c_void;
type HDC = *mut c_void;
type HINSTANCE = *mut c_void;
type HBRUSH = *mut c_void;
type HCURSOR = *mut c_void;
type HICON = *mut c_void;
type HMENU = *mut c_void;
type LPARAM = isize;
type WPARAM = usize;
type LRESULT = isize;
type ATOM = u16;
type UINT = u32;
type DWORD = u32;
type LONG = i32;
type BOOL = i32;
type BYTE = u8;
type WORD = u16;

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: UINT,
    style: UINT,
    lpfnWndProc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: HICON,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt_x: LONG,
    pt_y: LONG,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint_left: LONG,
    rcPaint_top: LONG,
    rcPaint_right: LONG,
    rcPaint_bottom: LONG,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [BYTE; 32],
}

#[repr(C)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}

#[repr(C)]
#[allow(dead_code)]
struct BITMAPINFOHEADER {
    biSize: DWORD,
    biWidth: LONG,
    biHeight: LONG,
    biPlanes: WORD,
    biBitCount: WORD,
    biCompression: DWORD,
    biSizeImage: DWORD,
    biXPelsPerMeter: LONG,
    biYPelsPerMeter: LONG,
    biClrUsed: DWORD,
    biClrImportant: DWORD,
}

// ── Win32 constants ──────────────────────────────────────────────────────────

const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;
const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
const WS_VISIBLE: DWORD = 0x10000000;
const CW_USEDEFAULT: i32 = -2147483648i32; // 0x80000000

const WM_DESTROY: UINT = 0x0002;
const WM_PAINT: UINT = 0x000F;
const WM_SIZE: UINT = 0x0005;
const WM_CLOSE: UINT = 0x0010;
const WM_MOUSEMOVE: UINT = 0x0200;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_LBUTTONUP: UINT = 0x0202;
const WM_RBUTTONDOWN: UINT = 0x0204;
const WM_KEYDOWN: UINT = 0x0100;
const WM_KEYUP: UINT = 0x0101;
const WM_CHAR: UINT = 0x0102;
const WM_MOUSEWHEEL: UINT = 0x020A;
const WM_TIMER: UINT = 0x0113;

const PM_REMOVE: UINT = 0x0001;
const SW_SHOW: i32 = 5;
const IDC_ARROW: *const u16 = 32512 as *const u16;
const COLOR_WINDOW: i32 = 5;
const BI_RGB: DWORD = 0;
const DIB_RGB_COLORS: UINT = 0;
const SRCCOPY: DWORD = 0x00CC0020;

const VK_ESCAPE: usize = 0x1B;
const VK_RETURN: usize = 0x0D;
const VK_TAB: usize = 0x09;
const VK_BACK: usize = 0x08;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;

// ── FFI declarations ─────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[allow(dead_code)]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> ATOM;
    fn CreateWindowExW(
        dwExStyle: DWORD, lpClassName: *const u16, lpWindowName: *const u16, dwStyle: DWORD,
        x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE, lpParam: *mut c_void,
    ) -> HWND;
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn PeekMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT, wRemoveMsg: UINT) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: UINT, lpTimerFunc: *const c_void) -> usize;
    fn KillTimer(hWnd: HWND, uIDEvent: usize) -> BOOL;
    fn StretchDIBits(
        hdc: HDC, xDest: i32, yDest: i32, DestWidth: i32, DestHeight: i32,
        xSrc: i32, ySrc: i32, SrcWidth: i32, SrcHeight: i32,
        lpBits: *const u8, lpbmi: *const BITMAPINFOHEADER, iUsage: UINT, rop: DWORD,
    ) -> i32;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
}

#[cfg(not(target_os = "windows"))]
mod ffi_stub {
    #![allow(unused)]
    use super::*;
    pub unsafe fn GetModuleHandleW(_: *const u16) -> HINSTANCE { std::ptr::null_mut() }
    pub unsafe fn RegisterClassExW(_: *const WNDCLASSEXW) -> ATOM { 0 }
    pub unsafe fn CreateWindowExW(_:DWORD,_:*const u16,_:*const u16,_:DWORD,_:i32,_:i32,_:i32,_:i32,_:HWND,_:HMENU,_:HINSTANCE,_:*mut c_void) -> HWND { std::ptr::null_mut() }
    pub unsafe fn ShowWindow(_: HWND, _: i32) -> BOOL { 0 }
    pub unsafe fn UpdateWindow(_: HWND) -> BOOL { 0 }
    pub unsafe fn PeekMessageW(_:*mut MSG,_:HWND,_:UINT,_:UINT,_:UINT) -> BOOL { 0 }
    pub unsafe fn TranslateMessage(_: *const MSG) -> BOOL { 0 }
    pub unsafe fn DispatchMessageW(_: *const MSG) -> LRESULT { 0 }
    pub unsafe fn DefWindowProcW(_:HWND,_:UINT,_:WPARAM,_:LPARAM) -> LRESULT { 0 }
    pub unsafe fn PostQuitMessage(_: i32) {}
    pub unsafe fn BeginPaint(_:HWND,_:*mut PAINTSTRUCT) -> HDC { std::ptr::null_mut() }
    pub unsafe fn EndPaint(_:HWND,_:*const PAINTSTRUCT) -> BOOL { 0 }
    pub unsafe fn InvalidateRect(_:HWND,_:*const RECT,_:BOOL) -> BOOL { 0 }
    pub unsafe fn GetClientRect(_:HWND,_:*mut RECT) -> BOOL { 0 }
    pub unsafe fn LoadCursorW(_:HINSTANCE,_:*const u16) -> HCURSOR { std::ptr::null_mut() }
    pub unsafe fn SetTimer(_:HWND,_:usize,_:UINT,_:*const c_void) -> usize { 0 }
    pub unsafe fn KillTimer(_:HWND,_:usize) -> BOOL { 0 }
    pub unsafe fn StretchDIBits(_:HDC,_:i32,_:i32,_:i32,_:i32,_:i32,_:i32,_:i32,_:i32,_:*const u8,_:*const BITMAPINFOHEADER,_:UINT,_:DWORD) -> i32 { 0 }
    pub unsafe fn DestroyWindow(_: HWND) -> BOOL { 0 }
}
#[cfg(not(target_os = "windows"))]
use ffi_stub::*;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn loword(l: LPARAM) -> i32 { (l & 0xFFFF) as i16 as i32 }
fn hiword(l: LPARAM) -> i32 { ((l >> 16) & 0xFFFF) as i16 as i32 }

// ── Global state for WndProc callback ────────────────────────────────────────
// Win32 WndProc is a bare function pointer; we use thread-local state.

static WINDOW_ALIVE: AtomicBool = AtomicBool::new(false);

/// Input event from the OS window, translated for killer_ui consumption.
#[derive(Debug, Clone)]
pub enum WinEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { x: i32, y: i32, button: u8 },
    MouseUp   { x: i32, y: i32, button: u8 },
    MouseWheel { x: i32, y: i32, delta: i32 },
    KeyDown { vk: u32 },
    KeyUp   { vk: u32 },
    CharInput { ch: char },
    Resize { width: u32, height: u32 },
    Close,
    Paint,
    Timer,
}

thread_local! {
    static PENDING_EVENTS: std::cell::RefCell<Vec<WinEvent>> = std::cell::RefCell::new(Vec::new());
    static WIN_SIZE: std::cell::RefCell<(u32, u32)> = std::cell::RefCell::new((800, 600));
}

fn push_event(e: WinEvent) {
    PENDING_EVENTS.with(|v| v.borrow_mut().push(e));
}

pub fn drain_events() -> Vec<WinEvent> {
    PENDING_EVENTS.with(|v| {
        let mut vec = v.borrow_mut();
        std::mem::take(&mut *vec)
    })
}

pub fn window_size() -> (u32, u32) {
    WIN_SIZE.with(|s| *s.borrow())
}

// ── WndProc ──────────────────────────────────────────────────────────────────

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            push_event(WinEvent::Close);
            WINDOW_ALIVE.store(false, Ordering::SeqCst);
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_PAINT => {
            push_event(WinEvent::Paint);
            // Still need to validate the region
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            0
        }
        WM_SIZE => {
            let w_new = loword(l) as u32;
            let h_new = hiword(l) as u32;
            if w_new > 0 && h_new > 0 {
                WIN_SIZE.with(|s| *s.borrow_mut() = (w_new, h_new));
                push_event(WinEvent::Resize { width: w_new, height: h_new });
            }
            0
        }
        WM_MOUSEMOVE => {
            push_event(WinEvent::MouseMove { x: loword(l), y: hiword(l) });
            0
        }
        WM_LBUTTONDOWN => {
            push_event(WinEvent::MouseDown { x: loword(l), y: hiword(l), button: 0 });
            0
        }
        WM_LBUTTONUP => {
            push_event(WinEvent::MouseUp { x: loword(l), y: hiword(l), button: 0 });
            0
        }
        WM_RBUTTONDOWN => {
            push_event(WinEvent::MouseDown { x: loword(l), y: hiword(l), button: 1 });
            0
        }
        WM_KEYDOWN => {
            push_event(WinEvent::KeyDown { vk: w as u32 });
            0
        }
        WM_KEYUP => {
            push_event(WinEvent::KeyUp { vk: w as u32 });
            0
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(w as u32) {
                push_event(WinEvent::CharInput { ch });
            }
            0
        }
        WM_MOUSEWHEEL => {
            let delta = hiword(w as isize);
            push_event(WinEvent::MouseWheel { x: loword(l), y: hiword(l), delta });
            0
        }
        WM_TIMER => {
            push_event(WinEvent::Timer);
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Create a Win32 window and return the HWND. Starts a 16ms timer for ~60 FPS repaints.
pub fn create_window(title: &str, width: u32, height: u32) -> Result<HWND, String> {
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("KillerUiWindowClass");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: wnd_proc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let title_wide = to_wide(title);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title_wide.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT,
            width as i32, height as i32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW failed".into());
        }
        WIN_SIZE.with(|s| *s.borrow_mut() = (width, height));
        WINDOW_ALIVE.store(true, Ordering::SeqCst);

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        // 16ms timer ≈ 60 FPS
        SetTimer(hwnd, 1, 16, std::ptr::null());

        Ok(hwnd)
    }
}

/// Pump all pending Win32 messages. Returns false if WM_QUIT received.
pub fn pump_messages() -> bool {
    unsafe {
        let mut msg = std::mem::zeroed::<MSG>();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == 0x0012 /* WM_QUIT */ {
                return false;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        WINDOW_ALIVE.load(Ordering::SeqCst)
    }
}

/// Blit a framebuffer to the window's client area via StretchDIBits.
pub fn blit_framebuffer(hwnd: HWND, fb: &Framebuffer) {
    unsafe {
        let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.is_null() {
            // Outside WM_PAINT: use GetDC/ReleaseDC pattern
            blit_with_getdc(hwnd, fb);
            EndPaint(hwnd, &ps);
            return;
        }
        blit_to_dc(hdc, fb);
        EndPaint(hwnd, &ps);
    }
}

/// Blit using GetDC (for outside WM_PAINT).
pub fn blit_with_getdc(hwnd: HWND, fb: &Framebuffer) {
    #[cfg(target_os = "windows")]
    extern "system" {
        fn GetDC(hWnd: HWND) -> HDC;
        fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    }
    #[cfg(not(target_os = "windows"))]
    unsafe fn GetDC(_: HWND) -> HDC { std::ptr::null_mut() }
    #[cfg(not(target_os = "windows"))]
    unsafe fn ReleaseDC(_: HWND, _: HDC) -> i32 { 0 }

    unsafe {
        let hdc = GetDC(hwnd);
        if !hdc.is_null() {
            blit_to_dc(hdc, fb);
            ReleaseDC(hwnd, hdc);
        }
    }
}

unsafe fn blit_to_dc(hdc: HDC, fb: &Framebuffer) {
    let bgr = fb.to_bgr_bottom_up();
    let bmi = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
        biWidth: fb.width as LONG,
        biHeight: fb.height as LONG, // positive = bottom-up
        biPlanes: 1,
        biBitCount: 24,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let (cw, ch) = window_size();
    StretchDIBits(
        hdc,
        0, 0, cw as i32, ch as i32,
        0, 0, fb.width as i32, fb.height as i32,
        bgr.as_ptr(),
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

/// Is the window still alive?
pub fn is_alive() -> bool {
    WINDOW_ALIVE.load(Ordering::SeqCst)
}

/// Map a Win32 virtual key code to a human-readable name.
pub fn vk_name(vk: u32) -> &'static str {
    match vk as usize {
        VK_ESCAPE => "Escape",
        VK_RETURN => "Enter",
        VK_TAB => "Tab",
        VK_BACK => "Backspace",
        VK_LEFT => "ArrowLeft",
        VK_UP => "ArrowUp",
        VK_RIGHT => "ArrowRight",
        VK_DOWN => "ArrowDown",
        0x20 => "Space",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_wide_null_terminated() {
        let w = to_wide("AB");
        assert_eq!(w, vec![0x41, 0x42, 0x00]);
    }

    #[test]
    fn loword_hiword() {
        let l: LPARAM = 0x0003_0005;
        assert_eq!(loword(l), 5);
        assert_eq!(hiword(l), 3);
    }
}
