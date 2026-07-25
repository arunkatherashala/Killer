// android_phone.rs — Native Android phone/call state detection for Killer Language
// Pure Rust + Android NDK/JNI — detects call start/end, provides call info
//
// Provides builtin functions:
//   phone_get_state()           → string ("idle"/"ringing"/"offhook")
//   phone_listen_calls()        → listener_id (starts monitoring)
//   phone_stop_listening(id)    → null
//   phone_get_call_info()       → dict {state, number, duration_ms, type}
//   phone_is_in_call()          → bool
//   phone_on_state_change(cb)   → listener_id (callback-based)
//
// On Android: Uses TelephonyManager via JNI bridge
// On Desktop: Simulates phone states for testing

#![allow(unsafe_code)]

use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// ── Phone call states ─────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneState {
    Idle = 0,
    Ringing = 1,
    Offhook = 2,  // In call
}

impl PhoneState {
    pub fn name(&self) -> &'static str {
        match self {
            PhoneState::Idle    => "idle",
            PhoneState::Ringing => "ringing",
            PhoneState::Offhook => "offhook",
        }
    }

    pub fn from_android_int(state: i32) -> Self {
        match state {
            1 => PhoneState::Ringing,
            2 => PhoneState::Offhook,
            _ => PhoneState::Idle,
        }
    }
}

// ── Call type enum ────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallType {
    Unknown,
    Incoming,
    Outgoing,
    Missed,
    VoIP,  // WhatsApp, Telegram, etc.
}

impl CallType {
    pub fn name(&self) -> &'static str {
        match self {
            CallType::Unknown  => "unknown",
            CallType::Incoming => "incoming",
            CallType::Outgoing => "outgoing",
            CallType::Missed   => "missed",
            CallType::VoIP     => "voip",
        }
    }
}

// ── Call info snapshot ────────────────────────────────────────
#[derive(Debug, Clone)]
struct CallInfo {
    state: PhoneState,
    call_type: CallType,
    phone_number: String,
    start_time_ms: u64,
    app_name: String,  // For VoIP calls: "whatsapp", "telegram", etc.
}

fn time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Listener state ────────────────────────────────────────────
#[allow(dead_code)]
struct PhoneListener {
    id: u64,
    running: bool,
    callback_fn: Option<String>,  // Name of Killer function to call on state change
}

struct PhoneManager {
    current_state: PhoneState,
    current_call: Option<CallInfo>,
    listeners: HashMap<u64, PhoneListener>,
    next_id: u64,
    state_history: Vec<(u64, PhoneState)>,  // (timestamp, state)
    monitoring_active: bool,
    auto_record: bool,
    voip_apps: Vec<String>,
}

static PHONE: OnceLock<Mutex<PhoneManager>> = OnceLock::new();

fn phone_mgr() -> &'static Mutex<PhoneManager> {
    PHONE.get_or_init(|| Mutex::new(PhoneManager {
        current_state: PhoneState::Idle,
        current_call: None,
        listeners: HashMap::new(),
        next_id: 1,
        state_history: Vec::new(),
        monitoring_active: false,
        auto_record: true,
        voip_apps: vec![
            "com.whatsapp".into(),
            "org.telegram.messenger".into(),
            "com.viber.voip".into(),
            "com.skype.raider".into(),
            "us.zoom.videomeetings".into(),
            "com.discord".into(),
            "com.facebook.orca".into(),
            "com.google.android.apps.meetings".into(),
        ],
    }))
}

// ══════════════════════════════════════════════════════════════
// ANDROID JNI PHONE STATE MONITORING
// ══════════════════════════════════════════════════════════════

#[cfg(target_os = "android")]
mod android_phone_jni {
    use super::*;
    use std::os::raw::c_void;

    // JNI types
    type JNIEnv = *mut c_void;
    type JavaVM = *mut c_void;
    type JObject = *mut c_void;
    type JClass = *mut c_void;
    type JMethodID = *mut c_void;
    type JString = *mut c_void;
    type JInt = i32;

    // Store the JavaVM pointer for getting JNIEnv in any thread
    static JAVA_VM: OnceLock<usize> = OnceLock::new(); // *mut JavaVM as usize

    /// Called from Java/Kotlin to initialize the JNI bridge
    /// This must be called once from the Android Activity's onCreate
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_initPhoneMonitor(
        env: JNIEnv,
        _class: JClass,
        java_vm: JavaVM,
    ) {
        let _ = JAVA_VM.set(java_vm as usize);
    }

    /// Called from Java when phone state changes
    /// This is the JNI callback invoked by PhoneStateListener
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_onPhoneStateChanged(
        _env: JNIEnv,
        _class: JClass,
        state: JInt,
        number_ptr: JString,
    ) {
        let phone_state = PhoneState::from_android_int(state);
        update_phone_state(phone_state, "");
    }

    /// Called from Java when a VoIP call is detected (via NotificationListener)
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_onVoIPCallDetected(
        _env: JNIEnv,
        _class: JClass,
        app_name_ptr: JString,
        is_active: JInt,
    ) {
        let state = if is_active != 0 { PhoneState::Offhook } else { PhoneState::Idle };
        update_phone_state(state, "");
    }
}

