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
    flate2::read::MultiGzDecoder::new(input)
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
mod tests {
    use std::borrow::Cow;

    use super::*;
    #[cfg(any(not(feature = "gzip"), not(feature = "zstd"), not(feature = "brotli")))]
    use crate::error::BoxError;
    use crate::{Compression, SourceError};

    #[test]
    fn detects_known_frame_signatures() {
        assert_eq!(detect_compression(&[0x1f, 0x8b, 0x08]), Compression::Gzip);
        assert_eq!(
            detect_compression(&[0x28, 0xb5, 0x2f, 0xfd]),
            Compression::Zstd
        );
        assert_eq!(
            detect_compression(&[0x50, 0x2a, 0x4d, 0x18]),
            Compression::Zstd
        );
        assert_eq!(
            detect_compression(&[0x5f, 0x2a, 0x4d, 0x18]),
            Compression::Zstd
        );
        assert_eq!(
            detect_compression(&[0x60, 0x2a, 0x4d, 0x18]),
            Compression::None
        );
        assert_eq!(detect_compression(&[0x1a, 0x00]), Compression::None);
    }

    #[test]
    fn none_decompression_borrows_input() {
        let input = b"raw";
        let output = decompress(Compression::None, input).unwrap();
        assert!(matches!(output, Cow::Borrowed(_)));
        assert_eq!(output.as_ref(), input);
    }

    #[test]
    fn reports_feature_availability() {
        assert!(feature_enabled(Compression::None));
        assert!(!feature_enabled(Compression::Other));
        assert_eq!(feature_enabled(Compression::Gzip), cfg!(feature = "gzip"));
        assert_eq!(feature_enabled(Compression::Zstd), cfg!(feature = "zstd"));
        assert_eq!(
            feature_enabled(Compression::Brotli),
            cfg!(feature = "brotli")
        );
    }

    #[test]
    fn none_compression_copies_input_into_bytes() {
        assert_eq!(
            compress(Compression::None, b"raw").unwrap().as_ref(),
            b"raw"
        );
    }

    #[test]
    fn other_decompression_returns_typed_unsupported_error() {
        assert!(matches!(
            decompress(Compression::Other, b"raw"),
            Err(SourceError::UnsupportedCompression {
                compression: Compression::Other
            })
        ));
    }

    #[test]
    fn other_compression_returns_unsupported_io_error() {
        let error = compress(Compression::Other, b"raw").unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::Unsupported)
        );
    }

    #[cfg(not(feature = "gzip"))]
    #[test]
    fn disabled_gzip_decompression_returns_typed_error() {
        assert!(matches!(
            decompress(Compression::Gzip, b"raw"),
            Err(SourceError::CompressionFeatureDisabled {
                compression: Compression::Gzip
            })
        ));
    }

    #[cfg(not(feature = "zstd"))]
    #[test]
    fn disabled_zstd_decompression_returns_typed_error() {
        assert!(matches!(
            decompress(Compression::Zstd, b"raw"),
            Err(SourceError::CompressionFeatureDisabled {
                compression: Compression::Zstd
            })
        ));
    }

    #[cfg(not(feature = "brotli"))]
    #[test]
    fn disabled_brotli_decompression_returns_typed_error() {
        assert!(matches!(
            decompress(Compression::Brotli, b"raw"),
            Err(SourceError::CompressionFeatureDisabled {
                compression: Compression::Brotli
            })
        ));
    }

    #[cfg(not(feature = "gzip"))]
    #[test]
    fn disabled_gzip_compression_returns_unsupported_io_error() {
        assert_unsupported(compress(Compression::Gzip, b"raw"));
    }

    #[cfg(not(feature = "zstd"))]
    #[test]
    fn disabled_zstd_compression_returns_unsupported_io_error() {
        assert_unsupported(compress(Compression::Zstd, b"raw"));
    }

    #[cfg(not(feature = "brotli"))]
    #[test]
    fn disabled_brotli_compression_returns_unsupported_io_error() {
        assert_unsupported(compress(Compression::Brotli, b"raw"));
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn gzip_round_trip() {
        let encoded = compress(Compression::Gzip, b"tile").unwrap();
        assert!(encoded.starts_with(&[0x1f, 0x8b]));
        assert_eq!(
            decompress(Compression::Gzip, &encoded).unwrap().as_ref(),
            b"tile"
        );
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn zstd_round_trip() {
        let encoded = compress(Compression::Zstd, b"tile").unwrap();
        assert!(encoded.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));
        assert_eq!(
            decompress(Compression::Zstd, &encoded).unwrap().as_ref(),
            b"tile"
        );
    }

    #[cfg(feature = "brotli")]
    #[test]
    fn brotli_round_trip() {
        let encoded = compress(Compression::Brotli, b"tile").unwrap();
        assert_eq!(
            decompress(Compression::Brotli, &encoded).unwrap().as_ref(),
            b"tile"
        );
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn invalid_gzip_input_returns_decompression_error() {
        assert!(matches!(
            decompress(Compression::Gzip, b"raw"),
            Err(SourceError::DecompressionFailed {
                compression: Compression::Gzip,
                ..
            })
        ));
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn invalid_zstd_input_returns_decompression_error() {
        assert!(matches!(
            decompress(Compression::Zstd, b"raw"),
            Err(SourceError::DecompressionFailed {
                compression: Compression::Zstd,
                ..
            })
        ));
    }

    #[cfg(feature = "brotli")]
    #[test]
    fn invalid_brotli_input_returns_decompression_error() {
        assert!(matches!(
            decompress(Compression::Brotli, b"raw"),
            Err(SourceError::DecompressionFailed {
                compression: Compression::Brotli,
                ..
            })
        ));
    }

    #[cfg(any(not(feature = "gzip"), not(feature = "zstd"), not(feature = "brotli")))]
    fn assert_unsupported(result: Result<bytes::Bytes, BoxError>) {
        let error = result.unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::Unsupported)
        );
    }
}
