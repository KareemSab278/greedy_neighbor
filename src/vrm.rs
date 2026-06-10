use reqwest::Client;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct VrmHealthResponse {
    status: Option<String>,
}

pub fn start_vrm_server() {
    let _ = get_vrm_env();
}

pub fn get_vrm_env() -> String {
    let vrm_base_url = std::env::var("VROOM_BASE_URL").unwrap_or_else(|_| {
        eprintln!(
            "Warning: VROOM_BASE_URL environment variable is not set. Use the .env file to set it to the base URL of your VROOM instance if you want to use optimization_profile."
        );
        std::process::exit(1);
    });
    println!("Starting with VROOM integration enabled on {}.", vrm_base_url);
    vrm_base_url
}

pub async fn ensure_vrm_running(vrm_base_url: &str) -> Result<(), String> {
    match fetch_vrm_health(vrm_base_url).await {
        Ok(health) => {
            if health.status.as_deref() == Some("ok") || health.status.as_deref() == Some("ready") {
                return Ok(());
            }
            // VROOM /health returns 200 with empty body or simple status
            // If we got any successful response, it's likely up
            Ok(())
        }
        Err(err) => {
            start_vrm_process().await?;
            Err(format!("VROOM is not running or unreachable: {}. Starting VROOM now; please wait and try again.", err))
        }
    }
}

async fn fetch_vrm_health(vrm_base_url: &str) -> Result<VrmHealthResponse, String> {
    let url = format!("{}/health", vrm_base_url.trim_end_matches('/'));
    let client = Client::builder()
        .user_agent("greedy-nn-rs/0.1")
        .build()
        .map_err(|err| format!("failed to create VROOM health client: {}", err))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("failed to contact VROOM health endpoint: {}", err))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("VROOM health endpoint returned {}", status));
    }

    // VROOM /health may return empty body or simple text
    let body = response
        .text()
        .await
        .map_err(|err| format!("failed to read VROOM health response: {}", err))?;

    if body.is_empty() || body.trim() == "200" {
        return Ok(VrmHealthResponse { status: Some("ok".to_string()) });
    }

    serde_json::from_str::<VrmHealthResponse>(&body)
        .map_err(|_| format!("unexpected VROOM health response: {}", body))
}

async fn start_vrm_process() -> Result<(), String> {
    // VROOM lives in the same docker-compose as ORS
    let command = "cd ~/ors-uk && docker compose up -d vroom";

    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to spawn VROOM startup command: {}", err))?;

    Ok(())
}