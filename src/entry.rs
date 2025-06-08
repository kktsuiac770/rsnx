//! Core data structures for representing parsed log entries.

use crate::error::{Error, Result};
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type alias for the underlying field storage.
/// All field values are stored as strings, with type conversion on demand.
pub type Fields = HashMap<String, String>;

/// A parsed log entry containing field name-value pairs.
/// 
/// This is the primary data structure returned by log parsing operations.
/// All field values are stored as strings internally, with type conversion
/// methods available for accessing values as different types.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Entry {
    /// The underlying field storage.
    fields: Fields,
}

impl Entry {
    /// Create a new empty entry.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Create a new entry from a fields map.
    pub fn from_fields(fields: Fields) -> Self {
        Self { fields }
    }

    /// Get a field value as a string.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to retrieve
    /// 
    /// # Returns
    /// 
    /// The field value as a string, or an error if the field doesn't exist.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// # use rsnx::Entry;
    /// # use std::collections::HashMap;
    /// let mut fields = HashMap::new();
    /// fields.insert("status".to_string(), "200".to_string());
    /// let entry = Entry::from_fields(fields);
    /// 
    /// assert_eq!(entry.field("status").unwrap(), "200");
    /// assert!(entry.field("nonexistent").is_err());
    /// ```
    pub fn field(&self, name: &str) -> Result<&str> {
        self.fields
            .get(name)
            .map(|s| s.as_str())
            .ok_or_else(|| Error::field_not_found(name))
    }

    /// Get a field value as a float.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to retrieve and convert
    /// 
    /// # Returns
    /// 
    /// The field value as a f64, or an error if the field doesn't exist or cannot be parsed.
    pub fn float_field(&self, name: &str) -> Result<f64> {
        let value = self.field(name)?;
        value.parse::<f64>().map_err(|e| {
            Error::field_parse_error(name, value, "f64", e)
        })
    }

    /// Get a field value as a 64-bit integer.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to retrieve and convert
    /// 
    /// # Returns
    /// 
    /// The field value as an i64, or an error if the field doesn't exist or cannot be parsed.
    pub fn int64_field(&self, name: &str) -> Result<i64> {
        let value = self.field(name)?;
        value.parse::<i64>().map_err(|e| {
            Error::field_parse_error(name, value, "i64", e)
        })
    }

    /// Get a field value as a 32-bit integer.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to retrieve and convert
    /// 
    /// # Returns
    /// 
    /// The field value as an i32, or an error if the field doesn't exist or cannot be parsed.
    pub fn int_field(&self, name: &str) -> Result<i32> {
        let value = self.field(name)?;
        value.parse::<i32>().map_err(|e| {
            Error::field_parse_error(name, value, "i32", e)
        })
    }

    /// Set a field value as a string.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to set
    /// * `value` - The string value to store
    pub fn set_field(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(name.into(), value.into());
    }

    /// Set a field value from a float.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to set
    /// * `value` - The float value to convert and store
    pub fn set_float_field(&mut self, name: impl Into<String>, value: f64) {
        self.fields.insert(name.into(), format!("{:.2}", value));
    }

    /// Set a field value from an unsigned integer.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The field name to set
    /// * `value` - The unsigned integer value to convert and store
    pub fn set_uint_field(&mut self, name: impl Into<String>, value: u64) {
        self.fields.insert(name.into(), value.to_string());
    }

    /// Merge another entry into this one.
    /// 
    /// All fields from the other entry will be copied into this entry,
    /// overwriting any existing fields with the same name.
    /// 
    /// # Arguments
    /// 
    /// * `other` - The entry to merge into this one
    pub fn merge(&mut self, other: &Entry) {
        for (key, value) in &other.fields {
            self.fields.insert(key.clone(), value.clone());
        }
    }

    /// Create a hash string from specified fields.
    /// 
    /// This creates a deterministic string representation of the specified fields,
    /// useful for grouping operations. Missing fields are represented as "NULL".
    /// 
    /// # Arguments
    /// 
    /// * `field_names` - The field names to include in the hash
    /// 
    /// # Returns
    /// 
    /// A semicolon-separated string in the format 'field1'=value1;'field2'=value2
    pub fn fields_hash(&self, field_names: &[&str]) -> String {
        field_names
            .iter()
            .map(|&name| {
                let value = self.fields.get(name).map(|s| s.as_str()).unwrap_or("NULL");
                format!("'{}'={}", name, value)
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Create a partial entry containing only specified fields.
    /// 
    /// This creates a new entry with only the specified fields copied from this entry.
    /// Missing fields will be included with empty string values.
    /// 
    /// # Arguments
    /// 
    /// * `field_names` - The field names to include in the partial entry
    /// 
    /// # Returns
    /// 
    /// A new entry containing only the specified fields
    pub fn partial(&self, field_names: &[&str]) -> Entry {
        let mut fields = HashMap::new();
        for &name in field_names {
            let value = self.fields.get(name).cloned().unwrap_or_default();
            fields.insert(name.to_string(), value);
        }
        Entry::from_fields(fields)
    }

    /// Get an iterator over all field names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.fields.iter()
    }

    /// Get the number of fields in this entry.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if this entry has no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get a reference to the underlying fields map.
    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    /// Get a mutable reference to the underlying fields map.
    pub fn fields_mut(&mut self) -> &mut Fields {
        &mut self.fields
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Fields> for Entry {
    fn from(fields: Fields) -> Self {
        Self::from_fields(fields)
    }
}

impl From<Entry> for Fields {
    fn from(entry: Entry) -> Self {
        entry.fields
    }
}
