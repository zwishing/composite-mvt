use fast_mvt::MvtTileBuilder;

pub fn tile_with_layers(names: &[&str]) -> Vec<u8> {
    let mut tile = MvtTileBuilder::new();
    for name in names {
        tile = tile.layer(*name).unwrap().end();
    }
    tile.encode()
}

#[cfg(feature = "gzip")]
pub fn gzip(input: &[u8]) -> Vec<u8> {
    use std::io::Write as _;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}
