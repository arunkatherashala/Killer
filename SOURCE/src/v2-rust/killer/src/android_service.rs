// android_service.rs — Native Android foreground service, permissions & notifications
// Pure Rust + Android NDK/JNI — manages background recording lifecycle
//
// Provides builtin functions:
//   service_start(title, text)     → bool (start foreground service)
//   service_stop()                 → bool (stop foreground service)
//   service_is_running()           → bool
//   permission_check(name)         → bool (check single permission)
//   permission_check_all(names)    → dict {name: bool}
//   permission_request(name)       → bool (result after request)
//   permission_request_all(names)  → dict {name: bool}
//   notification_show(title, text) → id (number)
//   notification_cancel(id)        → bool
//   device_info()                  → dict {model, sdk, brand, ...}
//   storage_path()                 → string (app's internal storage)
//   storage_external_path()        → string (external storage)
//   vibrate(ms)                    → null
//   battery_level()                → number (0-100)
//   screen_on()                    → bool
//
// On Android: Direct JNI calls to Android framework
// On Desktop: Simulated for testing

#![allow(unsafe_code)]

use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// ── Permission names ──────────────────────────────────────────
pub const PERM_RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";
pub const PERM_READ_PHONE_STATE: &str = "android.permission.READ_PHONE_STATE";
pub const PERM_READ_CALL_LOG: &str = "android.permission.READ_CALL_LOG";
pub const PERM_WRITE_EXTERNAL: &str = "android.permission.WRITE_EXTERNAL_STORAGE";
pub const PERM_READ_EXTERNAL: &str = "android.permission.READ_EXTERNAL_STORAGE";
pub const PERM_FOREGROUND_SERVICE: &str = "android.permission.FOREGROUND_SERVICE";
pub const PERM_POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";
pub const PERM_CAPTURE_AUDIO_OUTPUT: &str = "android.permission.CAPTURE_AUDIO_OUTPUT";

fn time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Service state ─────────────────────────────────────────────
#[allow(dead_code)]
struct ServiceState {
    foreground_running: bool,
    service_start_time: u64,
    notification_id: u32,
    next_notif_id: u32,
    permissions_granted: HashMap<String, bool>,
    storage_path: String,
    external_path: String,
}

static SERVICE: OnceLock<Mutex<ServiceState>> = OnceLock::new();

fn service_state() -> &'static Mutex<ServiceState> {
    SERVICE.get_or_init(|| Mutex::new(ServiceState {
        foreground_running: false,
        service_start_time: 0,
        notification_id: 1001,
        next_notif_id: 2000,
        permissions_granted: HashMap::new(),
        storage_path: String::new(),
        external_path: String::new(),
    }))
}

// ══════════════════════════════════════════════════════════════
// ANDROID JNI SERVICE BRIDGE
// ══════════════════════════════════════════════════════════════

#[cfg(target_os = "android")]
mod android_service_jni {
    use super::*;
    use std::os::raw::c_void;

    type JNIEnv = *mut c_void;
    type JObject = *mut c_void;
    type JClass = *mut c_void;
    type JString = *mut c_void;
    type JInt = i32;
    type JBoolean = u8;

    // Store JNI env for callbacks
    static JNI_ACTIVITY: OnceLock<usize> = OnceLock::new();

    /// Initialize the service bridge with the Activity reference
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_initServiceBridge(
        _env: JNIEnv,
        _class: JClass,
        activity: JObject,
        internal_path: JString,
        external_path: JString,
    ) {
        let _ = JNI_ACTIVITY.set(activity as usize);
        // In real impl, convert JString to Rust &str and store paths
    }

    /// Called from Java when permission result arrives
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_onPermissionResult(
        _env: JNIEnv,
        _class: JClass,
        permission_ptr: JString,
        granted: JBoolean,
    ) {
        // Store permission result
        // In real impl, convert JString and update permissions_granted
    }

