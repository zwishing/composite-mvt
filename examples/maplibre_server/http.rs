use tiny_http::{Header, Method, Request, Response};

use crate::state::AppState;

pub(crate) type HttpResponse = Response<std::io::Cursor<Vec<u8>>>;

pub(crate) fn dispatch(request: &Request, state: &AppState) -> HttpResponse {
    if request.method() != &Method::Get {
        return Response::from_string("method not allowed\n").with_status_code(405);
    }

    match request.url() {
        "/" => Response::from_data(state.html.clone())
            .with_status_code(200)
            .with_header(header("content-type", "text/html; charset=utf-8")),
        "/app.js" => Response::from_data(state.app_js.clone())
            .with_status_code(200)
            .with_header(header(
                "content-type",
                "application/javascript; charset=utf-8",
            )),
        "/maplibre-gl.js" => Response::from_data(state.maplibre_js.clone())
            .with_status_code(200)
            .with_header(header(
                "content-type",
                "application/javascript; charset=utf-8",
            )),
        "/maplibre-gl.css" => Response::from_data(state.maplibre_css.clone())
            .with_status_code(200)
            .with_header(header("content-type", "text/css; charset=utf-8")),
        "/health" => Response::from_data("ok".as_bytes().to_vec())
            .with_status_code(200)
            .with_header(header("content-type", "text/plain; charset=utf-8")),
        "/tiles/0/0/0.pbf" => match state.composer.compose(&[&state.roads, &state.buildings]) {
            Ok(tile) => Response::from_data(tile.to_vec())
                .with_status_code(200)
                .with_header(header("content-type", "application/vnd.mapbox-vector-tile"))
                .with_header(header("content-encoding", "gzip"))
                .with_header(header("cache-control", "no-store")),
            Err(_) => Response::from_string("internal server error\n")
                .with_status_code(500)
                .with_header(header("content-type", "text/plain; charset=utf-8")),
        },
        _ => Response::from_data("not found\n".as_bytes().to_vec())
            .with_status_code(404)
            .with_header(header("content-type", "text/plain; charset=utf-8")),
    }
}

pub(crate) fn serve(request: Request, state: &AppState) -> Result<(), String> {
    let response = dispatch(&request, state);
    request
        .respond(response)
        .map_err(|error| format!("failed to write response: {error}"))
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes())
        .expect("header value must be valid UTF-8 bytes")
}
