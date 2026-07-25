// Phase 11: Plugin Architecture - loaders, hooks, lifecycle, capabilities
// Features: Dynamic loading, plugin discovery, lifecycle management, capability system

use std::collections::HashMap;
use crate::value::Value;

/// Plugin hook types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HookType {
    PreCompile,
    PostCompile,
    PreExecution,
    PostExecution,
    PreFileLoad,
    PostFileLoad,
    Error,
    Custom(String),
}

impl HookType {
    pub fn as_str(&self) -> &str {
        match self {
            HookType::PreCompile => "pre:compile",
            HookType::PostCompile => "post:compile",
            HookType::PreExecution => "pre:execution",
            HookType::PostExecution => "post:execution",
            HookType::PreFileLoad => "pre:fileload",
            HookType::PostFileLoad => "post:fileload",
            HookType::Error => "error",
            HookType::Custom(name) => name,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pre:compile" => HookType::PreCompile,
            "post:compile" => HookType::PostCompile,
            "pre:execution" => HookType::PreExecution,
            "post:execution" => HookType::PostExecution,
            "pre:fileload" => HookType::PreFileLoad,
            "post:fileload" => HookType::PostFileLoad,
            "error" => HookType::Error,
            custom => HookType::Custom(custom.to_string()),
        }
    }
}

/// Plugin capabilities
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    ReadFiles,
    WriteFiles,
    NetworkAccess,
    SystemAccess,
    CompilerAccess,
    RuntimeAccess,
    Custom(String),
}

impl Capability {
    pub fn as_str(&self) -> &str {
        match self {
            Capability::ReadFiles => "fs:read",
            Capability::WriteFiles => "fs:write",
            Capability::NetworkAccess => "network",
            Capability::SystemAccess => "system",
            Capability::CompilerAccess => "compiler",
            Capability::RuntimeAccess => "runtime",
            Capability::Custom(name) => name,
        }
    }
}

/// Plugin metadata
#[derive(Clone, Debug)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry_point: String,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<String>,
    pub hooks: Vec<HookType>,
}

impl PluginMetadata {
    pub fn new(id: String, name: String) -> Self {
        PluginMetadata {
            id,
            name,
            version: "0.1.0".to_string(),
            description: String::new(),
            author: String::new(),
            entry_point: String::new(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            hooks: Vec::new(),
        }
    }

    /// Set version
    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set author
    pub fn with_author(mut self, author: String) -> Self {
        self.author = author;
        self
    }

    /// Set entry point
    pub fn with_entry_point(mut self, ep: String) -> Self {
        self.entry_point = ep;
        self
    }

    /// Add capability
    pub fn add_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add dependency
    pub fn add_dependency(mut self, dep: String) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add hook
    pub fn add_hook(mut self, hook: HookType) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Check if has capability
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

/// Hook handler
pub type HookHandler = Box<dyn Fn(&HookContext) -> Result<HookResult, String> + Send + Sync>;

/// Hook context
#[derive(Clone, Debug)]
pub struct HookContext {
    pub hook_type: HookType,
    pub plugin_id: String,
    pub data: HashMap<String, String>,
}

impl HookContext {
    pub fn new(hook_type: HookType, plugin_id: String) -> Self {
        HookContext {
            hook_type,
            plugin_id,
            data: HashMap::new(),
        }
    }

    /// Add context data
    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.data.insert(key, value);
        self
    }

    /// Get data
    pub fn get_data(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }
}

/// Hook result
#[derive(Clone, Debug)]
pub struct HookResult {
    pub success: bool,
    pub message: String,
    pub output: HashMap<String, String>,
}

impl HookResult {
    pub fn success() -> Self {
        HookResult {
            success: true,
            message: String::new(),
            output: HashMap::new(),
        }
    }

    pub fn error(msg: String) -> Self {
        HookResult {
            success: false,
            message: msg,
            output: HashMap::new(),
        }
    }

    /// Add output data
    pub fn with_output(mut self, key: String, value: String) -> Self {
        self.output.insert(key, value);
        self
    }
}

/// Plugin trait
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn initialize(&mut self) -> Result<(), String>;
    fn shutdown(&mut self) -> Result<(), String>;
    fn execute_hook(&self, _context: &HookContext) -> Result<HookResult, String> {
        Ok(HookResult::success())
    }
}

