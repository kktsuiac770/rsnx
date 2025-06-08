//! Error types for the rsnx library.

use thiserror::Error;

/// Result type alias for rsnx operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur during log parsing and processing.
#[derive(Error, Debug)]
pub enum Error {
    /// Error when a field is not found in an entry.
    #[error("field '{field}' not found")]
    FieldNotFound { field: String },

    /// Error when a field value cannot be parsed as the requested type.
    #[error("field '{field}' with value '{value}' cannot be parsed as {target_type}: {source}")]
    FieldParseError {
        field: String,
        value: String,
        target_type: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error when a log line doesn't match the expected format.
    #[error("log line '{line}' does not match format '{format}'")]
    LineFormatMismatch { line: String, format: String },

    /// Error when parsing a format string into a regex.
    #[error("invalid format string '{format}': {source}")]
    InvalidFormat {
        format: String,
        #[source]
        source: regex::Error,
    },

    /// Error when a log format is not found in nginx configuration.
    #[error("log format '{format_name}' not found in nginx configuration")]
    NginxFormatNotFound { format_name: String },

    /// IO error when reading log files or nginx configuration.
    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// Regex compilation error.
    #[error("regex error: {source}")]
    Regex {
        #[from]
        source: regex::Error,
    },

    /// Error when nginx configuration parsing fails.
    #[error("failed to parse nginx configuration: {message}")]
    NginxConfigError { message: String },
}

impl Error {
    /// Create a new field not found error.
    pub fn field_not_found(field: impl Into<String>) -> Self {
        Self::FieldNotFound {
            field: field.into(),
        }
    }

    /// Create a new field parse error.
    pub fn field_parse_error(
        field: impl Into<String>,
        value: impl Into<String>,
        target_type: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::FieldParseError {
            field: field.into(),
            value: value.into(),
            target_type: target_type.into(),
            source: Box::new(source),
        }
    }

    /// Create a new line format mismatch error.
    pub fn line_format_mismatch(line: impl Into<String>, format: impl Into<String>) -> Self {
        Self::LineFormatMismatch {
            line: line.into(),
            format: format.into(),
        }
    }

    /// Create a new invalid format error.
    pub fn invalid_format(format: impl Into<String>, source: regex::Error) -> Self {
        Self::InvalidFormat {
            format: format.into(),
            source,
        }
    }

    /// Create a new nginx format not found error.
    pub fn nginx_format_not_found(format_name: impl Into<String>) -> Self {
        Self::NginxFormatNotFound {
            format_name: format_name.into(),
        }
    }

    /// Create a new nginx config error.
    pub fn nginx_config_error(message: impl Into<String>) -> Self {
        Self::NginxConfigError {
            message: message.into(),
        }
    }
}
