//! Basic usage example for the rsnx library.
//!
//! This example demonstrates how to parse nginx access logs using custom format strings
//! and how to extract log formats from nginx configuration files.

use rsnx::{NginxReader, Reader};
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== rsnx Basic Usage Example ===\n");

    // Example 1: Basic log parsing with custom format
    basic_parsing_example()?;

    // Example 2: Nginx configuration parsing
    nginx_config_example()?;

    // Example 3: Processing multiple log entries
    multiple_entries_example()?;

    // Example 4: Error handling
    error_handling_example()?;

    Ok(())
}

/// Example 1: Basic log parsing with a custom format string
fn basic_parsing_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Basic Log Parsing");
    println!("-------------------");

    // Sample nginx access log line
    let log_line = r#"127.0.0.1 - - [08/Nov/2013:13:39:18 +0000] "GET /api/users/123 HTTP/1.1" 200 612 "-" "curl/7.64.1""#;

    // Define the log format (nginx combined format)
    let format = r#"$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent""#;

    println!("Log line: {}", log_line);
    println!("Format:   {}", format);
    println!();

    // Create a reader
    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format)?;

    // Parse the log entry
    if let Some(result) = reader.read() {
        let entry = result?;

        println!("Parsed fields:");
        println!("  Remote Address: {}", entry.field("remote_addr")?);
        println!("  Remote User:    {}", entry.field("remote_user")?);
        println!("  Time Local:     {}", entry.field("time_local")?);
        println!("  Request:        {}", entry.field("request")?);
        println!(
            "  Status:         {} (as int: {})",
            entry.field("status")?,
            entry.int_field("status")?
        );
        println!(
            "  Bytes Sent:     {} (as int: {})",
            entry.field("body_bytes_sent")?,
            entry.int_field("body_bytes_sent")?
        );
        println!("  Referer:        {}", entry.field("http_referer")?);
        println!("  User Agent:     {}", entry.field("http_user_agent")?);
    }

    println!("\n");
    Ok(())
}

/// Example 2: Extracting log format from nginx configuration
fn nginx_config_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Nginx Configuration Parsing");
    println!("------------------------------");

    // Sample nginx configuration with log format
    let nginx_config = r#"
    http {
        log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                        '$status $body_bytes_sent "$http_referer" '
                        '"$http_user_agent" "$http_x_forwarded_for"';
        
        log_format simple '$remote_addr [$time_local] "$request" $status';
        
        access_log /var/log/nginx/access.log main;
    }
    "#;

    // Sample log line that matches the 'main' format
    let log_line = r#"192.168.1.100 - john [09/Nov/2013:14:22:33 +0000] "POST /api/login HTTP/1.1" 201 45 "https://example.com/login" "Mozilla/5.0" "10.0.0.1""#;

    println!("Nginx config contains 'main' and 'simple' log formats");
    println!("Log line: {}", log_line);
    println!();

    // Create nginx reader using the 'main' format
    let config_cursor = Cursor::new(nginx_config);
    let log_cursor = Cursor::new(log_line);
    let mut reader = NginxReader::new(log_cursor, config_cursor, "main")?;

    // Parse the log entry
    if let Some(result) = reader.read() {
        let entry = result?;

        println!("Parsed using 'main' format:");
        println!("  Remote Address:     {}", entry.field("remote_addr")?);
        println!("  Remote User:        {}", entry.field("remote_user")?);
        println!("  Time Local:         {}", entry.field("time_local")?);
        println!("  Request:            {}", entry.field("request")?);
        println!("  Status:             {}", entry.int_field("status")?);
        println!(
            "  Bytes Sent:         {}",
            entry.int_field("body_bytes_sent")?
        );
        println!("  Referer:            {}", entry.field("http_referer")?);
        println!("  User Agent:         {}", entry.field("http_user_agent")?);
        println!(
            "  X-Forwarded-For:    {}",
            entry.field("http_x_forwarded_for")?
        );
    }

    println!("\n");
    Ok(())
}

/// Example 3: Processing multiple log entries
fn multiple_entries_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Processing Multiple Log Entries");
    println!("----------------------------------");

    let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/users HTTP/1.1" 200 1024
192.168.1.1 [08/Nov/2013:13:40:22 +0000] "POST /api/users HTTP/1.1" 201 256
10.0.0.1 [08/Nov/2013:13:41:15 +0000] "GET /api/users/123 HTTP/1.1" 200 512
172.16.0.1 [08/Nov/2013:13:42:33 +0000] "DELETE /api/users/456 HTTP/1.1" 404 0"#;

    let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

    println!("Processing {} log lines...", log_data.lines().count());
    println!();

    let cursor = Cursor::new(log_data);
    let reader = Reader::new(cursor, format)?;

    let mut total_bytes = 0u64;
    let mut status_counts = std::collections::HashMap::new();
    let mut entry_count = 0;

    // Process entries using iterator
    for result in reader {
        let entry = result?;
        entry_count += 1;

        // Accumulate statistics
        let bytes = entry.int_field("body_bytes_sent")? as u64;
        total_bytes += bytes;

        let status = entry.int_field("status")?;
        *status_counts.entry(status).or_insert(0) += 1;

        println!(
            "Entry {}: {} {} -> {} ({} bytes)",
            entry_count,
            entry.field("remote_addr")?,
            entry.field("request")?,
            status,
            bytes
        );
    }

    println!();
    println!("Statistics:");
    println!("  Total entries: {}", entry_count);
    println!("  Total bytes:   {}", total_bytes);
    println!("  Status codes:");
    for (status, count) in status_counts {
        println!("    {}: {} times", status, count);
    }

    println!("\n");
    Ok(())
}

/// Example 4: Error handling scenarios
fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Error Handling");
    println!("-----------------");

    // Example 4a: Invalid format string
    println!("4a. Invalid format string:");
    match Reader::new(Cursor::new(""), "[invalid regex") {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => println!("  Error: {}", e),
    }

    // Example 4b: Line doesn't match format
    println!("\n4b. Line doesn't match format:");
    let log_line = "This is not a valid log line";
    let format = r#"$remote_addr [$time_local] "$request""#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format)?;

    match reader.read() {
        Some(Ok(_)) => println!("  Unexpected success"),
        Some(Err(e)) => println!("  Error: {}", e),
        None => println!("  No data"),
    }

    // Example 4c: Field not found
    println!("\n4c. Field not found:");
    let log_line = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1""#;
    let format = r#"$remote_addr [$time_local] "$request""#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format)?;

    if let Some(Ok(entry)) = reader.read() {
        match entry.field("nonexistent_field") {
            Ok(_) => println!("  Unexpected success"),
            Err(e) => println!("  Error: {}", e),
        }
    }

    // Example 4d: Type conversion error
    println!("\n4d. Type conversion error:");
    let log_line = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" not_a_number"#;
    let format = r#"$remote_addr [$time_local] "$request" $status"#;

    let cursor = Cursor::new(log_line);
    let mut reader = Reader::new(cursor, format)?;

    if let Some(Ok(entry)) = reader.read() {
        match entry.int_field("status") {
            Ok(_) => println!("  Unexpected success"),
            Err(e) => println!("  Error: {}", e),
        }
    }

    println!("\n");
    Ok(())
}