/// Plugin state
#[derive(Clone, Debug, PartialEq)]
pub enum PluginState {
    Discovered,
    Loaded,
    Initialized,
    Running,
    Paused,
    Stopped,
    Failed(String),
}

impl PluginState {
    pub fn as_str(&self) -> &str {
        match self {
            PluginState::Discovered => "discovered",
            PluginState::Loaded => "loaded",
            PluginState::Initialized => "initialized",
            PluginState::Running => "running",
            PluginState::Paused => "paused",
            PluginState::Stopped => "stopped",
            PluginState::Failed(_) => "failed",
        }
    }
}

/// Plugin descriptor
#[derive(Clone, Debug)]
pub struct PluginDescriptor {
    pub metadata: PluginMetadata,
    pub state: PluginState,
    pub load_time: u64,
    pub initialization_time: u64,
    pub hook_count: usize,
}

impl PluginDescriptor {
    pub fn new(metadata: PluginMetadata) -> Self {
        PluginDescriptor {
            metadata,
            state: PluginState::Discovered,
            load_time: 0,
            initialization_time: 0,
            hook_count: 0,
        }
    }

    /// Update state
    pub fn set_state(mut self, state: PluginState) -> Self {
        self.state = state;
        self
    }

    /// Set load time
    pub fn with_load_time(mut self, time: u64) -> Self {
        self.load_time = time;
        self
    }

    /// Set initialization time
    pub fn with_init_time(mut self, time: u64) -> Self {
        self.initialization_time = time;
        self
    }
}

/// Plugin manager
pub struct PluginManager {
    pub plugins: HashMap<String, PluginDescriptor>,
    pub hooks: HashMap<HookType, Vec<String>>, // hook_type -> plugin_ids
    pub enabled: HashMap<String, bool>,
    pub load_path: String,
}

impl PluginManager {
    pub fn new(load_path: String) -> Self {
        PluginManager {
            plugins: HashMap::new(),
            hooks: HashMap::new(),
            enabled: HashMap::new(),
            load_path,
        }
    }

    /// Register plugin
    pub fn register(&mut self, plugin_id: String, descriptor: PluginDescriptor) -> Result<(), String> {
        if self.plugins.contains_key(&plugin_id) {
            return Err(format!("Plugin {} already registered", plugin_id));
        }

        // Register hooks
        for hook in &descriptor.metadata.hooks {
            self.hooks.entry(hook.clone())
                .or_insert_with(Vec::new)
                .push(plugin_id.clone());
        }

        self.plugins.insert(plugin_id.clone(), descriptor);
        self.enabled.insert(plugin_id, true);

        Ok(())
    }

    /// Unregister plugin
    pub fn unregister(&mut self, plugin_id: &str) -> Result<(), String> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(format!("Plugin {} not found", plugin_id));
        }

        // Remove hook registrations
        for hook_list in self.hooks.values_mut() {
            hook_list.retain(|id| id != plugin_id);
        }

        self.plugins.remove(plugin_id);
        self.enabled.remove(plugin_id);

        Ok(())
    }

    /// Enable plugin
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), String> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(format!("Plugin {} not found", plugin_id));
        }
        self.enabled.insert(plugin_id.to_string(), true);
        Ok(())
    }

    /// Disable plugin
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), String> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(format!("Plugin {} not found", plugin_id));
        }
        self.enabled.insert(plugin_id.to_string(), false);
        Ok(())
    }

    /// Check if enabled
    pub fn is_enabled(&self, plugin_id: &str) -> bool {
        self.enabled.get(plugin_id).copied().unwrap_or(false)
    }

    /// Get plugin descriptor
    pub fn get_plugin(&self, plugin_id: &str) -> Option<PluginDescriptor> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Get plugins for hook
    pub fn get_hooks_for(&self, hook_type: &HookType) -> Vec<String> {
        self.hooks.get(hook_type)
            .map(|v| v.iter()
                .filter(|id| self.is_enabled(id))
                .cloned()
                .collect())
            .unwrap_or_default()
    }

    /// List all plugins
    pub fn list_all(&self) -> Vec<PluginDescriptor> {
        self.plugins.values().cloned().collect()
    }

    /// List enabled plugins
    pub fn list_enabled(&self) -> Vec<PluginDescriptor> {
        self.plugins.iter()
            .filter(|(id, _)| self.is_enabled(id))
            .map(|(_, desc)| desc.clone())
            .collect()
    }

    /// Plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Enabled plugin count
    pub fn enabled_count(&self) -> usize {
        self.enabled.values().filter(|&&v| v).count()
    }

    /// Get plugin by id
    pub fn get_by_id(&self, plugin_id: &str) -> Option<PluginDescriptor> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Verify dependencies
    pub fn verify_dependencies(&self, plugin_id: &str) -> Result<(), String> {
        let plugin = self.plugins.get(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        for dep in &plugin.metadata.dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(format!("Dependency {} not found", dep));
            }
        }

        Ok(())
    }

    /// Verify capability
    pub fn verify_capability(&self, plugin_id: &str, cap: &Capability) -> bool {
        self.plugins.get(plugin_id)
            .map(|p| p.metadata.has_capability(cap))
            .unwrap_or(false)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new(".".to_string())
    }
}

