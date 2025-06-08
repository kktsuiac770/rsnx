//! Log format parsing and regex generation.

use crate::entry::Entry;
use crate::error::{Error, Result};
use regex::Regex;
use std::collections::HashMap;

/// Trait for parsing log lines into entries.
pub trait StringParser {
    /// Parse a log line into an entry.
    fn parse_string(&self, line: &str) -> Result<Entry>;
}

/// A parser that converts log format strings into regex patterns for parsing log lines.
///
/// The parser takes format strings like `$remote_addr [$time_local] "$request"` and
/// converts them into regular expressions that can extract named fields from log lines.
#[derive(Debug, Clone)]
pub struct Parser {
    /// The original format string.
    format: String,
    /// The compiled regular expression for parsing.
    regex: Regex,
}

impl Parser {
    /// Create a new parser from a format string.
    ///
    /// Format strings use `$field_name` syntax to define extractable fields.
    /// The parser automatically handles field boundaries and generates appropriate
    /// regex patterns.
    ///
    /// # Arguments
    ///
    /// * `format` - The log format string (e.g., `$remote_addr [$time_local] "$request"`)
    ///
    /// # Returns
    ///
    /// A new parser instance, or an error if the format string is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rsnx::Parser;
    ///
    /// let parser = Parser::new(r#"$remote_addr [$time_local] "$request" $status"#)?;
    /// # Ok::<(), rsnx::Error>(())
    /// ```
    pub fn new(format: &str) -> Result<Self> {
        let regex_pattern = Self::format_to_regex(format)?;
        let regex = Regex::new(&regex_pattern).map_err(|e| Error::invalid_format(format, e))?;

        Ok(Self {
            format: format.to_string(),
            regex,
        })
    }

    /// Get the original format string.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Get the compiled regex pattern.
    pub fn regex(&self) -> &Regex {
        &self.regex
    }

    /// Convert a format string to a regex pattern.
    ///
    /// This method handles the complex transformation from nginx-style format strings
    /// to regex patterns with named capture groups.
    fn format_to_regex(format: &str) -> Result<String> {
        let mut result = format.to_string();

        // Step 1: Handle concatenated fields by inserting temporary placeholders
        // This ensures proper field boundaries when fields are adjacent
        result = Self::handle_concatenated_fields(&result);

        // Step 2: Convert field tokens to named regex groups
        result = Self::convert_fields_to_groups(&result)?;

        // Step 3: Clean up temporary placeholders
        result = result.replace("@RSNX@", "");

        // Step 4: Anchor the regex to match from the beginning of the line
        Ok(format!("^{}$", result))
    }

    /// Handle concatenated fields by inserting placeholders.
    ///
    /// When fields are concatenated without separators (like $host$request_uri),
    /// we need to insert temporary placeholders to ensure proper field boundaries.
    fn handle_concatenated_fields(format: &str) -> String {
        // Find patterns where one field immediately follows another
        let field_pattern = Regex::new(r"\$\w+").unwrap();
        let mut result = String::new();
        let mut last_end = 0;

        let matches: Vec<_> = field_pattern.find_iter(format).collect();

        for (i, m) in matches.iter().enumerate() {
            // Add text between previous match and current match
            result.push_str(&format[last_end..m.start()]);

            // Add the current field
            result.push_str(m.as_str());

            // If the next match immediately follows this one, insert a placeholder
            if i + 1 < matches.len() && matches[i + 1].start() == m.end() {
                result.push_str("@RSNX@");
            }

            last_end = m.end();
        }

        // Add remaining text
        result.push_str(&format[last_end..]);
        result
    }

    /// Convert field tokens to named regex groups.
    ///
    /// This converts `$field_name` tokens to `(?P<field_name>[^delimiter]*)` patterns.
    fn convert_fields_to_groups(format: &str) -> Result<String> {
        let field_pattern = Regex::new(r"\$(\w+)").unwrap();
        let mut result = String::new();
        let mut last_end = 0;

        for captures in field_pattern.captures_iter(format) {
            let full_match = captures.get(0).unwrap();
            let field_name = captures.get(1).unwrap().as_str();

            // Add text before this field
            result.push_str(&format[last_end..full_match.start()]);

            // Determine the delimiter for this field
            let delimiter = Self::determine_delimiter(format, full_match.end());

            // Create the named capture group
            let group = if delimiter.is_empty() {
                // No delimiter, match everything to end of line
                format!("(?P<{}>.*)", field_name)
            } else {
                // Check if this field is followed by a placeholder (indicating concatenation)
                let remaining_after_field = &format[full_match.end()..];
                if remaining_after_field.starts_with("@RSNX@") {
                    // This field is concatenated with the next one
                    // Use a more specific pattern based on common field types
                    if field_name == "host" {
                        // Host is typically a domain name
                        format!("(?P<{}>[a-zA-Z0-9.-]+)", field_name)
                    } else {
                        // For other fields, use non-greedy matching
                        format!("(?P<{}>[^{}]*?)", field_name, regex::escape(&delimiter))
                    }
                } else {
                    // Normal field with delimiter
                    format!("(?P<{}>[^{}]*)", field_name, regex::escape(&delimiter))
                }
            };

            result.push_str(&group);
            last_end = full_match.end();
        }

        // Add remaining text
        result.push_str(&format[last_end..]);

        // Escape special regex characters in the non-field parts
        Ok(Self::escape_non_field_parts(&result))
    }

    /// Determine the delimiter character that follows a field.
    fn determine_delimiter(format: &str, field_end: usize) -> String {
        if field_end >= format.len() {
            return String::new();
        }

        let remaining = &format[field_end..];

        // Skip placeholder if present and look for the next delimiter
        if let Some(after_placeholder) = remaining.strip_prefix("@RSNX@") {
            // Find the next field and look past it for the delimiter
            let field_pattern = Regex::new(r"\$\w+").unwrap();
            if let Some(next_field_match) = field_pattern.find(after_placeholder) {
                let after_next_field = &after_placeholder[next_field_match.end()..];
                if after_next_field.is_empty() {
                    return String::new();
                }
                return after_next_field.chars().next().unwrap().to_string();
            } else {
                // No next field, check what's immediately after placeholder
                if after_placeholder.is_empty() {
                    return String::new();
                }
                return after_placeholder.chars().next().unwrap().to_string();
            }
        }

        // Return the first character as delimiter
        remaining.chars().next().unwrap_or_default().to_string()
    }

    /// Escape special regex characters in non-field parts of the format.
    fn escape_non_field_parts(format: &str) -> String {
        let group_pattern = Regex::new(r"\(\?P<\w+>[^)]*\)").unwrap();
        let mut result = String::new();
        let mut last_end = 0;

        for m in group_pattern.find_iter(format) {
            // Escape the text before this group
            let before = &format[last_end..m.start()];
            result.push_str(&regex::escape(before));

            // Add the group as-is
            result.push_str(m.as_str());

            last_end = m.end();
        }

        // Escape remaining text
        let remaining = &format[last_end..];
        result.push_str(&regex::escape(remaining));

        result
    }
}

impl StringParser for Parser {
    /// Parse a log line into an entry using the compiled regex.
    fn parse_string(&self, line: &str) -> Result<Entry> {
        let captures = self
            .regex
            .captures(line)
            .ok_or_else(|| Error::line_format_mismatch(line, &self.format))?;

        let mut fields = HashMap::new();

        // Extract all named capture groups
        for name in self.regex.capture_names().flatten() {
            if let Some(value) = captures.name(name) {
                fields.insert(name.to_string(), value.as_str().to_string());
            }
        }

        Ok(Entry::from_fields(fields))
    }
}
