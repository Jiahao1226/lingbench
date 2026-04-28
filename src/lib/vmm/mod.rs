// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

pub mod traits;
pub mod registry;
pub mod firecracker;
pub mod cloudhypervisor;
pub mod crosvm;
pub mod stratovirt;

pub use traits::{VmmRunner, VmInstance, VmConfig};
pub use registry::VmmRegistry;
pub use firecracker::FirecrackerRunner;
pub use cloudhypervisor::CloudHypervisorRunner;
pub use crosvm::CrosvmRunner;
pub use stratovirt::StratovirtRunner;
