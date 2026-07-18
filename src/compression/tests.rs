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