/// Plugin lifecycle manager
pub struct PluginLifecycleManager {
    pub plugin_manager: PluginManager,
}

impl PluginLifecycleManager {
    pub fn new(plugin_manager: PluginManager) -> Self {
        PluginLifecycleManager { plugin_manager }
    }

    /// Load plugin
    pub fn load(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        plugin.state = PluginState::Loaded;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }

    /// Initialize plugin
    pub fn initialize(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        // Verify dependencies first
        self.plugin_manager.verify_dependencies(plugin_id)?;

        plugin.state = PluginState::Initialized;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }

    /// Start plugin
    pub fn start(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        if plugin.state != PluginState::Initialized {
            return Err("Plugin not initialized".to_string());
        }

        plugin.state = PluginState::Running;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }

    /// Stop plugin
    pub fn stop(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        plugin.state = PluginState::Stopped;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }

    /// Pause plugin
    pub fn pause(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        plugin.state = PluginState::Paused;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }

    /// Resume plugin
    pub fn resume(&mut self, plugin_id: &str) -> Result<(), String> {
        let mut plugin = self.plugin_manager.get_plugin(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        plugin.state = PluginState::Running;
        self.plugin_manager.plugins.insert(plugin_id.to_string(), plugin);
        Ok(())
    }
}

/// Plugin factory
pub struct PluginFactory;

impl PluginFactory {
    /// Create new metadata
    pub fn create_metadata(id: String, name: String) -> PluginMetadata {
        PluginMetadata::new(id, name)
    }

    /// Create new descriptor
    pub fn create_descriptor(metadata: PluginMetadata) -> PluginDescriptor {
        PluginDescriptor::new(metadata)
    }

    /// Create new hook context
    pub fn create_hook_context(hook_type: HookType, plugin_id: String) -> HookContext {
        HookContext::new(hook_type, plugin_id)
    }

    /// Create new hook result
    pub fn create_hook_result() -> HookResult {
        HookResult::success()
    }

    /// Create new manager
    pub fn create_manager(load_path: String) -> PluginManager {
        PluginManager::new(load_path)
    }

    /// Create new lifecycle manager
    pub fn create_lifecycle_manager(manager: PluginManager) -> PluginLifecycleManager {
        PluginLifecycleManager::new(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_type_creation() {
        let hook = HookType::PreCompile;
        assert_eq!(hook.as_str(), "pre:compile");
    }

    #[test]
    fn test_hook_type_from_str() {
        let hook = HookType::from_str("post:compile");
        assert_eq!(hook, HookType::PostCompile);
    }

    #[test]
    fn test_capability_creation() {
        let cap = Capability::ReadFiles;
        assert_eq!(cap.as_str(), "fs:read");
    }

    #[test]
    fn test_plugin_metadata_creation() {
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        assert_eq!(metadata.id, "test-plugin");
    }

    #[test]
    fn test_plugin_metadata_with_properties() {
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string())
            .with_version("1.0.0".to_string())
            .with_author("Author".to_string());
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, "Author");
    }

