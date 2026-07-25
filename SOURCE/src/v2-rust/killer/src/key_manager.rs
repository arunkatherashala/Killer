/// Enhanced Encryption Module with Key Rotation - v4.3
/// Purpose: Cryptography framework with key versioning and rotation policies
/// Status: Production-ready

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Key version metadata
#[derive(Debug, Clone)]
pub struct KeyVersion {
    pub version_id: u32,
    pub created_at: SystemTime,
    pub rotated_at: Option<SystemTime>,
    pub compromised: bool,
    pub active: bool,
}

/// Key with version information
#[derive(Debug, Clone)]
pub struct VersionedKey {
    pub key_material: Vec<u8>,
    pub version: KeyVersion,
}

impl VersionedKey {
    pub fn new(key_material: Vec<u8>, version_id: u32) -> Self {
        VersionedKey {
            key_material,
            version: KeyVersion {
                version_id,
                created_at: SystemTime::now(),
                rotated_at: None,
                compromised: false,
                active: true,
            },
        }
    }
}

/// Key rotation policy
#[derive(Debug, Clone)]
pub struct KeyRotationPolicy {
    /// Rotate keys every N days (0 = disabled)
    pub rotation_interval_days: u32,
    /// Keep historical keys for N days before deletion
    pub retention_days: u32,
    /// Automatically deactivate keys after N days
    pub deactivation_days: u32,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        KeyRotationPolicy {
            rotation_interval_days: 90,      // Rotate every 90 days
            retention_days: 365,              // Keep for 1 year
            deactivation_days: 30,            // Deactivate after 30 days of non-use
        }
    }
}

/// Key manager with versioning and rotation
pub struct KeyManager {
    keys: Arc<Mutex<HashMap<String, Vec<VersionedKey>>>>,  // key_id -> versions
    policy: KeyRotationPolicy,
    rotation_history: Arc<Mutex<Vec<KeyRotationEvent>>>,
}

#[derive(Debug, Clone)]
pub struct KeyRotationEvent {
    pub key_id: String,
    pub old_version: u32,
    pub new_version: u32,
    pub timestamp: SystemTime,
    pub reason: String,
}

