mod structs;
use axum::{Extension, Json, Router, http::StatusCode};
use dotenv::dotenv;
use reqwest::Client;
use std::{env, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenv().ok().unwrap_or_else(|| {
        eprintln!("Warning: Failed to read .env file. Make sure it exists and is readable.");
        std::process::exit(1);
    });

    let ors_base_url = env::var("ORS_BASE_URL").unwrap_or_else(|_| {
        eprintln!(
            "Warning: ORS_BASE_URL environment variable is not set. Use the .env file to set it to the base URL of your OpenRouteService instance."
        );
        std::process::exit(1);
    });

    println!("Using ORS base URL: {}", ors_base_url);

    let client = Client::builder()
        .user_agent("greedy-nn-rs/0.1")
        .build()
        .unwrap();

    let config = Arc::new(structs::AppConfig {
        client,
        ors_base_url,
    });

    let app = Router::new()
        .route("/route", axum::routing::post(route_handler))
        .layer(Extension(config));

    let addr = "127.0.0.1:8081";
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn route_handler(
    Extension(config): Extension<Arc<structs::AppConfig>>,
    Json(payload): Json<structs::RouteRequest>,
) -> Result<Json<structs::RouteResponse>, (StatusCode, Json<structs::ErrorResponse>)> {
    if payload.coordinates.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(structs::ErrorResponse {
                error: "At least two coordinates are required".into(),
            }),
        ));
    }

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

    let profile = payload
        .profile
        .clone()
        .unwrap_or_else(|| "driving-car".to_string());
    let ors_request = structs::OrsMatrixRequest {
        locations: &payload.coordinates,
        metrics: vec!["distance", "duration"],
        profile: &profile,
    };

    let response: Result<structs::OrsMatrixResponse, ApiError> =
        fetch_ors_matrix(&config, &ors_request).await;
    let matrix = match response {
        Ok(matrix) => matrix,
        Err(err) => return Err(err.into_response()),
    };

    let response = match build_route(
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

async fn fetch_ors_matrix(
    config: &structs::AppConfig,
    req: &structs::OrsMatrixRequest<'_>,
) -> Result<structs::OrsMatrixResponse, ApiError> {
    let url = format!("{}/matrix", config.ors_base_url.trim_end_matches('/'));

    let resp = config
        .client
        .post(&url)
        .json(&serde_json::json!({
            "locations": req.locations,
            "metrics": req.metrics,
            "profile": req.profile,
        }))
        .send()
        .await
        .map_err(|err| {
            ApiError::internal(format!("failed to call ORS matrix endpoint: {}", err))
        })?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|err| ApiError::internal(format!("failed to read ORS response: {}", err)))?;
    if !status.is_success() {
        return Err(ApiError::bad_request(format!(
            "ORS matrix returned {}: {}",
            status, body
        )));
    }

    serde_json::from_str::<structs::OrsMatrixResponse>(&body).map_err(|err| {
        ApiError::internal(format!(
            "failed to parse ORS matrix JSON: {}\nbody={}",
            err, body
        ))
    })
}

fn build_route(
    coordinates: &[[f64; 2]],
    matrix: &structs::OrsMatrixResponse,
    start_index: usize,
    end_index: Option<usize>,
    round_trip: bool,
) -> Result<structs::RouteResult, ApiError> {
    let n = coordinates.len();
    if matrix.distances.len() != n || matrix.distances.iter().any(|row| row.len() != n) {
        return Err(ApiError::internal(
            "ORS matrix response size does not match coordinate count".into(),
        ));
    }

    let durations_fallback = vec![vec![0.0; n]; n];
    let durations = matrix.durations.as_ref().unwrap_or(&durations_fallback);
    if durations.len() != n || durations.iter().any(|row| row.len() != n) {
        return Err(ApiError::internal(
            "ORS durations matrix size does not match coordinate count".into(),
        ));
    }

    let mut visited = vec![false; n];
    let mut route_indices = vec![start_index];
    visited[start_index] = true;

    let mut legs = Vec::new();
    let mut current = start_index;
    let mut total_distance = 0.0;
    let mut total_duration = 0.0;

    let mut remaining = n - 1;
    if let Some(end) = end_index {
        if end == start_index && n > 1 && round_trip {
            // If the route is a round trip and start=end, path will return to start at end.
            // Nothing special required here.
        }
    }

    while remaining > 0 {
        let next_index = select_next_index(
            current,
            &visited,
            end_index,
            remaining == 1,
            &matrix.distances,
        )?;
        if visited[next_index] {
            return Err(ApiError::internal(
                "Routing algorithm selected an already visited index".into(),
            ));
        }

        let leg_distance = matrix.distances[current][next_index];
        let leg_duration = durations[current][next_index];
        legs.push(structs::RouteLeg {
            from_index: current,
            to_index: next_index,
            distance_meters: leg_distance,
            duration_seconds: leg_duration,
        });
        total_distance += leg_distance;
        total_duration += leg_duration;

        visited[next_index] = true;
        route_indices.push(next_index);
        current = next_index;
        remaining -= 1;
    }

    if round_trip {
        let leg_distance = matrix.distances[current][start_index];
        let leg_duration = durations[current][start_index];
        legs.push(structs::RouteLeg {
            from_index: current,
            to_index: start_index,
            distance_meters: leg_distance,
            duration_seconds: leg_duration,
        });
        total_distance += leg_distance;
        total_duration += leg_duration;
        route_indices.push(start_index);
    }

    let route = route_indices.iter().map(|&idx| coordinates[idx]).collect();

    Ok(structs::RouteResult {
        route_indices,
        route,
        legs,
        total_distance_meters: total_distance,
        total_duration_seconds: total_duration,
    })
}

fn select_next_index(
    current: usize,
    visited: &[bool],
    end_index: Option<usize>,
    last_step: bool,
    distances: &[Vec<f64>],
) -> Result<usize, ApiError> {
    let n = visited.len();
    let mut best_index = None;
    let mut best_distance = f64::INFINITY;

    for candidate in 0..n {
        if visited[candidate] {
            continue;
        }
        if let Some(end) = end_index {
            if candidate == end && !last_step {
                continue;
            }
        }
        if candidate == current {
            continue;
        }

        let distance = distances[current][candidate];
        if distance.is_nan() {
            continue;
        }

        if distance < best_distance {
            best_distance = distance;
            best_index = Some(candidate);
        }
    }

    best_index.ok_or_else(|| ApiError::internal("Unable to select a next route index".into()))
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Internal(String),
}

impl ApiError {
    fn bad_request(message: String) -> Self {
        ApiError::BadRequest(message)
    }

    fn internal(message: String) -> Self {
        ApiError::Internal(message)
    }

    fn into_response(self) -> (StatusCode, Json<structs::ErrorResponse>) {
        match self {
            ApiError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(structs::ErrorResponse { error: message }),
            ),
            ApiError::Internal(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(structs::ErrorResponse { error: message }),
            ),
        }
    }
}
