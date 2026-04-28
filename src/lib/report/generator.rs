// SPDX-FileCopyrightText: Copyright (c) 2026 LingCage. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use anyhow::Result;
use super::data::*;

pub fn generate_html_report(data: &ReportData, output: &Path, _chart_js: Option<&str>) -> Result<()> {
    let html = generate_html(data);
    std::fs::write(output, html)?;
    Ok(())
}

#[allow(dead_code)]
/// Calculate nice Y-axis bounds (round to nearest 100/1000/etc)
/// Returns (suggested_min, suggested_max, delta_percentage)
fn calculate_nice_y_axis(min_val: f64, max_val: f64) -> (f64, f64, f64) {
    let data_diff = max_val - min_val;

    if data_diff <= 0.0 {
        return (0.0, 100.0, 0.0);
    }

    // Calculate delta percentage
    let delta_pct = if min_val > 0.0 {
        ((max_val - min_val) / min_val) * 100.0
    } else {
        0.0
    };

    // Add 20% padding
    let padding = data_diff * 0.2;
    let raw_min = min_val - padding;
    let raw_max = max_val + padding;

    // Determine the "nice" step based on range
    let range = raw_max - raw_min;
    let nice_step = if range <= 200.0 {
        100.0
    } else if range <= 2000.0 {
        100.0
    } else if range <= 20000.0 {
        1000.0
    } else if range <= 200000.0 {
        10000.0
    } else if range <= 2000000.0 {
        100000.0
    } else {
        1000000.0
    };

    let nice_min = (raw_min / nice_step).floor() * nice_step;
    let nice_max = (raw_max / nice_step).ceil() * nice_step;

    (nice_min, nice_max, delta_pct)
}

#[allow(dead_code)]
/// Build chart configurations for VMM results
/// Returns a vector of ChartData with dynamic Y-axis bounds and Δ% annotations
fn build_charts(vmm_results: &[VmmResult]) -> Vec<ChartData> {
    use std::collections::HashMap;
    let mut charts = Vec::new();
    
    // Group scenarios by name across VMMs
    let mut scenario_map: HashMap<String, Vec<(&String, &f64)>> = HashMap::new();
    
    for vmm in vmm_results {
        for scenario in &vmm.scenarios {
            for metric in &scenario.metrics {
                scenario_map
                    .entry(metric.name.clone())
                    .or_default()
                    .push((&vmm.name, &metric.value));
            }
        }
    }
    
    // Generate charts for each scenario
    for (scenario_name, vmm_metrics) in scenario_map {
        if vmm_metrics.len() < 2 {
            continue; // Need at least 2 VMMs to compare
        }
        
        // Find min/max values
        let values: Vec<f64> = vmm_metrics.iter().map(|(_, v)| **v).collect();
        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let (suggested_min, suggested_max, delta_pct) = calculate_nice_y_axis(min_val, max_val);
        
        // Build chart title with Δ% annotation
        let title = if delta_pct > 0.1 {
            format!("📊 {} (Δ {:.1}%)", scenario_name, delta_pct)
        } else {
            format!("📊 {}", scenario_name)
        };
        
        // Determine if we should use beginAtZero
        // Use beginAtZero=false when suggested_min is close to data min
        let begin_at_zero = (suggested_min - min_val).abs() > min_val * 0.05;
        
        // Build chart config JSON
        let labels: Vec<String> = vmm_metrics.iter().map(|(name, _)| (*name).clone()).collect();
        let data_values: Vec<f64> = vmm_metrics.iter().map(|(_, v)| **v).collect();
        let colors: Vec<&'static str> = vmm_metrics.iter().map(|(name, _)| {
            match name.as_str() {
                "firecracker" => "#ff7043",
                "cloud-hypervisor" => "#4fc3f7",
                "stratovirt" => "#66bb6a",
                "crosvm" => "#ba68c8",
                _ => "#888888",
            }
        }).collect();
        
        let config = if begin_at_zero {
            format!(
                r##"{{"type":"bar","data":{{"labels":{},"datasets":[{{"label":"","data":{},"backgroundColor":{}}}]}},"options":{{"responsive":true,"maintainAspectRatio":false,"plugins":{{"legend":{{"display":false}}}},"scales":{{"x":{{"ticks":{{"color":"#888"}},"grid":{{"color":"rgba(255,255,255,0.05)"}}}},"y":{{"beginAtZero":true,"grid":{{"color":"rgba(255,255,255,0.05)"}},"ticks":{{"color":"#888"}}}}}}}}"##,
                serde_json::to_string(&labels).unwrap(),
                serde_json::to_string(&data_values).unwrap(),
                serde_json::to_string(&colors).unwrap()
            )
        } else {
            format!(
                r##"{{"type":"bar","data":{{"labels":{},"datasets":[{{"label":"","data":{},"backgroundColor":{}}}]}},"options":{{"responsive":true,"maintainAspectRatio":false,"plugins":{{"legend":{{"display":false}}}},"scales":{{"x":{{"ticks":{{"color":"#888"}},"grid":{{"color":"rgba(255,255,255,0.05)"}}}},"y":{{"min":{},"max":{},"grid":{{"color":"rgba(255,255,255,0.05)"}},"ticks":{{"color":"#888"}}}}}}}}"##,
                serde_json::to_string(&labels).unwrap(),
                serde_json::to_string(&data_values).unwrap(),
                serde_json::to_string(&colors).unwrap(),
                suggested_min,
                suggested_max
            )
        };
        
        let chart_id = format!("{}Chart", scenario_name.replace(" ", "").replace("-", ""));
        
        charts.push(ChartData {
            id: chart_id,
            title,
            subtitle: format!("Comparison across {} VMMs", vmm_metrics.len()),
            tooltip: scenario_name.clone(),
            config,
            delta_pct,
        });
    }
    
    charts
}

