//! Integration tests for the rsnx library.

use rsnx::{Error, NginxReader, Reader};
use std::f64::consts::PI;
use std::io::Cursor;

#[test]
fn test_common_nginx_formats() {
    // Test the common nginx log format
    let log_line =
        r#"127.0.0.1 - - [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1" 200 612"#;
    let format = r#"$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();

    let entry = reader.read().unwrap().unwrap();
    assert_eq!(entry.field("remote_addr").unwrap(), "127.0.0.1");
    assert_eq!(entry.field("remote_user").unwrap(), "-");
    assert_eq!(
        entry.field("time_local").unwrap(),
        "25/Dec/2013:14:30:00 +0000"
    );
    assert_eq!(entry.field("request").unwrap(), "GET /index.html HTTP/1.1");
    assert_eq!(entry.int_field("status").unwrap(), 200);
    assert_eq!(entry.int_field("body_bytes_sent").unwrap(), 612);
}

#[test]
fn test_combined_nginx_format() {
    // Test the combined nginx log format
    let log_line = r#"192.168.1.1 - john [25/Dec/2013:14:30:00 +0000] "POST /api/login HTTP/1.1" 201 45 "https://example.com/login" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36""#;
    let format = r#"$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent""#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();

    let entry = reader.read().unwrap().unwrap();
    assert_eq!(entry.field("remote_addr").unwrap(), "192.168.1.1");
    assert_eq!(entry.field("remote_user").unwrap(), "john");
    assert_eq!(entry.field("request").unwrap(), "POST /api/login HTTP/1.1");
    assert_eq!(entry.int_field("status").unwrap(), 201);
    assert_eq!(
        entry.field("http_referer").unwrap(),
        "https://example.com/login"
    );
    assert!(entry
        .field("http_user_agent")
        .unwrap()
        .contains("Mozilla/5.0"));
}

#[test]
fn test_custom_format_with_special_characters() {
    // Test format with various special characters
    let log_line =
        r#"[INFO] 2023-12-25T14:30:00Z user@example.com "action=login&result=success" 1.234ms"#;
    let format = r#"[$level] $timestamp $user "$params" $duration"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();

    let entry = reader.read().unwrap().unwrap();
    assert_eq!(entry.field("level").unwrap(), "INFO");
    assert_eq!(entry.field("timestamp").unwrap(), "2023-12-25T14:30:00Z");
    assert_eq!(entry.field("user").unwrap(), "user@example.com");
    assert_eq!(
        entry.field("params").unwrap(),
        "action=login&result=success"
    );
    assert_eq!(entry.field("duration").unwrap(), "1.234ms");
}

#[test]
fn test_concatenated_fields() {
    // Test fields that are concatenated without separators
    let log_line = r#"example.com/api/users?id=123 GET 200"#;
    let format = r#"$host$request_uri $method $status"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();
    let entry = reader.read().unwrap().unwrap();

    assert_eq!(entry.field("host").unwrap(), "example.com");
    assert_eq!(entry.field("request_uri").unwrap(), "/api/users?id=123");
    assert_eq!(entry.field("method").unwrap(), "GET");
    assert_eq!(entry.int_field("status").unwrap(), 200);
}

#[test]
fn test_nginx_config_parsing() {
    let nginx_config = r#"
    http {
        log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                        '$status $body_bytes_sent "$http_referer" '
                        '"$http_user_agent" "$http_x_forwarded_for"';
        
        log_format simple '$remote_addr [$time_local] "$request" $status';
        
        server {
            listen 80;
            access_log /var/log/nginx/access.log main;
        }
    }
    "#;

    let log_line = r#"10.0.0.1 - admin [26/Dec/2013:15:45:30 +0000] "GET /admin/dashboard HTTP/1.1" 200 2048 "https://admin.example.com/" "Mozilla/5.0" "192.168.1.100""#;

    let config_cursor = Cursor::new(nginx_config);
    let log_cursor = Cursor::new(log_line);

    let mut reader = NginxReader::new(log_cursor, config_cursor, "main").unwrap();
    let entry = reader.read().unwrap().unwrap();

    assert_eq!(entry.field("remote_addr").unwrap(), "10.0.0.1");
    assert_eq!(entry.field("remote_user").unwrap(), "admin");
    assert_eq!(
        entry.field("request").unwrap(),
        "GET /admin/dashboard HTTP/1.1"
    );
    assert_eq!(entry.int_field("status").unwrap(), 200);
    assert_eq!(entry.int_field("body_bytes_sent").unwrap(), 2048);
    assert_eq!(
        entry.field("http_referer").unwrap(),
        "https://admin.example.com/"
    );
    assert_eq!(entry.field("http_user_agent").unwrap(), "Mozilla/5.0");
    assert_eq!(
        entry.field("http_x_forwarded_for").unwrap(),
        "192.168.1.100"
    );
}

#[test]
fn test_nginx_config_simple_format() {
    let nginx_config = r#"
    log_format simple '$remote_addr [$time_local] "$request" $status';
    "#;

    let log_line =
        r#"172.16.0.1 [26/Dec/2013:16:00:00 +0000] "DELETE /api/users/789 HTTP/1.1" 404"#;

    let config_cursor = Cursor::new(nginx_config);
    let log_cursor = Cursor::new(log_line);

    let mut reader = NginxReader::new(log_cursor, config_cursor, "simple").unwrap();
    let entry = reader.read().unwrap().unwrap();

    assert_eq!(entry.field("remote_addr").unwrap(), "172.16.0.1");
    assert_eq!(
        entry.field("time_local").unwrap(),
        "26/Dec/2013:16:00:00 +0000"
    );
    assert_eq!(
        entry.field("request").unwrap(),
        "DELETE /api/users/789 HTTP/1.1"
    );
    assert_eq!(entry.int_field("status").unwrap(), 404);
}

