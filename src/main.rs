mod api_err;
mod gnn;
mod ors;
mod structs;
mod vrm;

use axum::{Extension, Json, Router, http::StatusCode};
use dotenv::dotenv;
use reqwest::Client;
use reqwest::Method;
use serde_json::Value;
use std::{sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // check environment variables
    dotenv().ok().unwrap_or_else(|| {
        eprintln!("Warning: Failed to read .env file. Make sure it exists and is readable.");
        kill_server();
    });

    let vrm_base_url = vrm::get_vrm_env();
    let ors_base_url = ors::get_ors_env();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST])
        .allow_headers(Any);

    let client = Client::builder()
        .user_agent("greedy-nn-rs/0.1")
        .build()
        .unwrap();

    let config = Arc::new(structs::AppConfig {
        client,
        ors_base_url,
        vrm_base_url,
    });

    // ORS must be running else try to start it.
    if let Err(err_msg) = ors::ensure_ors_running(&config.ors_base_url).await {
        eprintln!("Error: {}", err_msg);
        kill_server();
    }

    if let Err(err_msg) = vrm::ensure_vrm_running(&config.vrm_base_url).await {
        eprintln!("Warning: {}", err_msg);
    }

    println!("Successfully connected to OpenRouteService.");

    // all ok, start server
    let app = Router::new()
        .route("/gnn", axum::routing::post(gnn_req_handler))
        .route("/optimize", axum::routing::post(vrm_req_handler))
        .layer(Extension(config))
        .layer(cors);

    // let addr = "127.0.0.1:6969";
    let addr = "0.0.0.0:6969"; // Listen on all interfaces to allow access from other machines in the network
    let listener = TcpListener::bind(addr).await.unwrap_or_else(|err| {
        eprintln!("Failed to bind to {}: {}", addr, err);
        kill_server();
    });
    println!("Server running at http://{}", addr);

    // listen and serve
    axum::serve(listener, app).await.unwrap_or_else(|err| {
        eprintln!("Server error: {}", err);
        kill_server();
    });
}

async fn vrm_req_handler(
    // this fn is the main request handler for the /optimize endpoint. It forwards the request to VROOM and returns the VROOM response.
    Extension(config): Extension<Arc<structs::AppConfig>>,
    Json(payload): Json<structs::VroomRequest>,
) -> Result<Json<Value>, (StatusCode, Json<structs::ErrorResponse>)> {
    if !payload.payload.is_object() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(structs::ErrorResponse {
                error: "VROOM optimize request must be a JSON object".into(),
            }),
        ));
    }

    let vroom_response = vrm::optimize_vroom(&config, &payload.payload).await;
    match vroom_response {
        Ok(value) => Ok(Json(value)),
        Err(err) => Err(err.into_response()),
    }
}

async fn gnn_req_handler(
    // this fn is the main request handler for the /route endpoint and validates the input, calls ORS, and builds the response.
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

    // if boss wants optimization_profile to call vroom then include in api req otherwise call ORS matrix endpoint as normal
    let optimization_profile = payload
        .optimization_profile
        .clone()
        .unwrap_or_else(|| false);

    if optimization_profile {
        // If optimization_profile is true, we want to call the vroom optimization endpoint instead of the ORS matrix endpoint.
        // This is a placeholder for where you would implement that logic. For now, we'll just return an error.
        return Err((
            StatusCode::BAD_REQUEST,
            Json(structs::ErrorResponse {
                error: "optimization_profile is not yet implemented".into(),
            }),
        ));
    } else {
        let profile = payload
            .profile
            .clone()
            .unwrap_or_else(|| "driving-car".to_string());

        let ors_request = structs::OrsMatrixRequest {
            locations: &payload.coordinates,
            metrics: vec!["distance", "duration"],
            profile: &profile,
        };

        let ors_response: Result<structs::OrsMatrixResponse, api_err::ApiError> =
            ors::fetch_ors_matrix(&config, &ors_request).await;

        ors::handle_ors_req_res(ors_response, payload, &profile)
    }
}

fn kill_server() -> ! {
    std::process::exit(1);
}
