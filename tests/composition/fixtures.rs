pub fn layer_names(bytes: &[u8]) -> Vec<String> {
    fast_mvt::MvtReaderRef::new(bytes)
        .unwrap()
        .layers()
        .map(|layer| layer.name().to_owned())
        .collect()
}

#[cfg(feature = "gzip")]
pub fn gunzip(input: &[u8]) -> Vec<u8> {
    use std::io::Read as _;

    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .unwrap();
    output
}

#[cfg(feature = "zstd")]
pub fn zstd_encode(input: &[u8]) -> Vec<u8> {
    zstd::stream::encode_all(input, 0).unwrap()
}

#[cfg(feature = "zstd")]
pub fn zstd_decode(input: &[u8]) -> Vec<u8> {
    zstd::stream::decode_all(input).unwrap()
}

#[cfg(feature = "brotli")]
pub fn brotli_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    brotli::BrotliCompress(
        &mut std::io::Cursor::new(input),
        &mut output,
        &brotli::enc::BrotliEncoderParams::default(),
    )
    .unwrap();
    output
}

#[cfg(feature = "brotli")]
pub fn brotli_decode(input: &[u8]) -> Vec<u8> {
    use std::io::Read as _;

    let mut output = Vec::new();
    brotli::Decompressor::new(input, 4096)
        .read_to_end(&mut output)
        .unwrap();
    output
}
