//! Nginx configuration parsing functionality.

use crate::error::{Error, Result};
use crate::parser::Parser;
use crate::reader::Reader;
use regex::Regex;
use std::io::{BufRead, BufReader, Read};

/// A reader that extracts log formats from nginx configuration files.
///
/// This reader can parse nginx configuration files to extract log_format
/// definitions and use them to parse log files automatically.
#[derive(Debug)]
pub struct NginxReader<R: Read> {
    /// The underlying reader.
    reader: Reader<R>,
}

impl<R: Read> NginxReader<R> {
    /// Create a new nginx reader by extracting the format from nginx configuration.
    ///
    /// This function parses the nginx configuration to find the specified log format
    /// and uses it to create a reader for parsing log files.
    ///
    /// # Arguments
    ///
    /// * `log_input` - The log file input source
    /// * `nginx_config` - The nginx configuration input source
    /// * `format_name` - The name of the log format to extract (e.g., "main", "combined")
    ///
    /// # Returns
    ///
    /// A new nginx reader instance, or an error if the format is not found or invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsnx::NginxReader;
    /// use std::io::Cursor;
    ///
    /// let nginx_config = r#"
    /// log_format main '$remote_addr - $remote_user [$time_local] "$request" '
    ///                 '$status $body_bytes_sent "$http_referer" '
    ///                 '"$http_user_agent" "$http_x_forwarded_for"';
    /// "#;
    ///
    /// let log_data = r#"127.0.0.1 - - [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612 "-" "curl/7.64.1" "-""#;
    ///
    /// let config_cursor = Cursor::new(nginx_config);
    /// let log_cursor = Cursor::new(log_data);
    ///
    /// let reader = NginxReader::new(log_cursor, config_cursor, "main")?;
    /// # Ok::<(), rsnx::Error>(())
    /// ```
    pub fn new<C: Read>(log_input: R, nginx_config: C, format_name: &str) -> Result<Self> {
        let format = extract_nginx_format(nginx_config, format_name)?;
        let parser = Parser::new(&format)?;
        let reader = Reader::with_parser(log_input, parser);

        Ok(Self { reader })
    }

    /// Get a reference to the underlying reader.
    pub fn reader(&self) -> &Reader<R> {
        &self.reader
    }

    /// Get a mutable reference to the underlying reader.
    pub fn reader_mut(&mut self) -> &mut Reader<R> {
        &mut self.reader
    }

    /// Read the next entry from the log file.
    pub fn read(&mut self) -> Option<Result<crate::entry::Entry>> {
        self.reader.read()
    }

    /// Collect all entries into a vector.
    pub fn collect_all(self) -> Result<Vec<crate::entry::Entry>> {
        self.reader.collect_all()
    }
}

impl<R: Read> Iterator for NginxReader<R> {
    type Item = Result<crate::entry::Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next()
    }
}

