use std::io::Read as _;

use fast_mvt::MvtReaderRef;
use tiny_http::Method;

use crate::http::{HttpResponse, dispatch};
use crate::state::{self, AppState};

fn response(method: Method, path: &str, body: &[u8], state: &AppState) -> HttpResponse {
    dispatch(&method, path, body, state)
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
fn serves_the_single_html_frontend_and_local_maplibre_assets() {
    let state = AppState::new();
    for (path, content_type) in [
        ("/", "text/html; charset=utf-8"),
        ("/maplibre-gl.js", "application/javascript; charset=utf-8"),
        ("/maplibre-gl.css", "text/css; charset=utf-8"),
    ] {
        let response = response(Method::Get, path, &[], &state);
        assert_eq!(response.status_code().0, 200, "{path}");
        assert_eq!(header_value(&response, "Content-Type"), Some(content_type));
    }
    let html = String::from_utf8(body(response(Method::Get, "/", &[], &state))).unwrap();
    assert!(html.contains("id=\"sources\""));
    assert!(!html.contains("/app.js"));
    assert!(html.contains("/fixtures/demo/{z}/{x}/{y}.pbf"));
    assert!(html.contains("/fixtures/open/{z}/{x}/{y}.pbf"));
}

#[test]
fn accepts_multiple_source_descriptions() {
    let state = AppState::new();
    let config = b"https://demotiles.maplibre.org/tiles/{z}/{x}/{y}.pbf\tgeolines,centroids,countries\tnone\nhttps://tiles.openfreemap.org/planet/latest/{z}/{x}/{y}.pbf\tlanduse\tnone";
    let response = response(Method::Post, "/sources", config, &state);

    assert_eq!(response.status_code().0, 200);
    assert_eq!(body(response), b"sources configured\n");
}

#[test]
fn accepts_browser_encoded_template_placeholders() {
    let state = AppState::new();
    let config = b"http://127.0.0.1:3010/a/%7Bz%7D/%7Bx%7D/%7By%7D.pbf\troads\tnone";

    assert_eq!(
        response(Method::Post, "/sources", config, &state)
            .status_code()
            .0,
        200
    );
}

#[test]
fn rejects_incomplete_source_descriptions() {
    let state = AppState::new();
    for config in [
        "",
        "ftp://example.com/{z}/{x}/{y}.pbf\troads\tnone",
        "http://example.com/tile.pbf\troads\tnone",
        "http://example.com/{z}/{x}/{y}.pbf\t\tnone",
        "http://example.com/{z}/{x}/{y}.pbf\troads\tbr",
    ] {
        assert_eq!(
            response(Method::Post, "/sources", config.as_bytes(), &state)
                .status_code()
                .0,
            400
        );
    }
}

#[test]
fn local_fixtures_keep_the_expected_layers() {
    let state = AppState::new();
    for (path, expected_layers) in [
        ("/fixtures/roads/0/0/0.pbf", &["roads"][..]),
        ("/fixtures/buildings/0/0/0.pbf", &["buildings"][..]),
        (
            "/fixtures/demo/1/0/0.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/0/1.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/1/0.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        (
            "/fixtures/demo/1/1/1.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        ("/fixtures/open/1/0/0.pbf", &[]),
        ("/fixtures/open/1/0/1.pbf", &[]),
        ("/fixtures/open/1/1/0.pbf", &[]),
        ("/fixtures/open/1/1/1.pbf", &[]),
        ("/fixtures/demo/2/2/1.pbf", &[]),
        ("/fixtures/open/2/2/1.pbf", &[]),
        ("/fixtures/demo/3/4/3.pbf", &[]),
        ("/fixtures/open/3/4/3.pbf", &[]),
        (
            "/fixtures/demo/4/8/5.pbf",
            &["geolines", "centroids", "countries"][..],
        ),
        ("/fixtures/open/4/8/5.pbf", &["landuse"][..]),
        ("/fixtures/demo/5/16/14.pbf", &[]),
        ("/fixtures/open/5/16/14.pbf", &[]),
    ] {
        let response = response(Method::Get, path, &[], &state);
        assert_eq!(response.status_code().0, 200);
        assert_eq!(
            header_value(&response, "Content-Type"),
            Some("application/vnd.mapbox-vector-tile")
        );
        let tile = body(response);
        let reader = MvtReaderRef::new(&tile).unwrap();
        let layers = reader
            .layers()
            .map(|layer| layer.name())
            .collect::<Vec<_>>();
        for expected_layer in expected_layers {
            assert!(layers.contains(expected_layer), "{path}: {expected_layer}");
        }
    }
}

#[test]
fn tile_route_requires_configuration() {
    let state = AppState::new();
    let response = response(Method::Get, "/tiles/0/0/0.pbf", &[], &state);
    assert_eq!(response.status_code().0, 502);
    assert_eq!(body(response), b"configure sources first\n");
}

#[test]
fn other_routes_and_methods_are_rejected() {
    let state = AppState::new();
    assert_eq!(
        response(Method::Get, "/missing", &[], &state)
            .status_code()
            .0,
        404
    );
    assert_eq!(
        response(Method::Put, "/sources", &[], &state)
            .status_code()
            .0,
        405
    );
    assert_eq!(state::DEFAULT_PORT, 3010);
}