impl KeyManager {
    /// Create new key manager with default policy
    pub fn new() -> Self {
        KeyManager {
            keys: Arc::new(Mutex::new(HashMap::new())),
            policy: KeyRotationPolicy::default(),
            rotation_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create with custom rotation policy
    pub fn with_policy(policy: KeyRotationPolicy) -> Self {
        KeyManager {
            keys: Arc::new(Mutex::new(HashMap::new())),
            policy,
            rotation_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create or rotate a key
    pub fn create_key(&self, key_id: &str, key_material: Vec<u8>) -> Result<u32, String> {
        let mut keys = self.keys.lock().map_err(|e| e.to_string())?;

        let version_id = if let Some(versions) = keys.get(key_id) {
            versions.len() as u32
        } else {
            0
        };

        let versioned_key = VersionedKey::new(key_material, version_id);
        keys.entry(key_id.to_string())
            .or_insert_with(Vec::new)
            .push(versioned_key);

        // Record rotation history
        if version_id > 0 {
            let _ = self.rotation_history.lock().map(|mut history| {
                history.push(KeyRotationEvent {
                    key_id: key_id.to_string(),
                    old_version: version_id - 1,
                    new_version: version_id,
                    timestamp: SystemTime::now(),
                    reason: "Scheduled rotation".to_string(),
                });
            });
        }

        Ok(version_id)
    }

    /// Get the current active key
    pub fn get_active_key(&self, key_id: &str) -> Result<Option<Vec<u8>>, String> {
        let keys = self.keys.lock().map_err(|e| e.to_string())?;

        if let Some(versions) = keys.get(key_id) {
            for version in versions.iter().rev() {
                if version.version.active && !version.version.compromised {
                    return Ok(Some(version.key_material.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Get key by version
    pub fn get_key_version(
        &self,
        key_id: &str,
        version: u32,
    ) -> Result<Option<Vec<u8>>, String> {
        let keys = self.keys.lock().map_err(|e| e.to_string())?;

        if let Some(versions) = keys.get(key_id) {
            if let Some(v) = versions.get(version as usize) {
                return Ok(Some(v.key_material.clone()));
            }
        }

        Ok(None)
    }

    /// Mark a key as compromised
    pub fn mark_compromised(&self, key_id: &str, version: u32) -> Result<(), String> {
        let mut keys = self.keys.lock().map_err(|e| e.to_string())?;

        if let Some(versions) = keys.get_mut(key_id) {
            if let Some(v) = versions.get_mut(version as usize) {
                v.version.compromised = true;
                v.version.active = false;

                // Record in history
                let _ = self.rotation_history.lock().map(|mut history| {
                    history.push(KeyRotationEvent {
                        key_id: key_id.to_string(),
                        old_version: version,
                        new_version: version,
                        timestamp: SystemTime::now(),
                        reason: "Key marked as compromised".to_string(),
                    });
                });

                return Ok(());
            }
        }

        Err(format!("Key not found: {}:{}", key_id, version))
    }

    /// Check if rotation is needed
    pub fn should_rotate(&self, key_id: &str) -> Result<bool, String> {
        let keys = self.keys.lock().map_err(|e| e.to_string())?;

        if let Some(versions) = keys.get(key_id) {
            if let Some(active_key) = versions.iter().find(|v| v.version.active) {
                let age = active_key
                    .version
                    .created_at
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0));

                let rotation_interval = Duration::from_secs(
                    (self.policy.rotation_interval_days as u64) * 86400,
                );

                return Ok(age > rotation_interval);
            }
        }

        Ok(false)
    }

    /// Get rotation history
    pub fn get_rotation_history(&self) -> Result<Vec<KeyRotationEvent>, String> {
        self.rotation_history
            .lock()
            .map(|h| h.clone())
            .map_err(|e| e.to_string())
    }

    /// Clean up old keys based on retention policy
    pub fn cleanup_old_keys(&self) -> Result<usize, String> {
        let mut keys = self.keys.lock().map_err(|e| e.to_string())?;
        let mut deleted_count = 0;
        let retention_duration = Duration::from_secs((self.policy.retention_days as u64) * 86400);

        for versions in keys.values_mut() {
            versions.retain(|v| {
                if let Ok(age) = v.version.created_at.elapsed() {
                    if age > retention_duration {
                        deleted_count += 1;
                        return false;
                    }
                }
                true
            });
        }

        Ok(deleted_count)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_manager_create_and_get() {
        let manager = KeyManager::new();
        let key_material = vec![1, 2, 3, 4, 5];

        let version = manager.create_key("app-key", key_material.clone()).unwrap();
        assert_eq!(version, 0);

        let retrieved = manager.get_active_key("app-key").unwrap();
        assert_eq!(retrieved, Some(key_material));
    }

    #[test]
    fn key_manager_rotation() {
        let manager = KeyManager::new();

        let v1 = manager.create_key("app-key", vec![1, 2, 3]).unwrap();
        assert_eq!(v1, 0);

        let v2 = manager.create_key("app-key", vec![4, 5, 6]).unwrap();
        assert_eq!(v2, 1);

        // Should get latest active key
        let retrieved = manager.get_active_key("app-key").unwrap();
        assert_eq!(retrieved, Some(vec![4, 5, 6]));
    }

    #[test]
    fn key_manager_mark_compromised() {
        let manager = KeyManager::new();

        manager.create_key("app-key", vec![1, 2, 3]).unwrap();
        manager.mark_compromised("app-key", 0).unwrap();

        let retrieved = manager.get_active_key("app-key").unwrap();
        assert_eq!(retrieved, None);  // No active keys now
    }

    #[test]
    fn key_manager_rotation_history() {
        let manager = KeyManager::new();

        manager.create_key("app-key", vec![1, 2, 3]).unwrap();
        manager.create_key("app-key", vec![4, 5, 6]).unwrap();

        let history = manager.get_rotation_history().unwrap();
        assert!(history.len() >= 1);  // At least one rotation event
    }
}
