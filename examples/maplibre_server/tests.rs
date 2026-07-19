use std::io::Read as _;

use fast_mvt::MvtReaderRef;
use tiny_http::{Method, TestRequest};

use crate::http::{HttpResponse, dispatch};
use crate::state;

fn response(method: Method, path: &str, state: &state::AppState) -> HttpResponse {
    let request = TestRequest::new()
        .with_method(method)
        .with_path(path)
        .into();
    dispatch(&request, state)
}

fn header_value<'a>(response: &'a HttpResponse, name: &'static str) -> Option<&'a str> {
    response
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.as_str())
}

fn body(response: HttpResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn get_routes_return_the_approved_content_types() {
    let state = state::test_state();

    for (path, content_type) in [
        ("/health", "text/plain; charset=utf-8"),
        ("/", "text/html; charset=utf-8"),
        ("/app.js", "application/javascript; charset=utf-8"),
        ("/maplibre-gl.js", "application/javascript; charset=utf-8"),
        ("/maplibre-gl.css", "text/css; charset=utf-8"),
    ] {
        let response = response(Method::Get, path, &state);

        assert_eq!(response.status_code().0, 200, "{path}");
        assert_eq!(header_value(&response, "Content-Type"), Some(content_type));
    }
}

#[test]
fn tile_route_returns_the_approved_gzip_contract() {
    let state = state::test_state();
    let response = response(Method::Get, "/tiles/0/0/0.pbf", &state);

    assert_eq!(response.status_code().0, 200);
    assert_eq!(
        header_value(&response, "Content-Type"),
        Some("application/vnd.mapbox-vector-tile")
    );
    assert_eq!(header_value(&response, "Content-Encoding"), Some("gzip"));
    assert_eq!(header_value(&response, "Cache-Control"), Some("no-store"));
}

#[test]
fn tile_route_returns_both_feature_layers() {
    let state = state::test_state();
    let encoded = body(response(Method::Get, "/tiles/0/0/0.pbf", &state));
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(encoded.as_slice())
        .read_to_end(&mut raw)
        .unwrap();
    let reader = MvtReaderRef::new(&raw).unwrap();
    let layers = reader
        .layers()
        .map(|layer| (layer.name().to_owned(), layer.feature_count()))
        .collect::<Vec<_>>();

    assert_eq!(
        layers,
        vec![("roads".to_owned(), 1), ("buildings".to_owned(), 1)]
    );
}

#[test]
fn unknown_and_non_z0_tile_paths_return_not_found() {
    let state = state::test_state();

    for path in [
        "/missing",
        "/tiles/1/0/0.pbf",
        "/tiles/0/1/0.pbf",
        "/tiles/0/0/1.pbf",
    ] {
        assert_eq!(response(Method::Get, path, &state).status_code().0, 404);
    }
}

#[test]
fn non_get_methods_return_method_not_allowed() {
    let state = state::test_state();

    for method in [Method::Post, Method::Put, Method::Delete, Method::Patch] {
        assert_eq!(response(method, "/health", &state).status_code().0, 405);
    }
}

#[test]
fn composition_failure_returns_500_without_partial_tile_headers() {
    let state = state::test_state_with_invalid_composition_input();
    let response = response(Method::Get, "/tiles/0/0/0.pbf", &state);

    assert_eq!(response.status_code().0, 500);
    assert_eq!(
        header_value(&response, "Content-Type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(header_value(&response, "Content-Encoding"), None);
    assert_eq!(header_value(&response, "Cache-Control"), None);
    assert_eq!(body(response), b"internal server error\n");
}
