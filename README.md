# rsnx - Rust Nginx Log Parser

[![Crates.io](https://img.shields.io/crates/v/rsnx.svg)](https://crates.io/crates/rsnx)
[![Documentation](https://docs.rs/rsnx/badge.svg)](https://docs.rs/rsnx)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for parsing nginx access logs, inspired by the Go library [gonx](https://github.com/satyrius/gonx).

## Features

- **Format String Parsing**: Convert nginx log format strings into regex patterns for efficient parsing
- **Type-Safe Field Access**: Access log fields as strings, integers, or floats with proper error handling
- **Nginx Config Integration**: Extract log formats directly from nginx configuration files
- **Iterator Interface**: Process log files line by line using Rust's iterator patterns
- **Comprehensive Error Handling**: Detailed error types using `thiserror`
- **Optional Serde Support**: Serialize/deserialize entries when the `serde` feature is enabled

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsnx = "0.1"
```

### Basic Usage

```rust
use rsnx::Reader;
use std::io::Cursor;

let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612"#;
let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

let cursor = Cursor::new(log_data);
let reader = Reader::new(cursor, format)?;

for entry in reader {
    let entry = entry?;
    println!("IP: {}", entry.field("remote_addr")?);
    println!("Status: {}", entry.int_field("status")?);
    println!("Bytes: {}", entry.int_field("body_bytes_sent")?);
}
```

### Nginx Configuration Integration

```rust
use rsnx::NginxReader;
use std::io::Cursor;

let nginx_config = r#"
log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                '$status $body_bytes_sent "$http_referer" '
                '"$http_user_agent" "$http_x_forwarded_for"';
"#;

let log_data = r#"127.0.0.1 - - [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612 "-" "curl/7.64.1" "-""#;

let config_cursor = Cursor::new(nginx_config);
let log_cursor = Cursor::new(log_data);

let reader = NginxReader::new(log_cursor, config_cursor, "main")?;

for entry in reader {
    let entry = entry?;
    println!("Request: {}", entry.field("request")?);
    println!("User Agent: {}", entry.field("http_user_agent")?);
}
```

## Supported Log Formats

The library supports any nginx log format that uses `$variable` syntax. Common formats include:

### Common Log Format
```
$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent
```

### Combined Log Format
```
$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent"
```

### Custom Formats
```
$remote_addr [$time_local] "$request" $status $request_time "$http_user_agent"
```

## API Reference

### Entry

The `Entry` struct represents a parsed log line with methods for type-safe field access:

```rust
// String access
let ip = entry.field("remote_addr")?;

// Integer access
let status = entry.int_field("status")?;          // i32
let bytes = entry.int64_field("body_bytes_sent")?; // i64

// Float access
let request_time = entry.float_field("request_time")?; // f64

// Field manipulation
entry.set_field("custom_field", "value");
entry.set_uint_field("count", 42u64);
entry.set_float_field("ratio", 3.14);

// Utility methods
let partial = entry.partial(&["remote_addr", "status"]);
let hash = entry.fields_hash(&["remote_addr", "request"]);
entry.merge(&other_entry);
```

### Reader

The `Reader` struct provides an iterator interface for processing log files:

```rust
// Create reader
let reader = Reader::new(input, format)?;

// Iterator interface
for entry in reader {
    let entry = entry?;
    // process entry
}

// Collect all entries
let entries = reader.collect_all()?;

// Process with closure
reader.process_entries(|entry| {
    println!("{}", entry.field("remote_addr")?);
    Ok(())
})?;
```

### NginxReader

The `NginxReader` extracts log formats from nginx configuration files:

```rust
let reader = NginxReader::new(log_input, nginx_config, "format_name")?;
```

## Error Handling

The library provides comprehensive error handling with detailed error messages:

```rust
use rsnx::Error;

match entry.field("nonexistent") {
    Ok(value) => println!("Value: {}", value),
    Err(Error::FieldNotFound { field }) => {
        println!("Field '{}' not found", field);
    }
    Err(e) => println!("Other error: {}", e),
}
```

Error types include:
- `FieldNotFound`: When a requested field doesn't exist
- `FieldParseError`: When type conversion fails
- `LineFormatMismatch`: When a log line doesn't match the expected format
- `InvalidFormat`: When a format string is invalid
- `NginxFormatNotFound`: When a log format isn't found in nginx config
- `Io`: For I/O related errors

## Performance

The library is designed for efficient log processing:

- **Lazy Parsing**: Log lines are parsed on-demand as you iterate
- **Zero-Copy Field Access**: String fields return references to avoid copying
- **Compiled Regex**: Format strings are compiled once and reused
- **Memory Efficient**: Suitable for processing large log files

## Examples

See the [examples](examples/) directory for more comprehensive usage examples:

- [`basic.rs`](examples/basic.rs): Basic usage patterns and error handling

Run examples with:
```bash
cargo run --example basic
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests
```

## Features

### Optional Features

- `serde`: Enable serialization/deserialization support for `Entry`

```toml
[dependencies]
rsnx = { version = "0.1", features = ["serde"] }
```

## Comparison with gonx

This library aims to provide similar functionality to the Go library [gonx](https://github.com/satyrius/gonx) while following Rust idioms:

| Feature | gonx (Go) | rsnx (Rust) |
|---------|-----------|-------------|
| Format parsing | ✅ | ✅ |
| Nginx config parsing | ✅ | ✅ |
| Type-safe field access | ✅ | ✅ |
| Iterator interface | ✅ | ✅ |
| Error handling | `(value, error)` | `Result<T, Error>` |
| Memory management | GC | Ownership |
| Concurrency | Goroutines | (Future: async/await) |

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Inspired by the [gonx](https://github.com/satyrius/gonx) library by [@satyrius](https://github.com/satyrius)
- Thanks to the Rust community for excellent crates like `regex` and `thiserror`
