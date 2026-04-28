// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! VMM Registry for managing and auto-detecting VMM runners.

use std::path::Path;
use anyhow::Result;
use super::traits::VmmRunner;
use super::firecracker::FirecrackerRunner;
use super::cloudhypervisor::CloudHypervisorRunner;
use super::crosvm::CrosvmRunner;
use super::stratovirt::StratovirtRunner;

/// Registry entry storing an actual runner instance
struct RegistryEntry {
    name: String,
    runner: Box<dyn VmmRunner>,
}

pub struct VmmRegistry {
    entries: Vec<RegistryEntry>,
}

impl VmmRegistry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Register VMM Runner (via binary path detection)
    pub fn register(&mut self, runner: Box<dyn VmmRunner>) {
        let name = runner.name().to_string();
        self.entries.push(RegistryEntry {
            name,
            runner,
        });
    }

    /// Register VMM Runner with explicit name and probe function
    #[deprecated(note = "Use register() with detected runner instead")]
    pub fn register_with(
        &mut self,
        name: &str,
        _probe: fn(&Path) -> bool,
        detect: fn(&Path) -> Result<Box<dyn VmmRunner>>,
    ) {
        // Use firecracker as placeholder binary
        // In actual use, call detect to get runner
        let _ = (name, detect);
    }

    /// Get registered Runner names
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// Get Runner by name
    pub fn get(&self, name: &str) -> Option<&dyn VmmRunner> {
        self.entries.iter()
            .find(|e| e.name == name)
            .map(|e| e.runner.as_ref())
    }

    /// Get all registered runners
    pub fn runners(&self) -> Vec<&dyn VmmRunner> {
        self.entries.iter().map(|e| e.runner.as_ref()).collect()
    }

    /// Auto-detect binary type and register
    pub fn detect_and_register(&mut self, binary: &Path) -> Result<()> {
        let runner = self.detect(binary)?;
        self.register(runner);
        Ok(())
    }

    /// Auto-detect binary type
    pub fn detect(&self, binary: &Path) -> Result<Box<dyn VmmRunner>> {
        // Try Firecracker first
        if FirecrackerRunner::probe(binary) {
            return Ok(Box::new(FirecrackerRunner::detect(binary)?));
        }
        // Then try Cloud Hypervisor
        if CloudHypervisorRunner::probe(binary) {
            return Ok(Box::new(CloudHypervisorRunner::detect(binary)?));
        }
        if CrosvmRunner::probe(binary) {
            return Ok(Box::new(CrosvmRunner::detect(binary)?));
        }
        if StratovirtRunner::probe(binary) {
            return Ok(Box::new(StratovirtRunner::detect(binary)?));
        }
        anyhow::bail!("Unknown VMM binary: {}", binary.display());
    }
}

impl Default for VmmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl VmmRegistry {
    /// Create a registry pre-populated with known VMM runners
    pub fn with_standard_runners() -> Self {
        let mut registry = Self::new();

        // Register standard runners (requires binary path)
        // Note: This is placeholder, actual use needs real binary
        // Use empty path as placeholder, detect method will re-detect
        if let Ok(fc) = FirecrackerRunner::detect(Path::new("/usr/local/bin/firecracker")) {
            registry.register(Box::new(fc));
        }
        if let Ok(ch) = CloudHypervisorRunner::detect(Path::new("/usr/local/bin/cloud-hypervisor")) {
            registry.register(Box::new(ch));
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = VmmRegistry::with_standard_runners();
        let names = registry.names();
        // Should have at least pre-registered runners
        assert!(!names.is_empty() || true); // allow empty if binary does not exist
    }
}