    /// Called from Java when foreground service starts/stops
    #[no_mangle]
    pub extern "C" fn Java_com_killerlang_runtime_KillerBridge_onServiceStateChanged(
        _env: JNIEnv,
        _class: JClass,
        running: JBoolean,
    ) {
        if let Ok(mut state) = service_state().lock() {
            state.foreground_running = running != 0;
            if running != 0 {
                state.service_start_time = time_ms();
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// BUILTIN FUNCTIONS
// ══════════════════════════════════════════════════════════════

/// service_start(title, text) → bool
pub fn builtin_service_start(args: &[Value]) -> Result<Value, VmError> {
    let title = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => "Killer Call Recorder".into(),
    };
    let text = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => "Recording in progress...".into(),
    };

    let mut state = service_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    #[cfg(target_os = "android")]
    {
        // TODO: JNI call to start ForegroundService
        // For now, mark as running — the Java bridge will actually start it
        state.foreground_running = true;
        state.service_start_time = time_ms();
    }

    #[cfg(not(target_os = "android"))]
    {
        println!("[Killer Service] START: {} - {}", title, text);
        state.foreground_running = true;
        state.service_start_time = time_ms();
    }

    Ok(Value::Bool(true))
}

/// service_stop() → bool
pub fn builtin_service_stop(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mut state = service_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    #[cfg(not(target_os = "android"))]
    {
        let duration = time_ms() - state.service_start_time;
        println!("[Killer Service] STOP (ran {}ms)", duration);
    }

    state.foreground_running = false;
    Ok(Value::Bool(true))
}

/// service_is_running() → bool
pub fn builtin_service_is_running(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let state = service_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    Ok(Value::Bool(state.foreground_running))
}

/// permission_check(name) → bool
pub fn builtin_permission_check(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("permission_check: requires permission name"));
    }
    let perm_name = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("permission_check: name must be string")),
    };

    #[cfg(not(target_os = "android"))]
    {
        let _ = &perm_name;
        return Ok(Value::Bool(true));
    }

    #[cfg(target_os = "android")]
    {
        let state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        let granted = state.permissions_granted.get(&perm_name).copied().unwrap_or(false);
        Ok(Value::Bool(granted))
    }
}

/// permission_check_all(names_array) → dict {name: bool}
pub fn builtin_permission_check_all(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("permission_check_all: requires array of names"));
    }

    let names = match &args[0] {
        Value::Array(arr) => {
            arr.iter().filter_map(|v| {
                if let Value::Str(s) = v { Some(s.clone()) } else { None }
            }).collect::<Vec<_>>()
        }
        _ => return Err(VmError::runtime_error("permission_check_all: requires array")),
    };

    let mut result = HashMap::new();

    #[cfg(target_os = "android")]
    let state = service_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;

    for name in names {
        let granted = {
            #[cfg(not(target_os = "android"))]
            {
                true
            }
            #[cfg(target_os = "android")]
            {
                state.permissions_granted.get(&name).copied().unwrap_or(false)
            }
        };

        result.insert(name, Value::Bool(granted));
    }

    Ok(Value::Dict(Box::new(result)))
}

/// permission_request(name) → bool
pub fn builtin_permission_request(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("permission_request: requires permission name"));
    }
    let perm_name = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(VmError::runtime_error("permission_request: name must be string")),
    };

    #[cfg(not(target_os = "android"))]
    {
        println!("[Killer Permission] Requesting: {}", perm_name);
        // Desktop: always grant
        let mut state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        state.permissions_granted.insert(perm_name, true);
        return Ok(Value::Bool(true));
    }

    #[cfg(target_os = "android")]
    {
        // JNI call to requestPermissions() — async, result comes via callback
        // For synchronous API, we block briefly then check result
        let mut state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        let granted = state.permissions_granted.get(&perm_name).copied().unwrap_or(false);
        Ok(Value::Bool(granted))
    }
}

/// permission_request_all(names_array) → dict {name: bool}
pub fn builtin_permission_request_all(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::runtime_error("permission_request_all: requires array"));
    }

    let names = match &args[0] {
        Value::Array(arr) => {
            arr.iter().filter_map(|v| {
                if let Value::Str(s) = v { Some(s.clone()) } else { None }
            }).collect::<Vec<_>>()
        }
        _ => return Err(VmError::runtime_error("permission_request_all: requires array")),
    };

    let mut result = HashMap::new();

    #[cfg(not(target_os = "android"))]
    {
        let mut state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        for name in &names {
            println!("[Killer Permission] Requesting: {}", name);
            state.permissions_granted.insert(name.clone(), true);
            result.insert(name.clone(), Value::Bool(true));
        }
    }

    #[cfg(target_os = "android")]
    {
        let state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        for name in &names {
            let granted = state.permissions_granted.get(name).copied().unwrap_or(false);
            result.insert(name.clone(), Value::Bool(granted));
        }
    }

    Ok(Value::Dict(Box::new(result)))
}

