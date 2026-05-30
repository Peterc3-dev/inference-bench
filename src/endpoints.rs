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
    pub fn build_body(
        &self,
        model: &str,
        prompt: &str,
        num_predict: u32,
        stream: bool,
    ) -> serde_json::Value {
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
    let token = v
        .get("response")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
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
    let token = v
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(kind: EndpointKind) -> Endpoint {
        Endpoint {
            name: "test".to_string(),
            base_url: "http://localhost:9999".to_string(),
            kind,
        }
    }

    #[test]
    fn generate_url_per_kind() {
        assert_eq!(
            ep(EndpointKind::Ollama).generate_url(),
            "http://localhost:9999/api/generate"
        );
        assert_eq!(
            ep(EndpointKind::FastFlowLM).generate_url(),
            "http://localhost:9999/api/generate"
        );
        assert_eq!(
            ep(EndpointKind::LlamaCpp).generate_url(),
            "http://localhost:9999/completion"
        );
    }

    #[test]
    fn build_body_ollama_shape() {
        let body = ep(EndpointKind::Ollama).build_body("qwen3:4b", "hi", 42, true);
        assert_eq!(body["model"], "qwen3:4b");
        assert_eq!(body["prompt"], "hi");
        assert_eq!(body["stream"], true);
        assert_eq!(body["options"]["num_predict"], 42);
    }

    #[test]
    fn build_body_llamacpp_shape() {
        let body = ep(EndpointKind::LlamaCpp).build_body("ignored", "hi", 42, false);
        assert_eq!(body["prompt"], "hi");
        assert_eq!(body["n_predict"], 42);
        assert_eq!(body["stream"], false);
        // llama.cpp body carries no model field
        assert!(body.get("model").is_none());
    }

    #[test]
    fn parse_ollama_stream_token() {
        let line = r#"{"response":"Hello","done":false}"#;
        assert_eq!(
            parse_ollama_stream_line(line),
            Some(("Hello".to_string(), false))
        );
    }

    #[test]
    fn parse_ollama_stream_done() {
        let line = r#"{"response":"","done":true}"#;
        assert_eq!(parse_ollama_stream_line(line), Some((String::new(), true)));
    }

    #[test]
    fn parse_ollama_stream_invalid_json() {
        assert_eq!(parse_ollama_stream_line("not json"), None);
    }

    #[test]
    fn parse_llamacpp_stream_token() {
        let line = r#"data: {"content":"Hi","stop":false}"#;
        assert_eq!(
            parse_llamacpp_stream_line(line),
            Some(("Hi".to_string(), false))
        );
    }

    #[test]
    fn parse_llamacpp_stream_done_marker() {
        assert_eq!(
            parse_llamacpp_stream_line("data: [DONE]"),
            Some((String::new(), true))
        );
    }

    #[test]
    fn parse_llamacpp_stream_stop_flag() {
        let line = r#"data: {"content":"","stop":true}"#;
        assert_eq!(
            parse_llamacpp_stream_line(line),
            Some((String::new(), true))
        );
    }

    #[test]
    fn parse_llamacpp_stream_without_prefix_is_none() {
        // Lines that are not SSE `data:` frames are ignored.
        assert_eq!(parse_llamacpp_stream_line("event: ping"), None);
    }

    #[test]
    fn parse_ollama_timings_extracts_fields() {
        let body = r#"{"total_duration":1000,"prompt_eval_duration":400,
            "eval_duration":600,"eval_count":12,"prompt_eval_count":34}"#;
        let t = parse_ollama_timings(body);
        assert_eq!(t.total_duration_ns, Some(1000));
        assert_eq!(t.prompt_eval_duration_ns, Some(400));
        assert_eq!(t.eval_duration_ns, Some(600));
        assert_eq!(t.eval_count, Some(12));
        assert_eq!(t.prompt_eval_count, Some(34));
    }

    #[test]
    fn parse_ollama_timings_invalid_is_default() {
        let t = parse_ollama_timings("garbage");
        assert_eq!(t.total_duration_ns, None);
        assert_eq!(t.prompt_eval_count, None);
    }

    #[test]
    fn parse_llamacpp_timings_converts_ms_to_ns() {
        let body = r#"{"timings":{"prompt_ms":10.0,"predicted_ms":20.0,
            "predicted_n":5,"prompt_n":7}}"#;
        let t = parse_llamacpp_timings(body);
        assert_eq!(t.prompt_eval_duration_ns, Some(10_000_000));
        assert_eq!(t.eval_duration_ns, Some(20_000_000));
        assert_eq!(t.total_duration_ns, Some(30_000_000));
        assert_eq!(t.eval_count, Some(5));
        assert_eq!(t.prompt_eval_count, Some(7));
    }

    #[test]
    fn parse_llamacpp_timings_missing_block_is_default() {
        let t = parse_llamacpp_timings(r#"{"content":"hi"}"#);
        assert_eq!(t.prompt_eval_duration_ns, None);
        assert_eq!(t.total_duration_ns, None);
    }
}