/// Update phone state from any source (JNI callback or polling)
fn update_phone_state(new_state: PhoneState, number: &str) {
    if let Ok(mut mgr) = phone_mgr().lock() {
        let old_state = mgr.current_state;
        mgr.current_state = new_state;
        mgr.state_history.push((time_ms(), new_state));

        // Trim history to last 100 entries
        if mgr.state_history.len() > 100 {
            mgr.state_history.drain(0..50);
        }

        match new_state {
            PhoneState::Ringing => {
                mgr.current_call = Some(CallInfo {
                    state: PhoneState::Ringing,
                    call_type: CallType::Incoming,
                    phone_number: number.to_string(),
                    start_time_ms: time_ms(),
                    app_name: String::new(),
                });
            }
            PhoneState::Offhook if old_state == PhoneState::Idle => {
                // Outgoing call (went from idle to offhook without ringing)
                mgr.current_call = Some(CallInfo {
                    state: PhoneState::Offhook,
                    call_type: CallType::Outgoing,
                    phone_number: number.to_string(),
                    start_time_ms: time_ms(),
                    app_name: String::new(),
                });
            }
            PhoneState::Offhook => {
                // Answered incoming call
                if let Some(ref mut call) = mgr.current_call {
                    call.state = PhoneState::Offhook;
                }
            }
            PhoneState::Idle if old_state == PhoneState::Ringing => {
                // Missed call
                if let Some(ref mut call) = mgr.current_call {
                    call.call_type = CallType::Missed;
                    call.state = PhoneState::Idle;
                }
            }
            PhoneState::Idle => {
                // Call ended
                mgr.current_call = None;
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// DESKTOP SIMULATION (for testing)
// ══════════════════════════════════════════════════════════════

#[cfg(not(target_os = "android"))]
mod desktop_phone {
    use super::*;

    /// Simulate a phone state change (for testing .killer scripts on desktop)
    pub fn simulate_state(state_name: &str) {
        let state = match state_name {
            "ringing" => PhoneState::Ringing,
            "offhook" | "in_call" => PhoneState::Offhook,
            _ => PhoneState::Idle,
        };
        update_phone_state(state, "+1234567890");
    }
}

// ══════════════════════════════════════════════════════════════
// BUILTIN FUNCTIONS — Called from builtin.rs
// ══════════════════════════════════════════════════════════════

/// phone_get_state() → string ("idle"/"ringing"/"offhook")
pub fn builtin_phone_get_state(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Str(mgr.current_state.name().into()))
}

/// phone_is_in_call() → bool
pub fn builtin_phone_is_in_call(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Bool(mgr.current_state == PhoneState::Offhook))
}

/// phone_listen_calls() → listener_id (Number)
pub fn builtin_phone_listen_calls(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mut mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    let id = mgr.next_id;
    mgr.next_id += 1;
    mgr.listeners.insert(id, PhoneListener {
        id,
        running: true,
        callback_fn: None,
    });
    mgr.monitoring_active = true;
    Ok(Value::Number(id as f64))
}

/// phone_stop_listening(id) → null
pub fn builtin_phone_stop_listening(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("phone_stop_listening: requires listener id"));
    }
    let id = match &args[0] {
        Value::Number(n) => *n as u64,
        _ => return Err(VmError::runtime_error("phone_stop_listening: id must be number")),
    };
    let mut mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    mgr.listeners.remove(&id);
    if mgr.listeners.is_empty() {
        mgr.monitoring_active = false;
    }
    Ok(Value::Null)
}

/// phone_get_call_info() → dict {state, type, number, duration_ms, app}
pub fn builtin_phone_get_call_info(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    let mut dict = HashMap::new();
    dict.insert("state".into(), Value::Str(mgr.current_state.name().into()));

    if let Some(ref call) = mgr.current_call {
        dict.insert("type".into(), Value::Str(call.call_type.name().into()));
        dict.insert("number".into(), Value::Str(call.phone_number.clone()));
        dict.insert("app".into(), Value::Str(call.app_name.clone()));
        let duration = if call.start_time_ms > 0 {
            time_ms() - call.start_time_ms
        } else {
            0
        };
        dict.insert("duration_ms".into(), Value::Number(duration as f64));
    } else {
        dict.insert("type".into(), Value::Str("none".to_string()));
        dict.insert("number".into(), Value::Str(String::new()));
        dict.insert("duration_ms".into(), Value::Number(0.0));
        dict.insert("app".into(), Value::Str(String::new()));
    }

    Ok(Value::Dict(Box::new(dict)))
}

/// phone_set_auto_record(bool) → null
pub fn builtin_phone_set_auto_record(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("phone_set_auto_record: requires bool"));
    }
    let enabled = match &args[0] {
        Value::Bool(b) => *b,
        _ => return Err(VmError::runtime_error("phone_set_auto_record: requires bool")),
    };
    let mut mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    mgr.auto_record = enabled;
    Ok(Value::Null)
}

/// phone_get_auto_record() → bool
pub fn builtin_phone_get_auto_record(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Bool(mgr.auto_record))
}

/// phone_simulate(state_name) → null  (desktop testing only)
pub fn builtin_phone_simulate(args: &[Value]) -> Result<Value, VmError> {
    #[cfg(not(target_os = "android"))]
    {
        if let Some(Value::Str(state)) = args.first() {
            desktop_phone::simulate_state(state);
        }
    }
    #[cfg(target_os = "android")]
    {
        let _ = args;
        // No-op on real Android — state comes from JNI callbacks
    }
    Ok(Value::Null)
}

/// phone_get_voip_apps() → array of package names
pub fn builtin_phone_get_voip_apps(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    let arr: Vec<Value> = mgr.voip_apps.iter().map(|s| Value::Str(s.clone())).collect();
    Ok(Value::Array(arr.into()))
}

/// phone_add_voip_app(package_name) → null
pub fn builtin_phone_add_voip_app(args: &[Value]) -> Result<Value, VmError> {
    if let Some(Value::Str(pkg)) = args.first() {
        let mut mgr = phone_mgr().lock().map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        if !mgr.voip_apps.contains(pkg) {
            mgr.voip_apps.push(pkg.clone());
        }
    }
    Ok(Value::Null)
}
