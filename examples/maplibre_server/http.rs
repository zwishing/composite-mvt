use std::sync::Arc;

use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{Method, StatusCode, Uri},
    response::Response,
    routing::any,
};

use crate::state::AppState;

pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new().fallback(any(dispatch)).with_state(state)
}

async fn dispatch(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let path = uri.path();
    if method == Method::POST && path == "/sources" {
        let result = match std::str::from_utf8(&body) {
            Ok(body) => state.configure(body).await,
            Err(_) => Err("request body must be UTF-8".to_owned()),
        };
        return match result {
            Ok(()) => text_response(StatusCode::OK, "sources configured\n"),
            Err(error) => text_response(StatusCode::BAD_REQUEST, &format!("{error}\n")),
        };
    }
    if method != Method::GET {
        return text_response(StatusCode::METHOD_NOT_ALLOWED, "method not allowed\n");
    }

    match path {
        "/" => response(
            StatusCode::OK,
            crate::state::html(),
            "text/html; charset=utf-8",
        ),
        "/maplibre-gl.js" => response(
            StatusCode::OK,
            crate::state::maplibre_js(),
            "application/javascript; charset=utf-8",
        ),
        "/maplibre-gl.css" => response(
            StatusCode::OK,
            crate::state::maplibre_css(),
            "text/css; charset=utf-8",
        ),
        "/health" => text_response(StatusCode::OK, "ok\n"),
        _ if crate::state::fixture(path).is_some() => response(
            StatusCode::OK,
            crate::state::fixture(path).unwrap(),
            "application/vnd.mapbox-vector-tile",
        ),
        _ if tile_coordinates(path).is_some() => {
            let (z, x, y) = tile_coordinates(path).unwrap();
            match state.compose_tile(z, x, y).await {
                Ok(tile) => Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/vnd.mapbox-vector-tile")
                    .header("cache-control", "no-store")
                    .body(Body::from(tile))
                    .expect("static response headers are valid"),
                Err(error) => text_response(StatusCode::BAD_GATEWAY, &format!("{error}\n")),
            }
        }
        _ => text_response(StatusCode::NOT_FOUND, "not found\n"),
    }
}

fn response(status: StatusCode, body: impl Into<Body>, content_type: &'static str) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(body.into())
        .expect("static response headers are valid")
}

fn text_response(status: StatusCode, body: &str) -> Response {
    response(
        status,
        Body::from(body.to_owned()),
        "text/plain; charset=utf-8",
    )
}

fn tile_coordinates(path: &str) -> Option<(&str, &str, &str)> {
    let path = path.strip_prefix("/tiles/")?.strip_suffix(".pbf")?;
    let mut parts = path.split('/');
    let coordinates = (parts.next()?, parts.next()?, parts.next()?);
    parts.next().is_none().then_some(coordinates)
}
