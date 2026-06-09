use serde::{Deserialize, Serialize};
use reqwest::Client;

pub struct RouteResult {
    pub route_indices: Vec<usize>,
    pub route: Vec<[f64; 2]>,
    pub legs: Vec<RouteLeg>,
    pub total_distance_meters: f64,
    pub total_duration_seconds: f64,
}

#[derive(Clone)]
pub struct AppConfig {
    pub client: Client,
    pub ors_base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteRequest {
    pub coordinates: Vec<[f64; 2]>,
    pub start_index: Option<usize>,
    pub end_index: Option<usize>,
    pub round_trip: Option<bool>,
    pub profile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RouteResponse {
    pub route_indices: Vec<usize>,
    pub route: Vec<[f64; 2]>,
    pub legs: Vec<RouteLeg>,
    pub total_distance_meters: f64,
    pub total_duration_seconds: f64,
    pub profile: String,
    pub round_trip: bool,
    pub end_index: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RouteLeg {
    pub from_index: usize,
    pub to_index: usize,
    pub distance_meters: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Serialize)]
pub struct OrsMatrixRequest<'a> {
    pub locations: &'a [[f64; 2]],
    pub metrics: Vec<&'static str>,
    pub profile: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct OrsMatrixResponse {
    pub distances: Vec<Vec<f64>>,
    pub durations: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
