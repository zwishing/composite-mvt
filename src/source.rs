use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;

use fast_mvt::{MvtError, MvtReaderRef};

use crate::SourceError;
use crate::compression::{decompress, detect_compression};

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(Box<str>);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value.into_boxed_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_newtype!(SourceId);
string_newtype!(LayerName);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
    Brotli,
    Other,
}

impl Compression {
    #[must_use]
    pub const fn content_encoding(self) -> Option<&'static str> {
        match self {
            Self::None | Self::Other => None,
            Self::Gzip => Some("gzip"),
            Self::Zstd => Some("zstd"),
            Self::Brotli => Some("br"),
        }
    }
}

impl fmt::Display for Compression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Zstd => "zstd",
            Self::Brotli => "brotli",
            Self::Other => "other",
        })
    }
}

#[derive(Debug, Clone)]
pub struct MvtSource {
    id: SourceId,
    compression: Compression,
    layers: Box<[LayerName]>,
}

impl MvtSource {
    #[must_use]
    pub fn new(id: impl Into<SourceId>) -> Self {
        Self {
            id: id.into(),
            compression: Compression::None,
            layers: Box::new([]),
        }
    }

    #[must_use]
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    #[must_use]
    pub fn with_layers<I, S>(mut self, layers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<LayerName>,
    {
        self.layers = layers.into_iter().map(Into::into).collect();
        self
    }

    pub fn from_mvt(id: impl Into<SourceId>, bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.is_empty() {
            return Err(SourceError::EmptyBytes);
        }

        Self::from_mvt_with_compression(id, bytes, detect_compression(bytes))
    }

    pub fn from_mvt_with_compression(
        id: impl Into<SourceId>,
        bytes: &[u8],
        compression: Compression,
    ) -> Result<Self, SourceError> {
        if bytes.is_empty() {
            return Err(SourceError::EmptyBytes);
        }

        let raw = decompress(compression, bytes)?;
        Ok(Self {
            id: id.into(),
            compression,
            layers: read_layers(raw.as_ref())?,
        })
    }

    pub fn from_mvts<I, B>(id: impl Into<SourceId>, inputs: I) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let inputs: Vec<B> = inputs.into_iter().collect();
        let first = inputs.first().ok_or(SourceError::NoSamples)?;
        let expected = detect_compression(first.as_ref());

        for input in &inputs {
            let actual = detect_compression(input.as_ref());
            if actual != expected {
                return Err(SourceError::InconsistentSampleCompression { expected, actual });
            }
        }

        Self::from_mvts_with_compression(id, inputs, expected)
    }

    pub fn from_mvts_with_compression<I, B>(
        id: impl Into<SourceId>,
        inputs: I,
        compression: Compression,
    ) -> Result<Self, SourceError>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut found_sample = false;
        let mut seen = HashSet::new();
        let mut layers = Vec::new();

        for input in inputs {
            found_sample = true;
            if input.as_ref().is_empty() {
                return Err(SourceError::EmptyBytes);
            }

            let raw = decompress(compression, input.as_ref())?;
            for layer in read_layers(raw.as_ref())? {
                if seen.insert(layer.clone()) {
                    layers.push(layer);
                }
            }
        }

        if !found_sample {
            return Err(SourceError::NoSamples);
        }

        Ok(Self {
            id: id.into(),
            compression,
            layers: layers.into_boxed_slice(),
        })
    }

    pub fn decompress<'a>(&self, input: &'a [u8]) -> Result<Cow<'a, [u8]>, SourceError> {
        decompress(self.compression, input)
    }

    #[must_use]
    pub fn id(&self) -> &SourceId {
        &self.id
    }

    #[must_use]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    #[must_use]
    pub fn layers(&self) -> &[LayerName] {
        &self.layers
    }
}

fn read_layers(bytes: &[u8]) -> Result<Box<[LayerName]>, SourceError> {
    let reader = MvtReaderRef::new(bytes).map_err(|error| match error {
        MvtError::MissingLayerName => SourceError::MissingLayerName,
        _ => SourceError::InvalidMvt,
    })?;
    let layers: Box<[LayerName]> = reader.layers().map(|layer| layer.name().into()).collect();

    if layers.is_empty() {
        return Err(SourceError::NoLayers);
    }

    Ok(layers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_names_convert_from_strings_and_display() {
        let source = SourceId::from("roads");
        let layer = LayerName::from(String::from("road_labels"));

        assert_eq!(source.as_ref(), "roads");
        assert_eq!(source.to_string(), "roads");
        assert_eq!(layer.as_ref(), "road_labels");
        assert_eq!(layer.to_string(), "road_labels");
    }

    #[test]
    fn compression_names_match_content_encodings() {
        assert_eq!(Compression::None.content_encoding(), None);
        assert_eq!(Compression::Gzip.content_encoding(), Some("gzip"));
        assert_eq!(Compression::Zstd.content_encoding(), Some("zstd"));
        assert_eq!(Compression::Brotli.content_encoding(), Some("br"));
        assert_eq!(Compression::Other.content_encoding(), None);
    }
}
