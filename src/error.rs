use std::error::Error as StdError;

use thiserror::Error;

use crate::{Compression, LayerName, SourceId};

pub(crate) type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Errors produced while deriving [`crate::MvtSource`] metadata from MVT samples or decoding a
/// configured source input.
#[derive(Debug, Error)]
pub enum SourceError {
    /// The supplied MVT sample or request input is empty.
    #[error("MVT input is empty")]
    EmptyBytes,
    /// A multiple-sample constructor received no samples.
    #[error("no MVT samples were supplied")]
    NoSamples,
    /// The selected compression feature was not enabled in Cargo.
    #[error("the {compression} Cargo feature is disabled")]
    CompressionFeatureDisabled { compression: Compression },
    /// The selected compression is not implemented by this crate.
    #[error("unsupported compression format: {compression}")]
    UnsupportedCompression { compression: Compression },
    /// The configured decoder rejected the input bytes.
    #[error("failed to decompress {compression} input")]
    DecompressionFailed {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    /// The decoded bytes could not be parsed as an MVT.
    #[error("input is not a valid MVT")]
    InvalidMvt,
    /// An MVT layer omitted its required name or explicitly encoded it as empty.
    #[error("MVT layer name is missing or empty")]
    MissingLayerName,
    /// An MVT contains no layers.
    #[error("MVT contains no layers")]
    NoLayers,
    /// Auto-detected sample encodings were not all the same.
    #[error("sample compression mismatch: expected {expected}, got {actual}")]
    InconsistentSampleCompression {
        expected: Compression,
        actual: Compression,
    },
}

/// Errors produced while validating a [`crate::MvtComposerBuilder`].
#[derive(Debug, Error)]
pub enum BuildError {
    /// The builder has no sources.
    #[error("composer requires at least one source")]
    NoSources,
    /// Two configured sources use the same source ID.
    #[error("duplicate source id: {id}")]
    DuplicateSourceId { id: SourceId },
    /// A configured source declares no layers.
    #[error("source `{source_id}` has no layers")]
    NoLayers { source_id: SourceId },
    /// A configured source declares an empty layer name.
    #[error("source `{source_id}` contains an empty layer name")]
    EmptyLayerName { source_id: SourceId },
    /// A layer repeats within one source or violates the configured cross-source policy.
    #[error("layer `{layer}` is duplicated between `{first_source}` and `{second_source}`")]
    DuplicateLayerName {
        layer: LayerName,
        first_source: SourceId,
        second_source: SourceId,
    },
    /// A source requires a compression Cargo feature that is disabled.
    #[error("source `{source_id}` requires disabled {compression} support")]
    CompressionFeatureDisabled {
        source_id: SourceId,
        compression: Compression,
    },
    /// A source uses an unsupported compression format.
    #[error("source `{source_id}` uses unsupported compression {compression}")]
    UnsupportedCompression {
        source_id: SourceId,
        compression: Compression,
    },
    /// The selected output encoding requires a disabled Cargo feature.
    #[error("output requires disabled {compression} support")]
    OutputCompressionFeatureDisabled { compression: Compression },
    /// The selected output encoding is unsupported.
    #[error("unsupported output compression: {compression}")]
    UnsupportedOutputCompression { compression: Compression },
}

/// Errors produced while composing configured source inputs.
#[derive(Debug, Error)]
pub enum ComposeError {
    /// The request supplied a different number of inputs than the configured source count.
    #[error("input count mismatch: expected {expected}, got {actual}")]
    InputCountMismatch { expected: usize, actual: usize },
    /// A configured source input could not be decoded.
    #[error("failed to decompress source `{source_id}`")]
    SourceDecompression {
        source_id: SourceId,
        #[source]
        source: SourceError,
    },
    /// The complete composite output could not be encoded.
    #[error("failed to compress composite output as {compression}")]
    OutputCompression {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    /// The raw composite length exceeded `usize`.
    #[error("composite MVT size overflow")]
    SizeOverflow,
}
