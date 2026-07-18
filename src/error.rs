use std::error::Error as StdError;

use thiserror::Error;

use crate::{Compression, LayerName, SourceId};

pub(crate) type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("MVT input is empty")]
    EmptyBytes,
    #[error("no MVT samples were supplied")]
    NoSamples,
    #[error("the {compression} Cargo feature is disabled")]
    CompressionFeatureDisabled { compression: Compression },
    #[error("unsupported compression format: {compression}")]
    UnsupportedCompression { compression: Compression },
    #[error("failed to decompress {compression} input")]
    DecompressionFailed {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    #[error("input is not a valid MVT")]
    InvalidMvt,
    #[error("MVT layer name is missing")]
    MissingLayerName,
    #[error("MVT layer name is empty")]
    EmptyLayerName,
    #[error("MVT contains no layers")]
    NoLayers,
    #[error("sample compression mismatch: expected {expected}, got {actual}")]
    InconsistentSampleCompression {
        expected: Compression,
        actual: Compression,
    },
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("composer requires at least one source")]
    NoSources,
    #[error("duplicate source id: {id}")]
    DuplicateSourceId { id: SourceId },
    #[error("source `{source_id}` has no layers")]
    NoLayers { source_id: SourceId },
    #[error("source `{source_id}` contains an empty layer name")]
    EmptyLayerName { source_id: SourceId },
    #[error("layer `{layer}` is duplicated between `{first_source}` and `{second_source}`")]
    DuplicateLayerName {
        layer: LayerName,
        first_source: SourceId,
        second_source: SourceId,
    },
    #[error("source `{source_id}` requires disabled {compression} support")]
    CompressionFeatureDisabled {
        source_id: SourceId,
        compression: Compression,
    },
    #[error("source `{source_id}` uses unsupported compression {compression}")]
    UnsupportedCompression {
        source_id: SourceId,
        compression: Compression,
    },
    #[error("output requires disabled {compression} support")]
    OutputCompressionFeatureDisabled { compression: Compression },
    #[error("unsupported output compression: {compression}")]
    UnsupportedOutputCompression { compression: Compression },
}

#[derive(Debug, Error)]
pub enum ComposeError {
    #[error("input count mismatch: expected {expected}, got {actual}")]
    InputCountMismatch { expected: usize, actual: usize },
    #[error("failed to decompress source `{source_id}`")]
    SourceDecompression {
        source_id: SourceId,
        #[source]
        source: SourceError,
    },
    #[error("failed to compress composite output as {compression}")]
    OutputCompression {
        compression: Compression,
        #[source]
        source: BoxError,
    },
    #[error("composite MVT size overflow")]
    SizeOverflow,
}
