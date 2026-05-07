use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EndpointKind {
    Ollama,
    FastFlowLM,
    LlamaCpp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    pub base_url: String,
    pub kind: EndpointKind,
}

impl Endpoint {
    /// Build the generation URL for this endpoint.
    pub fn generate_url(&self) -> String {
        match self.kind {
            EndpointKind::Ollama | EndpointKind::FastFlowLM => {
                format!("{}/api/generate", self.base_url)
            }
            EndpointKind::LlamaCpp => {
                format!("{}/completion", self.base_url)
            }
        }
    }

    /// Build the request body for a generation request.
    pub fn build_body(&self, model: &str, prompt: &str, num_predict: u32, stream: bool) -> serde_json::Value {
        match self.kind {
            EndpointKind::Ollama | EndpointKind::FastFlowLM => {
                serde_json::json!({
                    "model": model,
                    "prompt": prompt,
                    "stream": stream,
                    "options": {
                        "num_predict": num_predict
                    }
                })
            }
            EndpointKind::LlamaCpp => {
                serde_json::json!({
                    "prompt": prompt,
                    "n_predict": num_predict,
                    "stream": stream
                })
            }
        }
    }
}

/// Parse a streamed NDJSON line from Ollama/FastFlowLM and extract the token text.
/// Returns (token_text, is_done).
pub fn parse_ollama_stream_line(line: &str) -> Option<(String, bool)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
    let token = v.get("response").and_then(|r| r.as_str()).unwrap_or("").to_string();
    Some((token, done))
}

/// Parse a streamed SSE line from llama.cpp and extract the token text.
/// Returns (token_text, is_done).
pub fn parse_llamacpp_stream_line(line: &str) -> Option<(String, bool)> {
    let data = line.strip_prefix("data: ")?;
    if data.trim() == "[DONE]" {
        return Some((String::new(), true));
    }
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let stop = v.get("stop").and_then(|s| s.as_bool()).unwrap_or(false);
    let token = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
    Some((token, stop))
}

/// Parse a non-streaming response to extract timing stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiTimings {
    pub total_duration_ns: Option<u64>,
    pub prompt_eval_duration_ns: Option<u64>,
    pub eval_duration_ns: Option<u64>,
    pub eval_count: Option<u64>,
    pub prompt_eval_count: Option<u64>,
}

pub fn parse_ollama_timings(body: &str) -> ApiTimings {
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return ApiTimings::default(),
    };
    ApiTimings {
        total_duration_ns: v.get("total_duration").and_then(|d| d.as_u64()),
        prompt_eval_duration_ns: v.get("prompt_eval_duration").and_then(|d| d.as_u64()),
        eval_duration_ns: v.get("eval_duration").and_then(|d| d.as_u64()),
        eval_count: v.get("eval_count").and_then(|d| d.as_u64()),
        prompt_eval_count: v.get("prompt_eval_count").and_then(|d| d.as_u64()),
    }
}

pub fn parse_llamacpp_timings(body: &str) -> ApiTimings {
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return ApiTimings::default(),
    };
    let timings = match v.get("timings") {
        Some(t) => t,
        None => return ApiTimings::default(),
    };
    // llama.cpp reports in ms
    let prompt_ms = timings.get("prompt_ms").and_then(|d| d.as_f64());
    let predicted_ms = timings.get("predicted_ms").and_then(|d| d.as_f64());
    let predicted_n = timings.get("predicted_n").and_then(|d| d.as_u64());
    let prompt_n = timings.get("prompt_n").and_then(|d| d.as_u64());

    ApiTimings {
        total_duration_ns: prompt_ms
            .zip(predicted_ms)
            .map(|(p, g)| ((p + g) * 1_000_000.0) as u64),
        prompt_eval_duration_ns: prompt_ms.map(|m| (m * 1_000_000.0) as u64),
        eval_duration_ns: predicted_ms.map(|m| (m * 1_000_000.0) as u64),
        eval_count: predicted_n,
        prompt_eval_count: prompt_n,
    }
}
