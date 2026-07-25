// Phase 4.2: Target Architecture Support
// Handles multiple target architectures (x86-64, ARM, WebAssembly, etc.)

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Wasm32,
    Riscv64,
}

#[derive(Debug, Clone)]
pub struct TargetArchSupport {
    /// Target architecture
    arch: TargetArch,
    /// Is supported by this implementation
    supported: bool,
    /// Native register width
    register_width: usize,
    /// Calling convention (e.g., "x64" for windows x64)
    calling_convention: String,
    /// Optimizations applicable to this arch
    applicable_optimizations: Vec<String>,
}

#[derive(Debug)]
pub struct TargetArchManager {
    /// Supported architectures
    architectures: HashMap<String, TargetArchSupport>,
    /// Current target architecture
    current_target: String,
}

impl TargetArchManager {
    pub fn new() -> Self {
        let mut architectures = HashMap::new();

        // X86-64 support (primary target)
        architectures.insert(
            "x86-64".to_string(),
            TargetArchSupport {
                arch: TargetArch::X86_64,
                supported: true,
                register_width: 64,
                calling_convention: "System V AMD64 ABI".to_string(),
                applicable_optimizations: vec![
                    "SIMD (SSE4.2, AVX2)".to_string(),
                    "Branch prediction".to_string(),
                    "Cache optimization".to_string(),
                    "Vectorization".to_string(),
                ],
            },
        );

        // ARM64 support
        architectures.insert(
            "arm64".to_string(),
            TargetArchSupport {
                arch: TargetArch::Aarch64,
                supported: true,
                register_width: 64,
                calling_convention: "ARM64 EABI".to_string(),
                applicable_optimizations: vec![
                    "NEON SIMD".to_string(),
                    "SVE vectorization".to_string(),
                    "Load/store optimization".to_string(),
                ],
            },
        );

        // WebAssembly support
        architectures.insert(
            "wasm32".to_string(),
            TargetArchSupport {
                arch: TargetArch::Wasm32,
                supported: true,
                register_width: 32,
                calling_convention: "WebAssembly".to_string(),
                applicable_optimizations: vec![
                    "Memory layout optimization".to_string(),
                    "Function inlining".to_string(),
                ],
            },
        );

        // RISC-V support
        architectures.insert(
            "riscv64".to_string(),
            TargetArchSupport {
                arch: TargetArch::Riscv64,
                supported: true,
                register_width: 64,
                calling_convention: "RISC-V".to_string(),
                applicable_optimizations: vec![
                    "Register utilization".to_string(),
                    "Instruction scheduling".to_string(),
                ],
            },
        );

        TargetArchManager {
            architectures,
            current_target: "x86-64".to_string(),
        }
    }

    /// Set current target architecture
    pub fn set_target(&mut self, target: String) -> bool {
        if self.architectures.contains_key(&target) {
            self.current_target = target;
            true
        } else {
            false
        }
    }

    /// Get current target architecture info
    pub fn get_current_target(&self) -> Option<TargetArchSupport> {
        self.architectures.get(&self.current_target).cloned()
    }

    /// Get supported optimizations for current target
    pub fn get_applicable_optimizations(&self) -> Vec<String> {
        self.architectures
            .get(&self.current_target)
            .map(|t| t.applicable_optimizations.clone())
            .unwrap_or_default()
    }

    /// Get register width for current target
    pub fn get_register_width(&self) -> usize {
        self.architectures
            .get(&self.current_target)
            .map(|t| t.register_width)
            .unwrap_or(64)
    }

    /// List all supported architectures
    pub fn get_supported_targets(&self) -> Vec<String> {
        self.architectures
            .iter()
            .filter(|(_, support)| support.supported)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = TargetArchManager::new();
        assert!(manager.get_current_target().is_some());
    }

    #[test]
    fn test_set_target() {
        let mut manager = TargetArchManager::new();
        assert!(manager.set_target("arm64".to_string()));
        assert_eq!(manager.current_target, "arm64");
    }

    #[test]
    fn test_register_width() {
        let mut manager = TargetArchManager::new();
        assert_eq!(manager.get_register_width(), 64); // x86-64 default

        manager.set_target("wasm32".to_string());
        assert_eq!(manager.get_register_width(), 32); // Wasm is 32-bit
    }

    #[test]
    fn test_applicable_optimizations() {
        let mut manager = TargetArchManager::new();

        let x64_opts = manager.get_applicable_optimizations();
        assert!(x64_opts.iter().any(|o| o.contains("SIMD")));

        manager.set_target("arm64".to_string());
        let arm_opts = manager.get_applicable_optimizations();
        assert!(arm_opts.iter().any(|o| o.contains("NEON")));
    }

    #[test]
    fn test_get_supported_targets() {
        let manager = TargetArchManager::new();
        let targets = manager.get_supported_targets();
        assert!(targets.contains(&"x86-64".to_string()));
        assert!(targets.contains(&"arm64".to_string()));
    }
}
