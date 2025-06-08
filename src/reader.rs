//! Log file reading and iteration functionality.

use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::parser::{Parser, StringParser};
use std::io::{BufRead, BufReader, Read};

/// A reader that parses log files line by line using a specified format.
///
/// The reader implements the Iterator trait, allowing you to process log entries
/// using standard Rust iterator patterns.
#[derive(Debug)]
pub struct Reader<R: Read> {
    /// The underlying buffered reader.
    reader: BufReader<R>,
    /// The parser for converting lines to entries.
    parser: Parser,
}

impl<R: Read> Reader<R> {
    /// Create a new reader with the specified input source and format string.
    ///
    /// # Arguments
    ///
    /// * `input` - The input source (file, stdin, etc.)
    /// * `format` - The log format string (e.g., `$remote_addr [$time_local] "$request"`)
    ///
    /// # Returns
    ///
    /// A new reader instance, or an error if the format string is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsnx::Reader;
    /// use std::io::Cursor;
    ///
    /// let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1""#;
    /// let format = r#"$remote_addr [$time_local] "$request""#;
    ///
    /// let cursor = Cursor::new(log_data);
    /// let reader = Reader::new(cursor, format)?;
    /// # Ok::<(), rsnx::Error>(())
    /// ```
    pub fn new(input: R, format: &str) -> Result<Self> {
        let parser = Parser::new(format)?;
        Ok(Self {
            reader: BufReader::new(input),
            parser,
        })
    }

    /// Create a new reader with a custom parser.
    ///
    /// This allows you to use a pre-configured parser or a custom parser implementation.
    ///
    /// # Arguments
    ///
    /// * `input` - The input source
    /// * `parser` - The parser to use for converting lines to entries
    pub fn with_parser(input: R, parser: Parser) -> Self {
        Self {
            reader: BufReader::new(input),
            parser,
        }
    }

    /// Get a reference to the underlying parser.
    pub fn parser(&self) -> &Parser {
        &self.parser
    }

    /// Read the next entry from the log file.
    ///
    /// This method reads one line from the input and parses it into an Entry.
    /// It returns `None` when the end of the file is reached.
    ///
    /// # Returns
    ///
    /// An `Option<Result<Entry>>` where:
    /// - `None` indicates end of file
    /// - `Some(Ok(entry))` indicates a successfully parsed entry
    /// - `Some(Err(error))` indicates a parsing or I/O error
    pub fn read(&mut self) -> Option<Result<Entry>> {
        let mut line = String::new();

        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => {
                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }

                // Skip empty lines
                if line.trim().is_empty() {
                    return self.read(); // Recursively read next line
                }

                Some(self.parser.parse_string(&line))
            }
            Err(e) => Some(Err(Error::Io { source: e })),
        }
    }

    /// Collect all entries into a vector.
    ///
    /// This is a convenience method that reads all entries from the log file
    /// and collects them into a vector. Use this for smaller files or when
    /// you need to process all entries at once.
    ///
    /// # Returns
    ///
    /// A vector of all entries, or an error if any line fails to parse.
    pub fn collect_all(mut self) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();

        while let Some(result) = self.read() {
            entries.push(result?);
        }

        Ok(entries)
    }

    /// Process entries with a closure.
    ///
    /// This method allows you to process entries one by one without collecting
    /// them all into memory. This is more memory-efficient for large files.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that processes each entry
    ///
    /// # Returns
    ///
    /// An error if any line fails to parse or if the closure returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsnx::Reader;
    /// use std::io::Cursor;
    ///
    /// let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200"#;
    /// let format = r#"$remote_addr [$time_local] "$request" $status"#;
    ///
    /// let cursor = Cursor::new(log_data);
    /// let mut reader = Reader::new(cursor, format)?;
    ///
    /// reader.process_entries(|entry| -> Result<(), Box<dyn std::error::Error>> {
    ///     println!("IP: {}", entry.field("remote_addr")?);
    ///     Ok(())
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn process_entries<F, E>(&mut self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(&Entry) -> std::result::Result<(), E>,
        E: From<Error>,
    {
        while let Some(result) = self.read() {
            let entry = result?;
            f(&entry)?;
        }
        Ok(())
    }
}

impl<R: Read> Iterator for Reader<R> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_reader_basic() {
        let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612"#;
        let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

        let cursor = Cursor::new(log_data);
        let mut reader = Reader::new(cursor, format).unwrap();

        let entry = reader.read().unwrap().unwrap();
        assert_eq!(entry.field("remote_addr").unwrap(), "127.0.0.1");
        assert_eq!(entry.field("status").unwrap(), "200");

        // Should be EOF now
        assert!(reader.read().is_none());
    }

    #[test]
    fn test_reader_iterator() {
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
    }

    #[test]
    fn test_reader_empty_lines() {
        let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612

192.168.1.1 [08/Nov/2013:13:40:18 +0000] "POST /api/bar HTTP/1.1" 404 0"#;
        let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

        let cursor = Cursor::new(log_data);
        let reader = Reader::new(cursor, format).unwrap();

        let entries: Result<Vec<_>> = reader.collect();
        let entries = entries.unwrap();

        // Empty line should be skipped
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_collect_all() {
        let log_data = r#"127.0.0.1 [08/Nov/2013:13:39:18 +0000] "GET /api/foo HTTP/1.1" 200 612
192.168.1.1 [08/Nov/2013:13:40:18 +0000] "POST /api/bar HTTP/1.1" 404 0"#;
        let format = r#"$remote_addr [$time_local] "$request" $status $body_bytes_sent"#;

        let cursor = Cursor::new(log_data);
        let reader = Reader::new(cursor, format).unwrap();

        let entries = reader.collect_all().unwrap();
        assert_eq!(entries.len(), 2);
    }
}
