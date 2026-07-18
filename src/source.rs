use std::fmt;

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
