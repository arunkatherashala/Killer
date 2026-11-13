//! Native widgets: the engine behind `window(title, width, height)` in `std/gui_native.killer`.
//!
//! On Windows the widgets are real Win32 controls (BUTTON, EDIT, STATIC, LISTBOX, COMBOBOX), declared here with
//! `extern "system"`, no crate. Everywhere else, and whenever `KILLER_GUI_HEADLESS=1` is set, the same API runs on
//! an in-memory model with no screen, so tests can drive widgets with `simulate_click`. Windows must be used from
//! the thread that created them (the VM thread). Event handlers are not stored here: `gui_win_wait` returns the
//! name of the next event and the Killer side calls the handler, so handlers run as ordinary Killer code.
//! Creating a window needs the process-spawn capability, like `gui_open`.

#![allow(unsafe_code)] // the Win32 calls below

use crate::error::VmError;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::VecDeque;

pub const NAMES: &[&str] = &[
    "gui_win_new", "gui_win_add", "gui_win_get", "gui_win_set", "gui_win_items", "gui_win_wait", "gui_win_click",
    "gui_win_close", "gui_win_mode", "gui_message_box",
];

fn err<T>(m: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(m.into()))
}

fn headless() -> bool {
    match std::env::var("KILLER_GUI_HEADLESS") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => !cfg!(windows),
    }
}

struct Widget {
    name: String,
    kind: String,
    text: String,
    checked: bool,
    items: Vec<String>,
    sel: Option<usize>,
    hwnd: isize,
    id: u32,
}

struct Win {
    hwnd: isize,
    width: i32,
    widgets: Vec<Widget>,
    events: VecDeque<String>,
    closed: bool,
    shown: bool,
    next_y: i32,
    real: bool,
}

thread_local! {
    static WINS: RefCell<Vec<Win>> = const { RefCell::new(Vec::new()) };
}

fn s(v: Option<&Value>, what: &str) -> Result<String, VmError> {
    match v {
        Some(Value::Str(x)) => Ok(x.clone()),
        Some(Value::Number(n)) => Ok(if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", *n as i64) } else { format!("{}", n) }),
        Some(Value::Integer(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        Some(Value::Null) | None => Ok(String::new()),
        _ => err(format!("{} must be text", what)),
    }
}

fn num(v: Option<&Value>, what: &str) -> Result<f64, VmError> {
    match v {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::Integer(n)) => Ok(*n as f64),
        _ => err(format!("{} must be a number", what)),
    }
}

fn str_list(v: Option<&Value>) -> Result<Vec<String>, VmError> {
    match v {
        Some(Value::Array(a)) => a.clone_elements().iter().map(|x| s(Some(x), "list items")).collect(),
        Some(Value::Null) | None => Ok(vec![]),
        _ => err("items must be a list"),
    }
}

fn with_win<R>(id: f64, f: impl FnOnce(&mut Win) -> Result<R, VmError>) -> Result<R, VmError> {
    WINS.with(|w| {
        let mut w = w.borrow_mut();
        match w.get_mut(id as usize) {
            Some(win) if id >= 0.0 => f(win),
            _ => err("this window handle is not valid"),
        }
    })
}

fn find<'a>(win: &'a mut Win, name: &str) -> Result<&'a mut Widget, VmError> {
    match win.widgets.iter_mut().find(|w| w.name == name) {
        Some(w) => Ok(w),
        None => err(format!("this window has no widget named \"{}\"", name)),
    }
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !NAMES.contains(&name) {
        return None;
    }
    Some(dispatch(name, args))
}

fn is_real(id: f64) -> bool {
    WINS.with(|w| w.borrow().get(id as usize).map(|x| x.real).unwrap_or(false))
}

