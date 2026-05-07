use crate::endpoints::{self, Endpoint, EndpointKind};
use crate::metrics::{self, MemorySnapshot};
use std::io::{BufRead, BufReader};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub model: String,
    pub prompt: String,
    pub tokens: u32,
    pub repeat: u32,
    pub warmup: u32,
}

#[derive(Debug, Clone)]
pub struct BenchResult {
    pub endpoint: String,
    pub metric_name: String,
    pub values: Vec<f64>,
    pub mean: f64,
    pub std: f64,
    pub unit: String,
    pub mem_before: MemorySnapshot,
    pub mem_after: MemorySnapshot,
}

#[derive(Debug, Clone)]
pub struct ContextResult {
    pub endpoint: String,
    pub context_len: u32,
    pub ttft_ms: f64,
    pub tps: f64,
}

#[derive(Debug, Clone)]
pub struct CompareResult {
    pub endpoint: String,
    pub ttft_ms: f64,
    pub tps: f64,
    pub prompt_eval_tps: f64,
}

fn stream_generate(ep: &Endpoint, model: &str, prompt: &str, num_predict: u32) -> (f64, f64, u32) {
    let body = ep.build_body(model, prompt, num_predict, true);
    let url = ep.generate_url();

    let resp = match ureq::post(&url)
        .timeout(std::time::Duration::from_secs(120))
        .send_json(&body)
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  Request failed: {}", e);
            return (0.0, 0.0, 0);
        }
    };

    let reader = BufReader::new(resp.into_reader());
    let start = Instant::now();
    let mut first_token_time: Option<f64> = None;
    let mut token_count: u32 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let parsed = match ep.kind {
            EndpointKind::Ollama | EndpointKind::FastFlowLM => {
                endpoints::parse_ollama_stream_line(&line)
            }
            EndpointKind::LlamaCpp => {
                endpoints::parse_llamacpp_stream_line(&line)
            }
        };

        if let Some((token, done)) = parsed {
            if !token.is_empty() {
                token_count += 1;
                if first_token_time.is_none() {
                    first_token_time = Some(start.elapsed().as_secs_f64() * 1000.0);
                }
            }
            if done {
                break;
            }
        }
    }

    let total_time = start.elapsed().as_secs_f64();
    let ttft = first_token_time.unwrap_or(total_time * 1000.0);
    let tps = if total_time > 0.0 { token_count as f64 / total_time } else { 0.0 };

    (ttft, tps, token_count)
}

pub fn run_ttft(endpoints: &[Endpoint], config: &BenchConfig) -> Vec<BenchResult> {
    let mut results = Vec::new();
    for ep in endpoints {
        eprintln!("Benchmarking TTFT: {} ...", ep.name);
        for _ in 0..config.warmup {
            stream_generate(ep, &config.model, &config.prompt, 10);
        }
        let mem_before = MemorySnapshot::capture();
        let mut values = Vec::new();
        for i in 0..config.repeat {
            let (ttft, _, count) = stream_generate(ep, &config.model, &config.prompt, config.tokens);
            if count == 0 {
                eprintln!("  Run {}: FAILED (no tokens)", i + 1);
                continue;
            }
            eprintln!("  Run {}: {:.1} ms", i + 1, ttft);
            values.push(ttft);
        }
        let mem_after = MemorySnapshot::capture();
        let (mean, std) = metrics::mean_std(&values);
        results.push(BenchResult {
            endpoint: ep.name.clone(),
            metric_name: "TTFT".to_string(),
            values,
            mean,
            std,
            unit: "ms".to_string(),
            mem_before,
            mem_after,
        });
    }
    results
}

