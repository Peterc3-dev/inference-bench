mod bench;
mod detect;
mod endpoints;
mod metrics;
mod output;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "inference-bench",
    version,
    about = "LLM inference benchmark CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Custom endpoint URL (overrides auto-detection)
    #[arg(long, global = true)]
    url: Option<String>,

    /// Model name (for Ollama/FastFlowLM endpoints)
    #[arg(long, global = true, default_value = "qwen3:4b")]
    model: String,

    /// Custom prompt text
    #[arg(long, global = true)]
    prompt: Option<String>,

    /// Number of tokens to generate
    #[arg(long, global = true, default_value = "100")]
    tokens: u32,

    /// Number of measurement runs
    #[arg(long, global = true, default_value = "3")]
    repeat: u32,

    /// Number of warmup runs before measuring
    #[arg(long, global = true, default_value = "1")]
    warmup: u32,

    /// Output results as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Output results as CSV
    #[arg(long, global = true)]
    csv: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Measure time to first token
    Ttft,
    /// Measure generation throughput (tokens/sec)
    Throughput,
    /// Test increasing context lengths
    Context,
    /// Run all benchmarks across all detected endpoints
    Compare,
    /// Detect available endpoints
    Detect,
}

fn main() {
    let cli = Cli::parse();

    let config = bench::BenchConfig {
        model: cli.model.clone(),
        prompt: cli
            .prompt
            .clone()
            .unwrap_or_else(|| "Explain the theory of relativity in simple terms.".to_string()),
        tokens: cli.tokens,
        repeat: cli.repeat,
        warmup: cli.warmup,
    };

    let output_format = if cli.json {
        output::OutputFormat::Json
    } else if cli.csv {
        output::OutputFormat::Csv
    } else {
        output::OutputFormat::Table
    };

    match cli.command {
        Commands::Detect => {
            let endpoints = resolve_endpoints(&cli.url);
            if endpoints.is_empty() {
                eprintln!("No endpoints detected. Use --url to specify one manually.");
                std::process::exit(1);
            }
            println!("Detected endpoints:");
            for ep in &endpoints {
                println!("  {} — {}", ep.name, ep.base_url);
            }
        }
        Commands::Ttft => {
            let endpoints = resolve_endpoints(&cli.url);
            if endpoints.is_empty() {
                eprintln!("No endpoints detected. Use --url to specify one manually.");
                std::process::exit(1);
            }
            let results = bench::run_ttft(&endpoints, &config);
            output::render(&results, "TTFT", output_format);
        }
        Commands::Throughput => {
            let endpoints = resolve_endpoints(&cli.url);
            if endpoints.is_empty() {
                eprintln!("No endpoints detected. Use --url to specify one manually.");
                std::process::exit(1);
            }
            let results = bench::run_throughput(&endpoints, &config);
            output::render(&results, "Throughput", output_format);
        }
        Commands::Context => {
            let endpoints = resolve_endpoints(&cli.url);
            if endpoints.is_empty() {
                eprintln!("No endpoints detected. Use --url to specify one manually.");
                std::process::exit(1);
            }
            let results = bench::run_context(&endpoints, &config);
            output::render_context(&results, output_format);
        }
        Commands::Compare => {
            let endpoints = resolve_endpoints(&cli.url);
            if endpoints.is_empty() {
                eprintln!("No endpoints detected. Use --url to specify one manually.");
                std::process::exit(1);
            }
            let results = bench::run_compare(&endpoints, &config);
            output::render_compare(&results, output_format);
        }
    }
}

fn resolve_endpoints(custom_url: &Option<String>) -> Vec<endpoints::Endpoint> {
    if let Some(url) = custom_url {
        // Guess the type from URL pattern
        let kind = if url.contains("11434") {
            endpoints::EndpointKind::Ollama
        } else if url.contains("52625") {
            endpoints::EndpointKind::FastFlowLM
        } else if url.contains("8080") {
            endpoints::EndpointKind::LlamaCpp
        } else {
            // Default: try Ollama-style API
            endpoints::EndpointKind::Ollama
        };
        vec![endpoints::Endpoint {
            name: "Custom".to_string(),
            base_url: url.clone(),
            kind,
        }]
    } else {
        detect::detect_all()
    }
}
