// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReportData {
    pub date: String,
    pub vmm_results: Vec<VmmResult>,
    pub charts: Vec<ChartData>,
    pub scenario_rows: Vec<ScenarioRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VmmResult {
    pub name: String,
    pub scenarios: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub name: String,
    pub metrics: Vec<NamedMetric>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedMetric {
    pub name: String,
    pub value: f64,
    pub formatted: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChartData {
    pub id: String,
    pub title: String,
    pub subtitle: String,  // e.g. "Tool: sysbench 1.0.20 | Test: 10s, 1 thread"
    pub tooltip: String,   // e.g. "CPU Sysbench: Measures CPU performance by calculating prime numbers."
    pub config: String,
    pub delta_pct: f64,     // Delta percentage for title annotation
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioRow {
    pub name: String,           // internal name like "cpu-sysbench"
    pub display_name: String,   // display name with unit like "CPU Sysbench (events/s)"
    pub cells: Vec<String>,     // values without units
}
