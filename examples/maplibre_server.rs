#[cfg(feature = "gzip")]
#[path = "maplibre_server/http.rs"]
mod http;
#[cfg(feature = "gzip")]
#[path = "maplibre_server/state.rs"]
mod state;
#[cfg(feature = "gzip")]
#[cfg(test)]
#[path = "maplibre_server/tests.rs"]
mod tests;

#[cfg(feature = "gzip")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    state::run_from_environment().await
}

#[cfg(not(feature = "gzip"))]
fn main() {
    println!("enable the gzip feature to run this example");
}