fn dispatch(name: &str, a: &[Value]) -> Result<Value, VmError> {
    match name {
        "gui_win_mode" => Ok(Value::Str(if headless() { "headless" } else { "win32" }.to_string())),
        "gui_message_box" => {
            crate::security::require_process_spawn()?;
            let title = s(a.first(), "title")?;
            let text = s(a.get(1), "text")?;
            let kind = s(a.get(2), "kind")?;
            if !["ok", "okcancel", "yesno"].contains(&kind.as_str()) {
                return err("message_box() kind must be \"ok\", \"okcancel\" or \"yesno\"");
            }
            if headless() {
                // nobody to ask: the affirmative answer, so a headless script does not hang
                return Ok(Value::Str(if kind == "yesno" { "yes" } else { "ok" }.to_string()));
            }
            sys::message_box(&title, &text, &kind).map(Value::Str)
        }
        "gui_win_new" => {
            crate::security::require_process_spawn()?;
            let title = s(a.first(), "title")?;
            let (w, h) = (num(a.get(1), "width")? as i32, num(a.get(2), "height")? as i32);
            if !(50..=10000).contains(&w) || !(50..=10000).contains(&h) {
                return err("window(): width and height must be between 50 and 10000");
            }
            let real = !headless();
            if real && !cfg!(windows) {
                return err("native windows need Windows; set KILLER_GUI_HEADLESS=1 to run without a screen, or use gui_show (the browser) instead");
            }
            let hwnd = if real { sys::create_window(&title, w, h)? } else { 0 };
            WINS.with(|ws| {
                let mut ws = ws.borrow_mut();
                ws.push(Win { hwnd, width: w, widgets: vec![], events: VecDeque::new(), closed: false, shown: false, next_y: 10, real });
                Ok(Value::Number((ws.len() - 1) as f64))
            })
        }
        "gui_win_add" => {
            let id = num(a.first(), "window")?;
            let (kind, wname, text) = (s(a.get(1), "kind")?, s(a.get(2), "name")?, s(a.get(3), "text")?);
            let items = str_list(a.get(4))?;
            with_win(id, |win| {
                if win.widgets.iter().any(|w| w.name == wname) {
                    return err(format!("this window already has a widget named \"{}\"", wname));
                }
                let h = match kind.as_str() {
                    "label" => 22,
                    "button" => 30,
                    "input" => 26,
                    "checkbox" => 24,
                    "list" => 120,
                    "combo" => 26,
                    _ => return err(format!("unknown widget kind \"{}\"", kind)),
                };
                let (x, y, w) = (10, win.next_y, (win.width - 40).max(20));
                let widx = win.widgets.len() as u32 + 1000;
                let mut wd = Widget { name: wname, kind, text, checked: false, items, sel: None, hwnd: 0, id: widx };
                if win.real {
                    // a combo box needs room for its drop-down list in its own height
                    let ch = if wd.kind == "combo" { 200 } else { h };
                    wd.hwnd = sys::create_control(win.hwnd, &wd.kind, &wd.text, x, y, w, ch, wd.id)?;
                    for it in &wd.items {
                        sys::add_item(wd.hwnd, &wd.kind, it);
                    }
                }
                win.next_y += h + 8;
                win.widgets.push(wd);
                Ok(Value::Null)
            })
        }
        "gui_win_get" => {
            let id = num(a.first(), "window")?;
            let wname = s(a.get(1), "name")?;
            with_win(id, |win| {
                let real = win.real && !win.closed; // a closed window answers from its last state
                let w = find(win, &wname)?;
                if real {
                    match w.kind.as_str() {
                        "input" | "label" | "button" => w.text = sys::get_text(w.hwnd),
                        "checkbox" => w.checked = sys::get_check(w.hwnd),
                        _ => w.sel = sys::get_sel(w.hwnd, &w.kind).filter(|i| *i < w.items.len()),
                    }
                }
                Ok(match w.kind.as_str() {
                    "checkbox" => Value::Bool(w.checked),
                    "list" | "combo" => match w.sel {
                        Some(i) => Value::Str(w.items[i].clone()),
                        None => Value::Null,
                    },
                    _ => Value::Str(w.text.clone()),
                })
            })
        }
        "gui_win_set" => {
            let id = num(a.first(), "window")?;
            let wname = s(a.get(1), "name")?;
            let v = a.get(2).cloned().unwrap_or(Value::Null);
            with_win(id, |win| {
                let real = win.real;
                let w = find(win, &wname)?;
                match w.kind.as_str() {
                    "checkbox" => {
                        w.checked = matches!(v, Value::Bool(true)) || matches!(v, Value::Number(n) if n != 0.0);
                        if real {
                            sys::set_check(w.hwnd, w.checked);
                        }
                    }
                    "list" | "combo" => {
                        let t = s(Some(&v), "value")?;
                        w.sel = w.items.iter().position(|x| *x == t);
                        if real {
                            sys::set_sel(w.hwnd, &w.kind, w.sel);
                        }
                    }
                    _ => {
                        w.text = s(Some(&v), "value")?;
                        if real {
                            sys::set_text(w.hwnd, &w.text);
                        }
                    }
                }
                Ok(Value::Null)
            })
        }
        "gui_win_items" => {
            let id = num(a.first(), "window")?;
            let wname = s(a.get(1), "name")?;
            let items = str_list(a.get(2))?;
            with_win(id, |win| {
                let real = win.real;
                let w = find(win, &wname)?;
                if w.kind != "list" && w.kind != "combo" {
                    return err("only a list or combo has items");
                }
                w.items = items;
                w.sel = None;
                if real {
                    sys::clear_items(w.hwnd, &w.kind);
                    for it in &w.items {
                        sys::add_item(w.hwnd, &w.kind, it);
                    }
                }
                Ok(Value::Null)
            })
        }
        "gui_win_click" => {
            // the test hook: behave as if the user clicked (a button; a checkbox toggles; a list or combo selects an item)
            let id = num(a.first(), "window")?;
            let wname = s(a.get(1), "name")?;
            let arg = a.get(2).cloned();
            with_win(id, |win| {
                let real = win.real;
                if win.closed {
                    return err("this window is closed");
                }
                let w = find(win, &wname)?;
                let mut push = false;
                match w.kind.as_str() {
                    "button" | "checkbox" => {
                        if real {
                            // a real BM_CLICK: the notification comes back through the window procedure
                            sys::click(w.hwnd);
                        } else {
                            if w.kind == "checkbox" {
                                w.checked = !w.checked;
                            }
                            push = true;
                        }
                    }
                    "list" | "combo" => {
                        let t = s(arg.as_ref(), "item")?;
                        match w.items.iter().position(|x| *x == t) {
                            Some(i) => {
                                w.sel = Some(i);
                                if real {
                                    sys::set_sel(w.hwnd, &w.kind, w.sel);
                                }
                                push = true;
                            }
                            None => return err(format!("\"{}\" has no item \"{}\"", wname, t)),
                        }
                    }
                    _ => return err("only a button, checkbox, list or combo can be clicked"),
                }
                if push {
                    win.events.push_back(wname.clone());
                }
                Ok(Value::Null)
            })?;
            if is_real(id) {
                sys::pump_once();
                route_events();
            }
            Ok(Value::Null)
        }
        "gui_win_close" => {
            let id = num(a.first(), "window")?;
            with_win(id, |win| {
                if !win.closed {
                    snapshot(win);
                    win.closed = true;
                    if win.real {
                        sys::destroy(win.hwnd);
                    }
                }
                Ok(Value::Null)
            })
        }
        "gui_win_wait" => {
            // next event name, or null when the window has been closed (or, headless, when nothing is queued)
            let id = num(a.first(), "window")?;
            let max_ms = match a.get(1) {
                Some(Value::Null) | None => 0.0,
                v => num(v, "timeout")?,
            };
            let start = std::time::Instant::now();
            loop {
                let step = with_win(id, |win| {
                    if !win.real {
                        return Ok(Some(win.events.pop_front().map(Value::Str).unwrap_or(Value::Null)));
                    }
                    if !win.shown {
                        win.shown = true;
                        if std::env::var("KILLER_GUI_HIDDEN").is_err() {
                            sys::show(win.hwnd);
                        }
                    }
                    Ok(None)
                })?;
                if let Some(v) = step {
                    return Ok(v);
                }
                sys::pump_once();
                route_events();
                let out = with_win(id, |win| {
                    if let Some(e) = win.events.pop_front() {
                        return Ok(Some(Value::Str(e)));
                    }
                    if win.closed {
                        return Ok(Some(Value::Null));
                    }
                    Ok(None)
                })?;
                if let Some(v) = out {
                    return Ok(v);
                }
                if max_ms > 0.0 && start.elapsed().as_secs_f64() * 1000.0 >= max_ms {
                    return Ok(Value::Null);
                }
                std::thread::sleep(std::time::Duration::from_millis(4));
            }
        }
        _ => err("unknown gui function"),
    }
}

