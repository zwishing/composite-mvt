#[cfg(feature = "gzip")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use composite_mvt::{Compression, MvtComposer, MvtSource};
    use fast_mvt::MvtReaderRef;

    let roads = tile_with_layers(&["roads"]);
    let pipeline = gzip(&tile_with_layers(&["pipeline", "valve"]));
    let building = tile_with_layers(&["building"]);
    let composer = MvtComposer::builder()
        .output_compression(Compression::Gzip)
        .add_source(MvtSource::new("roads").with_layers(["roads"]))
        .add_source(
            MvtSource::new("pipeline")
                .with_compression(Compression::Gzip)
                .with_layers(["pipeline", "valve"]),
        )
        .add_source(MvtSource::new("building").with_layers(["building"]))
        .build()?;
    let output = composer.compose(&[&roads, &pipeline, &building])?;
    let raw = gunzip(&output);
    let layers = MvtReaderRef::new(&raw)?
        .layers()
        .map(|layer| layer.name())
        .collect::<Vec<_>>()
        .join(",");
    println!("compression=gzip");
    println!("layers={layers}");
    Ok(())
}

#[cfg(not(feature = "gzip"))]
fn main() {
    println!("enable the gzip feature to run this example");
}

#[cfg(feature = "gzip")]
fn tile_with_layers(names: &[&str]) -> Vec<u8> {
    let mut tile = fast_mvt::MvtTileBuilder::new();
    for name in names {
        tile = tile.layer(*name).unwrap().end();
    }
    tile.encode()
}

#[cfg(feature = "gzip")]
fn gzip(input: &[u8]) -> Vec<u8> {
    use std::io::Write as _;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}

#[cfg(feature = "gzip")]
fn gunzip(input: &[u8]) -> Vec<u8> {
    use std::io::Read as _;

    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .unwrap();
    output
}