#[test]
fn test_multiple_log_entries() {
    let log_data = r#"127.0.0.1 [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1" 200 612
192.168.1.1 [25/Dec/2013:14:31:00 +0000] "POST /api/login HTTP/1.1" 201 45
10.0.0.1 [25/Dec/2013:14:32:00 +0000] "GET /api/users HTTP/1.1" 200 1024
172.16.0.1 [25/Dec/2013:14:33:00 +0000] "DELETE /api/users/123 HTTP/1.1" 404 0"#;

    let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

    let cursor = Cursor::new(log_data);
    let reader = Reader::new(cursor, format).unwrap();

    let entries: Result<Vec<_>, _> = reader.collect();
    let entries = entries.unwrap();

    assert_eq!(entries.len(), 4);

    // Check first entry
    assert_eq!(entries[0].field("remote_addr").unwrap(), "127.0.0.1");
    assert_eq!(entries[0].int_field("status").unwrap(), 200);

    // Check last entry
    assert_eq!(entries[3].field("remote_addr").unwrap(), "172.16.0.1");
    assert_eq!(entries[3].int_field("status").unwrap(), 404);
    assert_eq!(entries[3].int_field("body_bytes_sent").unwrap(), 0);
}

#[test]
fn test_entry_field_operations() {
    let log_line = r#"127.0.0.1 [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1" 200 612"#;
    let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();
    let mut entry = reader.read().unwrap().unwrap();

    // Test field access
    assert_eq!(entry.field("remote_addr").unwrap(), "127.0.0.1");
    assert_eq!(entry.int_field("status").unwrap(), 200);
    assert_eq!(entry.int64_field("body_bytes_sent").unwrap(), 612i64);

    // Test field setting
    entry.set_field("new_field", "test_value");
    assert_eq!(entry.field("new_field").unwrap(), "test_value");

    entry.set_uint_field("new_uint", 42u64);
    assert_eq!(entry.field("new_uint").unwrap(), "42");
    assert_eq!(entry.int_field("new_uint").unwrap(), 42);

    entry.set_float_field("new_float", PI);
    assert_eq!(entry.field("new_float").unwrap(), "3.14");
    assert!((entry.float_field("new_float").unwrap() - PI).abs() < 0.01);

    // Test partial entry
    let partial = entry.partial(&["remote_addr", "status", "nonexistent"]);
    assert_eq!(partial.field("remote_addr").unwrap(), "127.0.0.1");
    assert_eq!(partial.field("status").unwrap(), "200");
    assert_eq!(partial.field("nonexistent").unwrap(), ""); // Should be empty for missing fields

    // Test fields hash
    let hash = entry.fields_hash(&["remote_addr", "status"]);
    assert!(hash.contains("'remote_addr'=127.0.0.1"));
    assert!(hash.contains("'status'=200"));
}

#[test]
fn test_error_conditions() {
    // Test invalid format - for now, just test that empty format works
    // (This test might need to be adjusted based on what should actually be invalid)
    let result = Reader::new(Cursor::new(""), "");
    // Empty format should be valid (matches empty lines)
    assert!(result.is_ok());

    // Test line format mismatch
    let log_line = "This is not a valid log line";
    let format = r#"$remote_addr [$time_local] "$request""#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();

    let result = reader.read().unwrap();
    assert!(result.is_err());

    // Test field not found
    let log_line = r#"127.0.0.1 [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1""#;
    let format = r#"$remote_addr [$time_local] "$request""#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();
    let entry = reader.read().unwrap().unwrap();

    let result = entry.field("nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::FieldNotFound { .. }));

    // Test type conversion error
    let log_line =
        r#"127.0.0.1 [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1" not_a_number"#;
    let format = r#"$remote_addr [$time_local] "$request" $status"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format).unwrap();
    let entry = reader.read().unwrap().unwrap();

    let result = entry.int_field("status");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::FieldParseError { .. }));
}

#[test]
fn test_nginx_format_not_found() {
    let nginx_config = r#"
    log_format main '$remote_addr [$time_local] "$request"';
    "#;

    let config_cursor = Cursor::new(nginx_config);
    let log_cursor = Cursor::new("");

    let result = NginxReader::new(log_cursor, config_cursor, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Error::NginxFormatNotFound { .. }
    ));
}

#[test]
fn test_empty_lines_handling() {
    let log_data = r#"127.0.0.1 [25/Dec/2013:14:30:00 +0000] "GET /index.html HTTP/1.1" 200 612

192.168.1.1 [25/Dec/2013:14:31:00 +0000] "POST /api/login HTTP/1.1" 201 45

"#;

    let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

    let cursor = Cursor::new(log_data);
    let reader = Reader::new(cursor, format).unwrap();

    let entries: Result<Vec<_>, _> = reader.collect();
    let entries = entries.unwrap();

    // Empty lines should be skipped
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].field("remote_addr").unwrap(), "127.0.0.1");
    assert_eq!(entries[1].field("remote_addr").unwrap(), "192.168.1.1");
}
