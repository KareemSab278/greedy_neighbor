use crate::{api_err, gnn, structs};
use axum::{Json, http::StatusCode};

pub fn get_ors_env() -> String {
    let ors_base_url = std::env::var("ORS_BASE_URL").unwrap_or_else(|_| {
        eprintln!(
            "Warning: ORS_BASE_URL environment variable is not set. Use the .env file to set it to the base URL of your OpenRouteService instance."
        );
        std::process::exit(1);
    });
    println!("Starting with ORS integration enabled on {}.", ors_base_url);
    ors_base_url
}

pub async fn fetch_ors_matrix(
    config: &structs::AppConfig,
    req: &structs::OrsMatrixRequest<'_>,
) -> Result<structs::OrsMatrixResponse, api_err::ApiError> {
    let url = format!(
        "{}/matrix/{}",
        config.ors_base_url.trim_end_matches('/'),
        req.profile
    );

    let resp = config
        .client
        .post(&url)
        .json(&serde_json::json!({
            "locations": req.locations,
            "metrics": req.metrics,
        }))
        .send()
        .await
        .map_err(|err| {
            api_err::ApiError::internal(format!("failed to call ORS matrix endpoint: {}", err))
        })?;

    let status = resp.status();
    let body = resp.text().await.map_err(|err| {
        api_err::ApiError::internal(format!("failed to read ORS response: {}", err))
    })?;
    if !status.is_success() {
        return Err(api_err::ApiError::bad_request(format!(
            "ORS matrix returned {}: {}",
            status, body
        )));
    }

    serde_json::from_str::<structs::OrsMatrixResponse>(&body).map_err(|err| {
        api_err::ApiError::internal(format!(
            "failed to parse ORS matrix JSON: {}\nbody={}",
            err, body
        ))
    })
}

pub fn handle_ors_req_res(
    response: Result<structs::OrsMatrixResponse, api_err::ApiError>,
    payload: structs::RouteRequest,
    profile: &str,
) -> Result<Json<structs::RouteResponse>, (StatusCode, Json<structs::ErrorResponse>)> {
    // This is a placeholder for where you would implement any additional handling of the ORS response if needed.
    let start_index = payload.start_index.unwrap_or(0);
    if start_index >= payload.coordinates.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(structs::ErrorResponse {
                error: "start_index is out of range".into(),
            }),
        ));
    }

    let round_trip = payload.round_trip.unwrap_or(true);
    let end_index = payload.end_index;
    if let Some(end) = end_index {
        if end >= payload.coordinates.len() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(structs::ErrorResponse {
                    error: "end_index is out of range".into(),
                }),
            ));
        }
        if end == start_index && payload.coordinates.len() > 2 && !round_trip {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(structs::ErrorResponse {
                    error:
                        "end_index cannot be the same as start_index unless round_trip is enabled"
                            .into(),
                }),
            ));
        }
    }

    let matrix = match response {
        Ok(matrix) => matrix,
        Err(err) => return Err(err.into_response()),
    };

    let response = match gnn::build_route(
        &payload.coordinates,
        &matrix,
        start_index,
        end_index,
        round_trip,
    ) {
        Ok(result) => Json(structs::RouteResponse {
            route_indices: result.route_indices,
            route: result.route,
            legs: result.legs,
            total_distance_meters: result.total_distance_meters,
            total_duration_seconds: result.total_duration_seconds,
            profile: profile.to_string(),
            round_trip,
            end_index,
        }),
        Err(err) => return Err(err.into_response()),
    };

    Ok(response)
}


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
