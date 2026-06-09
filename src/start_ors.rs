use reqwest::Client;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct OrsHealthResponse {
    status: Option<String>,
    message: Option<String>,
    error: Option<String>,
}

pub async fn ensure_ors_running(ors_base_url: &str) -> Result<(), String> {
    match fetch_health(ors_base_url).await {
        Ok(health) => {
            if let Some(status) = health.status.as_deref() {
                if status.eq_ignore_ascii_case("available") || status.eq_ignore_ascii_case("ready") {
                    return Ok(());
                }

                if status.eq_ignore_ascii_case("not ready") || status.eq_ignore_ascii_case("not_ready") {
                    return Err("ORS is not ready yet, please wait a moment and try again".into());
                }
            }

            if let Some(message) = health.message.as_deref() {
                if message.to_lowercase().contains("failed") {
                    start_ors_process().await?;
                    return Err("ORS is starting, please wait a moment and try again".into());
                }
            }

            if let Some(error) = health.error.as_deref() {
                if error.to_lowercase().contains("failed") {
                    start_ors_process().await?;
                    return Err("ORS is starting, please wait a moment and try again".into());
                }
            }

            Err("Unable to confirm ORS status. Please check the ORS instance and try again.".into())
        }
        Err(err) => {
            start_ors_process().await?;
            Err(format!("ORS is not running or unreachable: {}. Starting ORS now; please wait and try again.", err))
        }
    }
}

async fn fetch_health(ors_base_url: &str) -> Result<OrsHealthResponse, String> {
    let url = format!("{}/health", ors_base_url.trim_end_matches('/'));
    let client = Client::builder()
        .user_agent("greedy-nn-rs/0.1")
        .build()
        .map_err(|err| format!("failed to create ORS health client: {}", err))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("failed to contact ORS health endpoint: {}", err))?;

    let body = response
        .text()
        .await
        .map_err(|err| format!("failed to read ORS health response: {}", err))?;

    serde_json::from_str::<OrsHealthResponse>(&body)
        .map_err(|_| format!("unexpected ORS health response: {}", body))
}

async fn start_ors_process() -> Result<(), String> {
    let command = "cd ~/ors-uk && nohup ./RUN.sh > /tmp/ors-start.log 2>&1 &";

    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to spawn ORS startup command: {}", err))?;

    Ok(())
}