/// Extract a log format from nginx configuration.
///
/// This function parses nginx configuration to find a log_format directive
/// with the specified name and returns the format string.
///
/// # Arguments
///
/// * `nginx_config` - The nginx configuration input source
/// * `format_name` - The name of the log format to extract
///
/// # Returns
///
/// The format string, or an error if the format is not found.
pub fn extract_nginx_format<R: Read>(nginx_config: R, format_name: &str) -> Result<String> {
    let reader = BufReader::new(nginx_config);

    // Regex to match log_format directive
    let log_format_regex = Regex::new(&format!(
        r"^\s*log_format\s+{}\s+(.+)",
        regex::escape(format_name)
    ))
    .unwrap();

    let mut format_lines = Vec::new();
    let mut in_format = false;
    let mut brace_count = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !in_format {
            // Look for the start of our log format
            if let Some(captures) = log_format_regex.captures(trimmed) {
                let format_part = captures.get(1).unwrap().as_str();
                format_lines.push(format_part.to_string());
                in_format = true;

                // Count braces to handle multi-line formats
                brace_count += count_braces(format_part);

                // If the line ends with semicolon and braces are balanced, we're done
                if format_part.trim_end().ends_with(';') && brace_count == 0 {
                    break;
                }
            }
        } else {
            // Continue collecting format lines
            format_lines.push(trimmed.to_string());
            brace_count += count_braces(trimmed);

            // If the line ends with semicolon and braces are balanced, we're done
            if trimmed.ends_with(';') && brace_count == 0 {
                break;
            }
        }
    }

    if format_lines.is_empty() {
        return Err(Error::nginx_format_not_found(format_name));
    }

    // Process each line to remove quotes and join
    let mut processed_lines = Vec::new();
    for line in format_lines.iter() {
        let mut trimmed = line.trim();

        // Remove trailing semicolon if present
        if trimmed.ends_with(';') {
            trimmed = &trimmed[..trimmed.len() - 1];
        }

        let cleaned = remove_surrounding_quotes(trimmed);
        processed_lines.push(cleaned);
    }

    // Join all format lines
    let mut format = processed_lines.join(" ");

    // Remove trailing semicolon if present
    if format.ends_with(';') {
        format.pop();
    }

    // Remove surrounding quotes from the entire format if present
    format = remove_surrounding_quotes(&format);

    // Simple whitespace cleanup - just normalize spaces
    format = format.split_whitespace().collect::<Vec<_>>().join(" ");

    Ok(format)
}

/// Count opening and closing braces in a string.
/// Returns the net count (opening - closing).
fn count_braces(s: &str) -> i32 {
    let mut count = 0;
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in s.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => escape_next = true,
            '"' | '\'' => in_quotes = !in_quotes,
            '{' if !in_quotes => count += 1,
            '}' if !in_quotes => count -= 1,
            _ => {}
        }
    }

    count
}

/// Remove surrounding quotes from a string.
fn remove_surrounding_quotes(s: &str) -> String {
    let trimmed = s.trim();

    // Only remove one layer of matching quotes
    if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        && trimmed.len() >= 2
    {
        return trimmed[1..trimmed.len() - 1].to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_extract_simple_format() {
        let config = r#"
        log_format main '$remote_addr - $remote_user [$time_local] "$request" $status';
        "#;

        let cursor = Cursor::new(config);
        let format = extract_nginx_format(cursor, "main").unwrap();

        assert_eq!(
            format,
            r#"$remote_addr - $remote_user [$time_local] "$request" $status"#
        );
    }

    #[test]
    fn test_extract_multiline_format() {
        let config = r#"
        log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                        '$status $body_bytes_sent "$http_referer" '
                        '"$http_user_agent" "$http_x_forwarded_for"';
        "#;

        let cursor = Cursor::new(config);
        let format = extract_nginx_format(cursor, "main").unwrap();

        let expected = r#"$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent" "$http_x_forwarded_for""#;
        assert_eq!(format, expected);
    }

    #[test]
    fn test_format_not_found() {
        let config = r#"
        log_format main '$remote_addr - $remote_user [$time_local]';
        "#;

        let cursor = Cursor::new(config);
        let result = extract_nginx_format(cursor, "nonexistent");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::NginxFormatNotFound { .. }
        ));
    }

    #[test]
    fn test_nginx_reader() {
        let config = r#"
        log_format main '$remote_addr - $remote_user [$time_local] "$request" $status';
        "#;

        let log_data = r#"127.0.0.1 - - [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200"#;

        let config_cursor = Cursor::new(config);
        let log_cursor = Cursor::new(log_data);

        let mut reader = NginxReader::new(log_cursor, config_cursor, "main").unwrap();

        let entry = reader.read().unwrap().unwrap();
        assert_eq!(entry.field("remote_addr").unwrap(), "127.0.0.1");
        assert_eq!(entry.field("status").unwrap(), "200");
    }
}
