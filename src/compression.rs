use std::borrow::Cow;

use bytes::Bytes;

use crate::error::BoxError;
use crate::{Compression, SourceError};

pub(crate) fn detect_compression(input: &[u8]) -> Compression {
    let zstd_skippable = input.get(..4).is_some_and(|prefix| {
        (0x50..=0x5f).contains(&prefix[0]) && prefix[1..] == [0x2a, 0x4d, 0x18]
    });

    if input.starts_with(&[0x1f, 0x8b]) {
        Compression::Gzip
    } else if input.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) || zstd_skippable {
        Compression::Zstd
    } else {
        Compression::None
    }
}

pub(crate) const fn feature_enabled(compression: Compression) -> bool {
    match compression {
        Compression::None => true,
        Compression::Gzip => cfg!(feature = "gzip"),
        Compression::Zstd => cfg!(feature = "zstd"),
        Compression::Brotli => cfg!(feature = "brotli"),
        Compression::Other => false,
    }
}

pub(crate) fn decompress(
    compression: Compression,
    input: &[u8],
) -> Result<Cow<'_, [u8]>, SourceError> {
    match compression {
        Compression::None => Ok(Cow::Borrowed(input)),
        Compression::Gzip => decompress_gzip(input).map(Cow::Owned),
        Compression::Zstd => decompress_zstd(input).map(Cow::Owned),
        Compression::Brotli => decompress_brotli(input).map(Cow::Owned),
        Compression::Other => Err(SourceError::UnsupportedCompression { compression }),
    }
}

#[cfg(any(feature = "gzip", feature = "zstd", feature = "brotli"))]
fn decode_failure(
    compression: Compression,
    source: impl std::error::Error + Send + Sync + 'static,
) -> SourceError {
    SourceError::DecompressionFailed {
        compression,
        source: Box::new(source),
    }
}

#[cfg(feature = "gzip")]
fn decompress_gzip(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    use std::io::Read as _;

    let mut output = Vec::new();
    flate2::read::GzDecoder::new(input)
        .read_to_end(&mut output)
        .map_err(|error| decode_failure(Compression::Gzip, error))?;
    Ok(output)
}

#[cfg(not(feature = "gzip"))]
fn decompress_gzip(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Gzip,
    })
}

#[cfg(feature = "zstd")]
fn decompress_zstd(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    zstd::stream::decode_all(input).map_err(|error| decode_failure(Compression::Zstd, error))
}

#[cfg(not(feature = "zstd"))]
fn decompress_zstd(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Zstd,
    })
}

#[cfg(feature = "brotli")]
fn decompress_brotli(input: &[u8]) -> Result<Vec<u8>, SourceError> {
    use std::io::Read as _;

    let mut output = Vec::new();
    brotli::Decompressor::new(input, 4096)
        .read_to_end(&mut output)
        .map_err(|error| decode_failure(Compression::Brotli, error))?;
    Ok(output)
}

#[cfg(not(feature = "brotli"))]
fn decompress_brotli(_: &[u8]) -> Result<Vec<u8>, SourceError> {
    Err(SourceError::CompressionFeatureDisabled {
        compression: Compression::Brotli,
    })
}

pub(crate) fn compress(compression: Compression, input: &[u8]) -> Result<Bytes, BoxError> {
    match compression {
        Compression::None => Ok(Bytes::copy_from_slice(input)),
        Compression::Gzip => compress_gzip(input).map(Bytes::from),
        Compression::Zstd => compress_zstd(input).map(Bytes::from),
        Compression::Brotli => compress_brotli(input).map(Bytes::from),
        Compression::Other => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "unsupported output compression",
        ))),
    }
}

#[cfg(feature = "gzip")]
fn compress_gzip(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    use std::io::Write as _;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input)?;
    Ok(encoder.finish()?)
}

#[cfg(not(feature = "gzip"))]
fn compress_gzip(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "gzip feature is disabled",
    )))
}

#[cfg(feature = "zstd")]
fn compress_zstd(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    Ok(zstd::stream::encode_all(input, 0)?)
}

#[cfg(not(feature = "zstd"))]
fn compress_zstd(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "zstd feature is disabled",
    )))
}

#[cfg(feature = "brotli")]
fn compress_brotli(input: &[u8]) -> Result<Vec<u8>, BoxError> {
    let mut output = Vec::new();
    let params = brotli::enc::BrotliEncoderParams::default();
    brotli::BrotliCompress(&mut std::io::Cursor::new(input), &mut output, &params)?;
    Ok(output)
}

#[cfg(not(feature = "brotli"))]
fn compress_brotli(_: &[u8]) -> Result<Vec<u8>, BoxError> {
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "brotli feature is disabled",
    )))
}

#[cfg(test)]
#[path = "compression/tests.rs"]
mod tests;