    #[test]
    fn test_plugin_metadata_with_capability() {
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string())
            .add_capability(Capability::ReadFiles);
        assert!(metadata.has_capability(&Capability::ReadFiles));
    }

    #[test]
    fn test_hook_context_creation() {
        let context = HookContext::new(HookType::PreCompile, "plugin1".to_string());
        assert_eq!(context.plugin_id, "plugin1");
    }

    #[test]
    fn test_hook_context_with_data() {
        let context = HookContext::new(HookType::PreCompile, "plugin1".to_string())
            .with_data("key".to_string(), "value".to_string());
        assert_eq!(context.get_data("key"), Some("value".to_string()));
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult::success();
        assert!(result.success);
    }

    #[test]
    fn test_hook_result_error() {
        let result = HookResult::error("Error message".to_string());
        assert!(!result.success);
    }

    #[test]
    fn test_hook_result_with_output() {
        let result = HookResult::success()
            .with_output("key".to_string(), "value".to_string());
        assert_eq!(result.output.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_plugin_descriptor_creation() {
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata);
        assert_eq!(descriptor.state, PluginState::Discovered);
    }

    #[test]
    fn test_plugin_descriptor_with_load_time() {
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata)
            .with_load_time(100);
        assert_eq!(descriptor.load_time, 100);
    }

    #[test]
    fn test_plugin_manager_register() {
        let mut manager = PluginManager::new(".".to_string());
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata);
        
        assert!(manager.register("test-plugin".to_string(), descriptor).is_ok());
        assert_eq!(manager.plugin_count(), 1);
    }

    #[test]
    fn test_plugin_manager_unregister() {
        let mut manager = PluginManager::new(".".to_string());
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata);
        
        manager.register("test-plugin".to_string(), descriptor).unwrap();
        assert!(manager.unregister("test-plugin").is_ok());
        assert_eq!(manager.plugin_count(), 0);
    }

    #[test]
    fn test_plugin_manager_enable_disable() {
        let mut manager = PluginManager::new(".".to_string());
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata);
        
        manager.register("test-plugin".to_string(), descriptor).unwrap();
        assert!(manager.is_enabled("test-plugin"));
        
        manager.disable("test-plugin").unwrap();
        assert!(!manager.is_enabled("test-plugin"));
    }

    #[test]
    fn test_plugin_manager_list_enabled() {
        let mut manager = PluginManager::new(".".to_string());
        let metadata1 = PluginMetadata::new("plugin1".to_string(), "Plugin 1".to_string());
        let metadata2 = PluginMetadata::new("plugin2".to_string(), "Plugin 2".to_string());
        
        manager.register("plugin1".to_string(), PluginDescriptor::new(metadata1)).unwrap();
        manager.register("plugin2".to_string(), PluginDescriptor::new(metadata2)).unwrap();
        manager.disable("plugin2").unwrap();
        
        assert_eq!(manager.enabled_count(), 1);
    }

    #[test]
    fn test_plugin_state_as_str() {
        assert_eq!(PluginState::Loaded.as_str(), "loaded");
        assert_eq!(PluginState::Running.as_str(), "running");
    }

    #[test]
    fn test_plugin_lifecycle_manager_load() {
        let manager = PluginManager::new(".".to_string());
        let mut lifecycle_manager = PluginLifecycleManager::new(manager);
        
        let metadata = PluginMetadata::new("test-plugin".to_string(), "Test Plugin".to_string());
        let descriptor = PluginDescriptor::new(metadata);
        lifecycle_manager.plugin_manager.register("test-plugin".to_string(), descriptor).unwrap();
        
        assert!(lifecycle_manager.load("test-plugin").is_ok());
    }

    #[test]
    fn test_plugin_factory_create_metadata() {
        let metadata = PluginFactory::create_metadata("test".to_string(), "Test".to_string());
        assert_eq!(metadata.id, "test");
    }

    #[test]
    fn test_plugin_factory_create_manager() {
        let manager = PluginFactory::create_manager(".".to_string());
        assert_eq!(manager.plugin_count(), 0);
    }
}