/// Copy what the real controls show into the model, so the values can still be read after the window is gone.
fn snapshot(win: &mut Win) {
    if !win.real {
        return;
    }
    for w in win.widgets.iter_mut() {
        match w.kind.as_str() {
            "input" | "label" | "button" => w.text = sys::get_text(w.hwnd),
            "checkbox" => w.checked = sys::get_check(w.hwnd),
            _ => w.sel = sys::get_sel(w.hwnd, &w.kind).filter(|i| *i < w.items.len()),
        }
    }
}

/// Move the events the window procedure queued (control id + notification) to the event queue of their window.
fn route_events() {
    let raw = sys::take_events();
    if raw.is_empty() {
        return;
    }
    WINS.with(|ws| {
        let mut ws = ws.borrow_mut();
        for (hwnd, id, code) in raw {
            let Some(win) = ws.iter_mut().find(|w| w.real && w.hwnd == hwnd) else { continue };
            if id == u32::MAX {
                if !win.closed {
                    snapshot(win); // WM_CLOSE: the user closed the window; keep what the widgets showed
                    win.closed = true;
                    sys::destroy(win.hwnd);
                }
                continue;
            }
            if let Some(w) = win.widgets.iter_mut().find(|w| w.id == id) {
                // BN_CLICKED = 0 for buttons and check boxes, LBN_SELCHANGE / CBN_SELCHANGE = 1 for list and combo
                match (w.kind.as_str(), code) {
                    ("button", 0) => win.events.push_back(w.name.clone()),
                    ("checkbox", 0) => {
                        w.checked = sys::get_check(w.hwnd);
                        win.events.push_back(w.name.clone());
                    }
                    ("list", 1) | ("combo", 1) => {
                        w.sel = sys::get_sel(w.hwnd, &w.kind).filter(|i| *i < w.items.len());
                        win.events.push_back(w.name.clone());
                    }
                    _ => {}
                }
            }
        }
    });
}

