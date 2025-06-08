//! # rsnx - Rust Nginx Log Parser
//!
//! A Rust library for parsing nginx access logs, inspired by the Go library [gonx](https://github.com/satyrius/gonx).
//!
//! This library provides functionality to:
//! - Parse nginx access logs using custom format strings
//! - Extract log formats from nginx configuration files
//! - Process log entries with type-safe field access
//! - Iterate over log files efficiently
//!
//! ## Quick Start
//!
//! ```rust
//! use rsnx::{Reader, Entry};
//! use std::io::Cursor;
//!
//! let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612"#;
//! let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;
//!
//! let cursor = Cursor::new(log_data);
//! let reader = Reader::new(cursor, format)?;
//!
//! for entry in reader {
//!     let entry = entry?;
//!     println!("IP: {}", entry.field("remote_addr")?);
//!     println!("Status: {}", entry.int_field("status")?);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Features
//!
//! - **Format String Parsing**: Convert nginx log format strings into regex patterns
//! - **Type-Safe Field Access**: Access fields as strings, integers, or floats with proper error handling
//! - **Nginx Config Integration**: Extract log formats directly from nginx configuration files
//! - **Iterator Interface**: Process log files line by line with Rust's iterator patterns
//! - **Error Handling**: Comprehensive error types using `thiserror`
//! - **Optional Serde Support**: Serialize/deserialize entries when the `serde` feature is enabled

pub mod entry;
pub mod error;
pub mod nginx;
pub mod parser;
pub mod reader;

// Re-export main types for convenience
pub use entry::{Entry, Fields};
pub use error::{Error, Result};
pub use parser::Parser;
pub use reader::Reader;

// Re-export nginx-specific functionality
pub use nginx::NginxReader;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_basic_parsing() {
        let log_line = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612"#;
        let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;
        
        let cursor = Cursor::new(log_line);
        let reader = Reader::new(cursor, format).unwrap();
        
        let entries: Result<Vec<_>> = reader.collect();
        let entries = entries.unwrap();
        
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        
        assert_eq!(entry.field("remote_addr").unwrap(), "127.0.0.1");
        assert_eq!(entry.field("status").unwrap(), "200");
        assert_eq!(entry.int_field("status").unwrap(), 200);
        assert_eq!(entry.int_field("body_bytes_sent").unwrap(), 612);
    }

    #[test]
    fn test_multiple_lines() {
        let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612
192.168.1.1 [08/Nov/2013:13:40:18 +0000] "POST /api/bar HTTP/1.1" 404 0"#;
        let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;
        
        let cursor = Cursor::new(log_data);
        let reader = Reader::new(cursor, format).unwrap();
        
        let entries: Result<Vec<_>> = reader.collect();
        let entries = entries.unwrap();
        
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].field("remote_addr").unwrap(), "127.0.0.1");
        assert_eq!(entries[1].field("remote_addr").unwrap(), "192.168.1.1");
        assert_eq!(entries[0].int_field("status").unwrap(), 200);
        assert_eq!(entries[1].int_field("status").unwrap(), 404);
    }
}
