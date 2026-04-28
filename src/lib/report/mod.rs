// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

pub mod generator;
pub mod data;

pub use generator::generate_html_report;
pub use data::{ReportData, VmmResult, ScenarioResult, ChartData, NamedMetric, ScenarioRow};
