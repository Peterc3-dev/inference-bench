# inference-bench

LLM inference benchmark CLI -- measures TTFT, throughput, and context scaling across Ollama, FastFlowLM, and llama.cpp endpoints.

## Features

- Auto-detects running inference servers (Ollama :11434, FastFlowLM :52625, llama.cpp :8080)
- Subcommands: `ttft`, `throughput`, `context`, `compare`, `detect`
- Configurable warmup runs, measurement repeats, token count, and prompt
- `compare` runs all benchmarks across all detected endpoints in one pass
- `context` tests increasing context lengths to profile degradation
- Output as terminal table, JSON, or CSV
- Custom endpoint URL override with `--url`

## Install

```
cargo build --release
cp target/release/inference-bench ~/.local/bin/
```

## Usage

```bash
inference-bench detect                          # list running endpoints
inference-bench ttft --model qwen3:4b           # time to first token
inference-bench throughput --tokens 200         # generation speed (tok/s)
inference-bench context --repeat 5              # context scaling curve
inference-bench compare --json                  # full benchmark, JSON output
inference-bench throughput --url http://localhost:8080  # custom endpoint
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--model` | `qwen3:4b` | Model name for Ollama/FLM |
| `--tokens` | `100` | Tokens to generate per run |
| `--repeat` | `3` | Measurement runs |
| `--warmup` | `1` | Warmup runs before measuring |
| `--json` | | Output as JSON |
| `--csv` | | Output as CSV |
| `--url` | | Override endpoint URL |
| `--prompt` | | Custom prompt text |

## Development

```bash
cargo test            # run the unit test suite
cargo clippy          # lint
cargo fmt             # format
```

CI (build + test + clippy + fmt) runs on every push and pull request via
GitHub Actions (`.github/workflows/ci.yml`).

Built with Rust + comfy-table.
