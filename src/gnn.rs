use crate::{api_err, structs};

pub fn build_route(
    coordinates: &[[f64; 2]],
    matrix: &structs::OrsMatrixResponse,
    start_index: usize,
    end_index: Option<usize>,
    round_trip: bool,
) -> Result<structs::RouteResult, api_err::ApiError> {
    let n = coordinates.len();
    if matrix.distances.len() != n || matrix.distances.iter().any(|row| row.len() != n) {
        return Err(api_err::ApiError::internal(
            "ORS matrix response size does not match coordinate count".into(),
        ));
    }

    let durations_fallback = vec![vec![0.0; n]; n];
    let durations = matrix.durations.as_ref().unwrap_or(&durations_fallback);
    if durations.len() != n || durations.iter().any(|row| row.len() != n) {
        return Err(api_err::ApiError::internal(
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
            return Err(api_err::ApiError::internal(
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
) -> Result<usize, api_err::ApiError> {
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

    best_index.ok_or_else(|| api_err::ApiError::internal("Unable to select a next route index".into()))
}