fn generate_html(data: &ReportData) -> String {
    let mut html = String::new();
    
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LingBench Results - "#);
    html.push_str(&data.date);
    html.push_str(r#"</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #f8f9fa 0%, #e9ecef 100%);
            min-height: 100vh;
            color: #333333;
            padding: 20px;
        }
        .container { max-width: 1400px; margin: 0 auto; }
        header { text-align: center; padding: 40px 0; }
        h1 { font-size: 2.5rem; color: #1a1a1a; margin-bottom: 10px; display: flex; align-items: center; justify-content: center; gap: 15px; }
        h1 .logo { height: 1.2em; vertical-align: middle; }
        .subtitle { color: #888; font-size: 1.1rem; }
        .chart-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(500px, 1fr)); gap: 30px; margin: 30px 0; }
        .chart-container { background: rgba(255,255,255,0.95); border-radius: 16px; padding: 25px; border: 1px solid rgba(0,0,0,0.1); box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        .chart-title { font-size: 1.3rem; color: #1a1a1a; margin-bottom: 5px; padding-bottom: 5px; border-bottom: 1px solid rgba(0,0,0,0.1); }
        .chart-subtitle { font-size: 0.85rem; color: #888; margin-bottom: 10px; }
        .tooltip-hint { position: relative; display: inline-block; cursor: help; }
        .tooltip-hint:hover::after {
            content: attr(data-tooltip);
            position: absolute;
            bottom: 100%;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(0,0,0,0.9);
            color: white;
            padding: 10px 14px;
            border-radius: 6px;
            font-size: 0.9rem;
            white-space: pre-line;
            max-width: 500px;
            min-width: 350px;
            z-index: 100;
            margin-bottom: 5px;
            line-height: 1.4;
        }
        .chart-wrapper { position: relative; height: 300px; }
        table { width: 100%; border-collapse: collapse; margin-top: 20px; }
        th, td { padding: 12px 15px; text-align: left; border-bottom: 1px solid rgba(255,255,255,0.1); }
        th { color: #333; font-weight: 500; text-transform: uppercase; font-size: 0.8rem; background: #f8f9fa; }
        tr:hover { background: rgba(0,0,0,0.05); }
        .best-value { color: #4fc3f7; font-weight: bold; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1><img class="logo" src="lingcage_logo.svg" alt="LingCage" /> LingBench Results</h1>
            <p class="subtitle">VMM Performance Comparison - "#);
    html.push_str(&data.date);
    html.push_str(r#"</p>
        </header>

        <div class="chart-grid">
"#);

    // Generate Charts
    for chart in &data.charts {
        // Build title with tooltip
        let title_html = if chart.tooltip.is_empty() {
            format!("<h3 class=\"chart-title\">{}</h3>", chart.title)
        } else {
            format!("<h3 class=\"chart-title tooltip-hint\" data-tooltip=\"{}\">{}</h3>", 
                chart.tooltip.replace("\"", "&quot;"), chart.title)
        };
        
        html.push_str(&format!(r#"
            <div class="chart-container">
                {}
                <p class="chart-subtitle">{}</p>
                <div class="chart-wrapper"><canvas id="{}"></canvas></div>
            </div>
"#, title_html, chart.subtitle, chart.id));
    }

    html.push_str(r#"
        </div>

        <div class="chart-container">
            <h2 class="chart-title">📋 Detailed Results Table</h2>
            <table>
                <thead><tr><th>Scenario</th>"#);

    // Table header
    for vmm in &data.vmm_results {
        html.push_str(&format!("<th>{}</th>", vmm.name));
    }
    html.push_str("</tr></thead><tbody>");

    // Table body - use display_name
    for row in &data.scenario_rows {
        html.push_str("<tr><td>");
        html.push_str(&row.display_name);
        html.push_str("</td>");
        for cell in &row.cells {
            html.push_str("<td>");
            html.push_str(cell);
            html.push_str("</td>");
        }
        html.push_str("</tr>");
    }

    html.push_str(r#"
                </tbody>
            </table>
        </div>
    </div>

    <script>
        const colors = {
            cloudHypervisor: '#4fc3f7',
            firecracker: '#ff7043',
            stratovirt: '#66bb6a',
            crosvm: '#ba68c8'
        };
        const commonOptions = {
            responsive: true,
            maintainAspectRatio: false,
            plugins: { legend: { display: false, labels: { color: '#888', padding: 20 } } },
            scales: { x: { ticks: { color: '#888' }, grid: { color: 'rgba(255,255,255,0.05)' } },
                      y: { ticks: { color: '#888' }, grid: { color: 'rgba(255,255,255,0.05)' } } }
        };
"#);

    // Generate Chart.js initialization code
    for chart in &data.charts {
        html.push_str(&format!("new Chart(document.getElementById('{}'), {});\n", chart.id, chart.config));
    }

    html.push_str("    </script>\n</body>\n</html>");

    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> ReportData {
        ReportData {
            date: "April 23, 2026".to_string(),
            vmm_results: vec![
                VmmResult {
                    name: "firecracker".to_string(),
                    scenarios: vec![],
                },
            ],
            charts: vec![
                ChartData {
                    id: "testChart".to_string(),
                    title: "Test Chart".to_string(),
                    subtitle: "Tool: test 1.0 | Test: 10s".to_string(),
                    tooltip: "Test tooltip".to_string(),
                    config: r#"{"type":"bar","data":{"labels":["A","B"],"datasets":[{"data":[1,2]}]}}"#.to_string(),
                    delta_pct: 0.0,
                },
            ],
            scenario_rows: vec![
                ScenarioRow {
                    name: "cpu-sysbench".to_string(),
                    display_name: "CPU Sysbench (events/s)".to_string(),
                    cells: vec!["5599".to_string()],
                },
            ],
        }
    }

    #[test]
    fn test_report_generation() {
        let data = create_test_data();
        let html = generate_html(&data);
        
        assert!(html.contains("LingCage"));
        assert!(html.contains("testChart"));
        assert!(html.contains("cpu-sysbench"));
        assert!(html.contains("tooltip-hint"));
    }

    #[test]
    fn test_html_file_generation() {
        let data = create_test_data();
        let output = std::env::temp_dir().join("test_report.html");
        generate_html_report(&data, &output).unwrap();
        
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("LingCage"));
        
        std::fs::remove_file(output).ok();
    }
    
    #[test]
    fn test_calculate_nice_y_axis() {
        // CPU Sysbench: 5535-5574, diff=38
        let (min, max, delta) = calculate_nice_y_axis(5535.0, 5574.0);
        println!("cpu-sysbench: min={}, max={}, delta={}%", min, max, delta);
        assert!(delta > 0.5 && delta < 1.0);

        // Memory: 3.8M-4.0M
        let (min, max, delta) = calculate_nice_y_axis(3800000.0, 4050000.0);
        println!("mem-sysbench: min={}, max={}, delta={}%", min, max, delta);
        assert!(delta > 5.0 && delta < 8.0);
    }
}
