use tiny_http::{Header, Method, Request, Response};

use crate::state::AppState;

pub(crate) type HttpResponse = Response<std::io::Cursor<Vec<u8>>>;

pub(crate) fn dispatch(method: &Method, path: &str, body: &[u8], state: &AppState) -> HttpResponse {
    if method == &Method::Post && path == "/sources" {
        let result = match std::str::from_utf8(body) {
            Ok(body) => state.configure(body),
            Err(_) => Err("request body must be UTF-8".to_owned()),
        };
        return match result {
            Ok(()) => text_response(200, "sources configured\n"),
            Err(error) => text_response(400, &format!("{error}\n")),
        };
    }
    if method != &Method::Get {
        return text_response(405, "method not allowed\n");
    }

    match path {
        "/" => Response::from_data(crate::state::html())
            .with_status_code(200)
            .with_header(header("content-type", "text/html; charset=utf-8")),
        "/maplibre-gl.js" => Response::from_data(crate::state::maplibre_js())
            .with_status_code(200)
            .with_header(header(
                "content-type",
                "application/javascript; charset=utf-8",
            )),
        "/maplibre-gl.css" => Response::from_data(crate::state::maplibre_css())
            .with_status_code(200)
            .with_header(header("content-type", "text/css; charset=utf-8")),
        "/health" => text_response(200, "ok\n"),
        _ if crate::state::fixture(path).is_some() => {
            Response::from_data(crate::state::fixture(path).unwrap())
                .with_status_code(200)
                .with_header(header("content-type", "application/vnd.mapbox-vector-tile"))
        }
        _ if tile_coordinates(path).is_some() => {
            let (z, x, y) = tile_coordinates(path).unwrap();
            match state.compose_tile(z, x, y) {
                Ok(tile) => Response::from_data(tile)
                    .with_status_code(200)
                    .with_header(header("content-type", "application/vnd.mapbox-vector-tile"))
                    .with_header(header("cache-control", "no-store")),
                Err(error) => text_response(502, &format!("{error}\n")),
            }
        }
        _ => text_response(404, "not found\n"),
    }
}

pub(crate) fn serve(mut request: Request, state: &AppState) -> Result<(), String> {
    let mut body = Vec::new();
    request
        .as_reader()
        .read_to_end(&mut body)
        .map_err(|error| format!("failed to read request: {error}"))?;
    let response = dispatch(request.method(), request.url(), &body, state);
    request
        .respond(response)
        .map_err(|error| format!("failed to write response: {error}"))
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes())
        .expect("header value must be valid UTF-8 bytes")
}

fn text_response(status: u16, body: &str) -> HttpResponse {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(header("content-type", "text/plain; charset=utf-8"))
}

fn tile_coordinates(path: &str) -> Option<(&str, &str, &str)> {
    let path = path.strip_prefix("/tiles/")?.strip_suffix(".pbf")?;
    let mut parts = path.split('/');
    let coordinates = (parts.next()?, parts.next()?, parts.next()?);
    parts.next().is_none().then_some(coordinates)
}