pub fn run_throughput(endpoints: &[Endpoint], config: &BenchConfig) -> Vec<BenchResult> {
    let mut results = Vec::new();
    for ep in endpoints {
        eprintln!("Benchmarking Throughput: {} ...", ep.name);
        for _ in 0..config.warmup {
            stream_generate(ep, &config.model, &config.prompt, 10);
        }
        let mem_before = MemorySnapshot::capture();
        let mut values = Vec::new();
        for i in 0..config.repeat {
            let (_, tps, count) = stream_generate(ep, &config.model, &config.prompt, config.tokens);
            if count == 0 {
                eprintln!("  Run {}: FAILED (no tokens)", i + 1);
                continue;
            }
            eprintln!("  Run {}: {:.1} t/s ({} tokens)", i + 1, tps, count);
            values.push(tps);
        }
        let mem_after = MemorySnapshot::capture();
        let (mean, std) = metrics::mean_std(&values);
        results.push(BenchResult {
            endpoint: ep.name.clone(),
            metric_name: "Throughput".to_string(),
            values,
            mean,
            std,
            unit: "t/s".to_string(),
            mem_before,
            mem_after,
        });
    }
    results
}

pub fn run_context(endpoints: &[Endpoint], config: &BenchConfig) -> Vec<ContextResult> {
    let context_lengths = [128, 256, 512, 1024, 2048, 4096];
    let mut results = Vec::new();

    for ep in endpoints {
        eprintln!("Context scaling test: {} ...", ep.name);
        for _ in 0..config.warmup {
            stream_generate(ep, &config.model, &config.prompt, 10);
        }
        for &ctx_len in &context_lengths {
            let long_prompt = config.prompt.repeat((ctx_len / 10).max(1) as usize);
            let target_len = ctx_len as usize * 4;
            let prompt = if long_prompt.len() > target_len {
                match long_prompt.char_indices().nth(target_len.min(long_prompt.chars().count())) {
                    Some((byte_idx, _)) => long_prompt[..byte_idx].to_string(),
                    None => long_prompt,
                }
            } else {
                long_prompt
            };
            let (ttft, tps, _) = stream_generate(ep, &config.model, &prompt, 20);
            eprintln!("  ctx={}: TTFT={:.0}ms TPS={:.1}", ctx_len, ttft, tps);
            results.push(ContextResult {
                endpoint: ep.name.clone(),
                context_len: ctx_len,
                ttft_ms: ttft,
                tps,
            });
        }
    }
    results
}

pub fn run_compare(endpoints: &[Endpoint], config: &BenchConfig) -> Vec<CompareResult> {
    let mut results = Vec::new();
    for ep in endpoints {
        eprintln!("Full comparison: {} ...", ep.name);
        for _ in 0..config.warmup {
            stream_generate(ep, &config.model, &config.prompt, 10);
        }

        let mut ttft_vals = Vec::new();
        let mut tps_vals = Vec::new();

        for _ in 0..config.repeat {
            let (ttft, tps, _) = stream_generate(ep, &config.model, &config.prompt, config.tokens);
            ttft_vals.push(ttft);
            tps_vals.push(tps);
        }

        let (ttft_mean, _) = metrics::mean_std(&ttft_vals);
        let (tps_mean, _) = metrics::mean_std(&tps_vals);

        // Try to get prompt eval speed from non-streaming request
        let body = ep.build_body(&config.model, &config.prompt, config.tokens, false);
        let prompt_eval_tps = match ureq::post(&ep.generate_url())
            .timeout(std::time::Duration::from_secs(120))
            .send_json(&body)
        {
            Ok(resp) => {
                let body_str = resp.into_string().unwrap_or_default();
                let timings = match ep.kind {
                    EndpointKind::Ollama | EndpointKind::FastFlowLM => {
                        endpoints::parse_ollama_timings(&body_str)
                    }
                    EndpointKind::LlamaCpp => {
                        endpoints::parse_llamacpp_timings(&body_str)
                    }
                };
                match (timings.prompt_eval_count, timings.prompt_eval_duration_ns) {
                    (Some(count), Some(dur)) if dur > 0 => {
                        count as f64 / (dur as f64 / 1_000_000_000.0)
                    }
                    _ => 0.0,
                }
            }
            Err(_) => 0.0,
        };

        results.push(CompareResult {
            endpoint: ep.name.clone(),
            ttft_ms: ttft_mean,
            tps: tps_mean,
            prompt_eval_tps,
        });
    }
    results
}