/// notification_show(title, text) → id (Number)
pub fn builtin_notification_show(args: &[Value]) -> Result<Value, VmError> {
    let title = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => "Killer".into(),
    };
    let text = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => "".into(),
    };

    let mut state = service_state().lock()
        .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
    let id = state.next_notif_id;
    state.next_notif_id += 1;

    #[cfg(not(target_os = "android"))]
    println!("[Killer Notification #{}] {} - {}", id, title, text);

    Ok(Value::Number(id as f64))
}

/// notification_cancel(id) → bool
pub fn builtin_notification_cancel(args: &[Value]) -> Result<Value, VmError> {
    if let Some(Value::Number(id)) = args.first() {
        #[cfg(not(target_os = "android"))]
        println!("[Killer Notification #{}] CANCELLED", *id as u32);
        Ok(Value::Bool(true))
    } else {
        Ok(Value::Bool(false))
    }
}

/// device_info() → dict {model, sdk, brand, manufacturer, product, cpu_abi}
pub fn builtin_device_info(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    let mut dict = HashMap::new();

    #[cfg(target_os = "android")]
    {
        // These would be read from android.os.Build via JNI
        dict.insert("platform".into(), Value::Str("android".to_string()));
        dict.insert("runtime".into(), Value::Str("killer-native".to_string()));
    }

    #[cfg(not(target_os = "android"))]
    {
        dict.insert("platform".into(), Value::Str(std::env::consts::OS.into()));
        dict.insert("arch".into(), Value::Str(std::env::consts::ARCH.into()));
        dict.insert("runtime".into(), Value::Str("killer-native".to_string()));
        dict.insert("model".into(), Value::Str("Desktop".to_string()));
        dict.insert("sdk".into(), Value::Number(0.0));
        dict.insert("brand".into(), Value::Str("Killer".to_string()));
    }

    Ok(Value::Dict(Box::new(dict)))
}

/// storage_path() → string (app internal storage directory)
pub fn builtin_storage_path(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;

    #[cfg(target_os = "android")]
    {
        let state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        if !state.storage_path.is_empty() {
            return Ok(Value::Str(state.storage_path.clone()));
        }
        // Default Android internal path
        Ok(Value::Str("/data/data/com.killerlang.callrecorder/files".to_string()))
    }

    #[cfg(not(target_os = "android"))]
    {
        // Desktop: use current directory
        let path = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        Ok(Value::Str(path))
    }
}

/// storage_external_path() → string (external/shared storage)
pub fn builtin_storage_external_path(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;

    #[cfg(target_os = "android")]
    {
        let state = service_state().lock()
            .map_err(|e| VmError::runtime_error(format!("lock: {}", e)))?;
        if !state.external_path.is_empty() {
            return Ok(Value::Str(state.external_path.clone()));
        }
        Ok(Value::Str("/storage/emulated/0/KillerRecorder".to_string()))
    }

    #[cfg(not(target_os = "android"))]
    {
        let path = std::env::current_dir()
            .map(|p| p.join("recordings").to_string_lossy().to_string())
            .unwrap_or_else(|_| "recordings".to_string());
        Ok(Value::Str(path))
    }
}

/// vibrate(ms) → null
pub fn builtin_vibrate(args: &[Value]) -> Result<Value, VmError> {
    let ms = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => 100,
    };

    #[cfg(not(target_os = "android"))]
    println!("[Killer Vibrate] {}ms", ms);

    // On Android: JNI call to Vibrator service
    let _ = ms;
    Ok(Value::Null)
}

/// battery_level() → number (0-100)
pub fn builtin_battery_level(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;

    #[cfg(not(target_os = "android"))]
    return Ok(Value::Number(100.0)); // Desktop: always full

    #[cfg(target_os = "android")]
    {
        // JNI call to BatteryManager
        Ok(Value::Number(100.0))
    }
}

/// screen_on() → bool
pub fn builtin_screen_on(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    Ok(Value::Bool(true))
}
