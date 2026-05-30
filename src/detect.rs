use crate::endpoints::{Endpoint, EndpointKind};

/// Probe known local endpoints and return those that respond.
pub fn detect_all() -> Vec<Endpoint> {
    let candidates = vec![
        Endpoint {
            name: "Ollama".to_string(),
            base_url: "http://localhost:11434".to_string(),
            kind: EndpointKind::Ollama,
        },
        Endpoint {
            name: "FastFlowLM (NPU)".to_string(),
            base_url: "http://localhost:52625".to_string(),
            kind: EndpointKind::FastFlowLM,
        },
        Endpoint {
            name: "llama.cpp".to_string(),
            base_url: "http://localhost:8080".to_string(),
            kind: EndpointKind::LlamaCpp,
        },
    ];

    let mut live = Vec::new();
    for ep in candidates {
        if probe(&ep) {
            eprintln!("  [+] {} at {} — online", ep.name, ep.base_url);
            live.push(ep);
        } else {
            eprintln!("  [-] {} at {} — offline", ep.name, ep.base_url);
        }
    }
    live
}

fn probe(ep: &Endpoint) -> bool {
    let url = match ep.kind {
        EndpointKind::Ollama | EndpointKind::FastFlowLM => {
            format!("{}/api/tags", ep.base_url)
        }
        EndpointKind::LlamaCpp => {
            format!("{}/health", ep.base_url)
        }
    };

    match ureq::get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .call()
    {
        Ok(resp) => resp.status() == 200,
        Err(_) => false,
    }
}
