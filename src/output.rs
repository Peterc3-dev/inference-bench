use crate::bench::{BenchResult, CompareResult, ContextResult};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

pub fn render(results: &[BenchResult], title: &str, format: OutputFormat) {
    match format {
        OutputFormat::Table => render_table(results, title),
        OutputFormat::Json => {
            let data: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "endpoint": r.endpoint,
                        "metric": r.metric_name,
                        "mean": r.mean,
                        "std": r.std,
                        "unit": r.unit,
                        "values": r.values,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        OutputFormat::Csv => {
            println!("endpoint,metric,mean,std,unit");
            for r in results {
                println!(
                    "{},{},{:.2},{:.2},{}",
                    r.endpoint, r.metric_name, r.mean, r.std, r.unit
                );
            }
        }
    }
}

fn render_table(results: &[BenchResult], title: &str) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Endpoint").fg(Color::Rgb {
            r: 0,
            g: 255,
            b: 200,
        }),
        Cell::new(format!("{} (mean)", title)).fg(Color::Rgb {
            r: 0,
            g: 255,
            b: 200,
        }),
        Cell::new("Std Dev").fg(Color::Rgb {
            r: 0,
            g: 255,
            b: 200,
        }),
        Cell::new("Runs").fg(Color::Rgb {
            r: 0,
            g: 255,
            b: 200,
        }),
        Cell::new("Mem Δ (sys)").fg(Color::Rgb {
            r: 0,
            g: 255,
            b: 200,
        }),
    ]);

    for r in results {
        let mem_delta =
            r.mem_after.system_available_mib as i64 - r.mem_before.system_available_mib as i64;
        table.add_row(vec![
            Cell::new(&r.endpoint),
            Cell::new(format!("{:.1} {}", r.mean, r.unit)),
            Cell::new(format!("±{:.1}", r.std)),
            Cell::new(format!("{}", r.values.len())),
            Cell::new(format!("{:+} MiB", mem_delta)),
        ]);
    }
    println!("{}", table);
}

pub fn render_context(results: &[ContextResult], format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec![
                Cell::new("Endpoint").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("Context").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("TTFT (ms)").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("TPS").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
            ]);
            for r in results {
                table.add_row(vec![
                    Cell::new(&r.endpoint),
                    Cell::new(format!("{}", r.context_len)),
                    Cell::new(format!("{:.0}", r.ttft_ms)),
                    Cell::new(format!("{:.1}", r.tps)),
                ]);
            }
            println!("{}", table);
        }
        OutputFormat::Json => {
            let data: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "endpoint": r.endpoint,
                        "context_len": r.context_len,
                        "ttft_ms": r.ttft_ms,
                        "tps": r.tps,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        OutputFormat::Csv => {
            println!("endpoint,context_len,ttft_ms,tps");
            for r in results {
                println!(
                    "{},{},{:.0},{:.1}",
                    r.endpoint, r.context_len, r.ttft_ms, r.tps
                );
            }
        }
    }
}

pub fn render_compare(results: &[CompareResult], format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec![
                Cell::new("Endpoint").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("TTFT (ms)").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("Gen TPS").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
                Cell::new("Prompt TPS").fg(Color::Rgb {
                    r: 0,
                    g: 255,
                    b: 200,
                }),
            ]);
            for r in results {
                let prompt_str = if r.prompt_eval_tps > 0.0 {
                    format!("{:.1}", r.prompt_eval_tps)
                } else {
                    "N/A".to_string()
                };
                table.add_row(vec![
                    Cell::new(&r.endpoint),
                    Cell::new(format!("{:.0}", r.ttft_ms)),
                    Cell::new(format!("{:.1}", r.tps)),
                    Cell::new(prompt_str),
                ]);
            }
            println!("{}", table);
        }
        OutputFormat::Json => {
            let data: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "endpoint": r.endpoint,
                        "ttft_ms": r.ttft_ms,
                        "tps": r.tps,
                        "prompt_eval_tps": r.prompt_eval_tps,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        OutputFormat::Csv => {
            println!("endpoint,ttft_ms,tps,prompt_eval_tps");
            for r in results {
                println!(
                    "{},{:.0},{:.1},{:.1}",
                    r.endpoint, r.ttft_ms, r.tps, r.prompt_eval_tps
                );
            }
        }
    }
}