#[cfg(windows)]
mod sys {
    use super::*;

    type Hwnd = isize;

    #[repr(C)]
    struct WndClassW {
        style: u32,
        wnd_proc: unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize,
        cls_extra: i32,
        wnd_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu: *const u16,
        class_name: *const u16,
    }
    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt_x: i32,
        pt_y: i32,
        private: u32,
    }
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(c: *const WndClassW) -> u16;
        fn DefWindowProcW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn CreateWindowExW(ex: u32, class: *const u16, title: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Hwnd, menu: isize, inst: isize, param: *const u8) -> Hwnd;
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn DestroyWindow(h: Hwnd) -> i32;
        fn PeekMessageW(m: *mut Msg, h: Hwnd, min: u32, max: u32, remove: u32) -> i32;
        fn TranslateMessage(m: *const Msg) -> i32;
        fn DispatchMessageW(m: *const Msg) -> isize;
        fn SendMessageW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn SetWindowTextW(h: Hwnd, t: *const u16) -> i32;
        fn GetWindowTextW(h: Hwnd, buf: *mut u16, max: i32) -> i32;
        fn GetWindowTextLengthW(h: Hwnd) -> i32;
        fn MessageBoxW(h: Hwnd, text: *const u16, title: *const u16, kind: u32) -> i32;
        fn LoadCursorW(inst: isize, name: usize) -> isize;
        fn AdjustWindowRect(r: *mut Rect, style: u32, menu: i32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn GetStockObject(i: i32) -> isize;
    }

    thread_local! {
        static EVENTS: RefCell<Vec<(isize, u32, u32)>> = const { RefCell::new(Vec::new()) };
        static REGISTERED: RefCell<bool> = const { RefCell::new(false) };
    }

    const WM_COMMAND: u32 = 0x111;
    const WM_CLOSE: u32 = 0x10;
    const WM_SETFONT: u32 = 0x30;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const WS_TABSTOP: u32 = 0x1_0000;
    const WS_BORDER: u32 = 0x80_0000;
    const WS_VSCROLL: u32 = 0x20_0000;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;

    fn w16(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe extern "system" fn wnd_proc(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
        match m {
            WM_COMMAND => {
                // low word of wparam: control id, high word: notification code
                let (id, code) = ((w & 0xFFFF) as u32, ((w >> 16) & 0xFFFF) as u32);
                EVENTS.with(|e| e.borrow_mut().push((h, id, code)));
                0
            }
            WM_CLOSE => {
                // the model destroys the window after it has copied the widget values (see route_events)
                EVENTS.with(|e| e.borrow_mut().push((h, u32::MAX, 0)));
                0
            }
            _ => DefWindowProcW(h, m, w, l),
        }
    }

    pub fn take_events() -> Vec<(isize, u32, u32)> {
        EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
    }

    pub fn create_window(title: &str, w: i32, h: i32) -> Result<isize, VmError> {
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let class = w16("KillerWindow");
            if !REGISTERED.with(|r| std::mem::replace(&mut *r.borrow_mut(), true)) {
                let c = WndClassW {
                    style: 0,
                    wnd_proc,
                    cls_extra: 0,
                    wnd_extra: 0,
                    instance: inst,
                    icon: 0,
                    cursor: LoadCursorW(0, 32512),
                    background: 6, // COLOR_WINDOW + 1
                    menu: std::ptr::null(),
                    class_name: class.as_ptr(),
                };
                RegisterClassW(&c);
            }
            let mut r = Rect { left: 0, top: 0, right: w, bottom: h };
            AdjustWindowRect(&mut r, WS_OVERLAPPEDWINDOW, 0);
            let t = w16(title);
            let hwnd = CreateWindowExW(0, class.as_ptr(), t.as_ptr(), WS_OVERLAPPEDWINDOW, 0x8000_0000u32 as i32, 0x8000_0000u32 as i32, r.right - r.left, r.bottom - r.top, 0, 0, inst, std::ptr::null());
            if hwnd == 0 {
                return err("cannot create a window (is a desktop available?)");
            }
            Ok(hwnd)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_control(parent: isize, kind: &str, text: &str, x: i32, y: i32, w: i32, h: i32, id: u32) -> Result<isize, VmError> {
        let (class, style) = match kind {
            "label" => ("STATIC", 0),
            "button" => ("BUTTON", WS_TABSTOP),
            "checkbox" => ("BUTTON", WS_TABSTOP | 3),            // BS_AUTOCHECKBOX
            "input" => ("EDIT", WS_TABSTOP | WS_BORDER | 0x80),  // ES_AUTOHSCROLL
            "list" => ("LISTBOX", WS_TABSTOP | WS_BORDER | WS_VSCROLL | 1), // LBS_NOTIFY
            _ => ("COMBOBOX", WS_TABSTOP | WS_VSCROLL | 3),      // CBS_DROPDOWNLIST
        };
        unsafe {
            let (c, t) = (w16(class), w16(text));
            let hw = CreateWindowExW(0, c.as_ptr(), t.as_ptr(), WS_CHILD | WS_VISIBLE | style, x, y, w, h, parent, id as isize, GetModuleHandleW(std::ptr::null()), std::ptr::null());
            if hw == 0 {
                return err(format!("cannot create a {}", kind));
            }
            SendMessageW(hw, WM_SETFONT, GetStockObject(17) as usize, 1); // DEFAULT_GUI_FONT
            Ok(hw)
        }
    }

    pub fn show(h: isize) {
        unsafe {
            ShowWindow(h, 5);
        }
    }
    pub fn destroy(h: isize) {
        unsafe {
            DestroyWindow(h);
        }
        pump_once();
    }
    pub fn click(h: isize) {
        unsafe {
            SendMessageW(h, 0xF5, 0, 0); // BM_CLICK
        }
    }
    pub fn pump_once() {
        unsafe {
            let mut m = std::mem::zeroed::<Msg>();
            while PeekMessageW(&mut m, 0, 0, 0, 1) != 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
    }
    pub fn set_text(h: isize, t: &str) {
        let v = w16(t);
        unsafe {
            SetWindowTextW(h, v.as_ptr());
        }
    }
    pub fn get_text(h: isize) -> String {
        unsafe {
            let n = GetWindowTextLengthW(h);
            let mut buf = vec![0u16; n as usize + 1];
            let got = GetWindowTextW(h, buf.as_mut_ptr(), n + 1);
            String::from_utf16_lossy(&buf[..got.max(0) as usize])
        }
    }
    pub fn get_check(h: isize) -> bool {
        unsafe { SendMessageW(h, 0xF0, 0, 0) == 1 }
    }
    pub fn set_check(h: isize, on: bool) {
        unsafe {
            SendMessageW(h, 0xF1, on as usize, 0);
        }
    }
    pub fn add_item(h: isize, kind: &str, t: &str) {
        let v = w16(t);
        unsafe {
            SendMessageW(h, if kind == "list" { 0x180 } else { 0x143 }, 0, v.as_ptr() as isize);
        }
    }
    pub fn clear_items(h: isize, kind: &str) {
        unsafe {
            SendMessageW(h, if kind == "list" { 0x184 } else { 0x14B }, 0, 0);
        }
    }
    pub fn get_sel(h: isize, kind: &str) -> Option<usize> {
        let r = unsafe { SendMessageW(h, if kind == "list" { 0x188 } else { 0x147 }, 0, 0) };
        if r < 0 { None } else { Some(r as usize) }
    }
    pub fn set_sel(h: isize, kind: &str, i: Option<usize>) {
        unsafe {
            SendMessageW(h, if kind == "list" { 0x186 } else { 0x14E }, i.map(|x| x as isize).unwrap_or(-1) as usize, 0);
        }
    }
    pub fn message_box(title: &str, text: &str, kind: &str) -> Result<String, VmError> {
        let (t, x) = (w16(title), w16(text));
        let style = match kind {
            "okcancel" => 1,
            "yesno" => 4,
            _ => 0,
        };
        let r = unsafe { MessageBoxW(0, x.as_ptr(), t.as_ptr(), style | 0x40) };
        Ok(match r {
            6 => "yes",
            7 => "no",
            2 => "cancel",
            _ => "ok",
        }
        .to_string())
    }
}

#[cfg(not(windows))]
mod sys {
    use super::*;
    fn no<T>() -> Result<T, VmError> {
        err("native windows need Windows; set KILLER_GUI_HEADLESS=1 to run without a screen, or use gui_show (the browser) instead")
    }
    pub fn take_events() -> Vec<(isize, u32, u32)> {
        vec![]
    }
    pub fn create_window(_: &str, _: i32, _: i32) -> Result<isize, VmError> {
        no()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create_control(_: isize, _: &str, _: &str, _: i32, _: i32, _: i32, _: i32, _: u32) -> Result<isize, VmError> {
        no()
    }
    pub fn show(_: isize) {}
    pub fn destroy(_: isize) {}
    pub fn click(_: isize) {}
    pub fn pump_once() {}
    pub fn set_text(_: isize, _: &str) {}
    pub fn get_text(_: isize) -> String {
        String::new()
    }
    pub fn get_check(_: isize) -> bool {
        false
    }
    pub fn set_check(_: isize, _: bool) {}
    pub fn add_item(_: isize, _: &str, _: &str) {}
    pub fn clear_items(_: isize, _: &str) {}
    pub fn get_sel(_: isize, _: &str) -> Option<usize> {
        None
    }
    pub fn set_sel(_: isize, _: &str, _: Option<usize>) {}
    pub fn message_box(_: &str, _: &str, _: &str) -> Result<String, VmError> {
        no()
    }
}
