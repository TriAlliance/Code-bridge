//! Data format transformations (JSON, YAML, TOML, etc.)

use crate::{Format, Result, TransformError, TransformOptions};

/// Transform data between formats
pub fn transform_data(
    data: &[u8],
    from: Format,
    to: Format,
    options: &TransformOptions,
) -> Result<Vec<u8>> {
    let input = std::str::from_utf8(data)
        .map_err(|e| TransformError::Failed(e.to_string()))?;

    // Parse input to generic value
    let value = parse_to_value(input, from)?;

    // Serialize to output format
    let output = serialize_from_value(&value, to, options)?;

    Ok(output.into_bytes())
}

/// Parse input string to serde_json::Value
fn parse_to_value(input: &str, format: Format) -> Result<serde_json::Value> {
    match format {
        Format::Json => {
            serde_json::from_str(input).map_err(|e| TransformError::Json(e))
        }
        Format::Yaml => {
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(input)?;
            // Convert to JSON value for uniform handling
            serde_json::to_value(yaml_value).map_err(|e| TransformError::Json(e))
        }
        Format::Toml => {
            let toml_value: toml::Value = toml::from_str(input)
                .map_err(|e| TransformError::Toml(e.to_string()))?;
            serde_json::to_value(toml_value).map_err(|e| TransformError::Json(e))
        }
        _ => Err(TransformError::UnsupportedFormat(format!("{:?}", format))),
    }
}

/// Serialize serde_json::Value to output format
fn serialize_from_value(
    value: &serde_json::Value,
    format: Format,
    options: &TransformOptions,
) -> Result<String> {
    match format {
        Format::Json => {
            if options.pretty {
                serde_json::to_string_pretty(value).map_err(|e| TransformError::Json(e))
            } else {
                serde_json::to_string(value).map_err(|e| TransformError::Json(e))
            }
        }
        Format::Yaml => {
            serde_yaml::to_string(value).map_err(|e| TransformError::Yaml(e))
        }
        Format::Toml => {
            // TOML requires a table at the root
            if let serde_json::Value::Object(map) = value {
                let toml_value: toml::Value = serde_json::from_value(serde_json::Value::Object(map.clone()))
                    .map_err(|e| TransformError::Toml(e.to_string()))?;
                toml::to_string_pretty(&toml_value)
                    .map_err(|e| TransformError::Toml(e.to_string()))
            } else {
                Err(TransformError::Toml("TOML requires object at root".to_string()))
            }
        }
        _ => Err(TransformError::UnsupportedFormat(format!("{:?}", format))),
    }
}

/// Validate JSON
pub fn validate_json(input: &str) -> Result<()> {
    serde_json::from_str::<serde_json::Value>(input)?;
    Ok(())
}

/// Validate YAML
pub fn validate_yaml(input: &str) -> Result<()> {
    serde_yaml::from_str::<serde_yaml::Value>(input)?;
    Ok(())
}

/// Validate TOML
pub fn validate_toml(input: &str) -> Result<()> {
    toml::from_str::<toml::Value>(input)
        .map_err(|e| TransformError::Toml(e.to_string()))?;
    Ok(())
}

/// Minify JSON
pub fn minify_json(input: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    serde_json::to_string(&value).map_err(|e| TransformError::Json(e))
}

/// Pretty print JSON
pub fn prettify_json(input: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    serde_json::to_string_pretty(&value).map_err(|e| TransformError::Json(e))
}

/// Merge two JSON objects
pub fn merge_json(base: &str, overlay: &str) -> Result<String> {
    let mut base_value: serde_json::Value = serde_json::from_str(base)?;
    let overlay_value: serde_json::Value = serde_json::from_str(overlay)?;

    merge_values(&mut base_value, overlay_value);

    serde_json::to_string_pretty(&base_value).map_err(|e| TransformError::Json(e))
}

fn merge_values(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    merge_values(base_value, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}

/// Extract a path from JSON
pub fn json_path(input: &str, path: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(input)?;

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = &value;

    for part in parts {
        current = current.get(part)
            .ok_or_else(|| TransformError::Failed(format!("Path not found: {}", path)))?;
    }

    serde_json::to_string_pretty(current).map_err(|e| TransformError::Json(e))
}

/// Convert CSV to JSON array
pub fn csv_to_json(csv: &str) -> Result<String> {
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    let headers = reader.headers()
        .map_err(|e| TransformError::Failed(e.to_string()))?
        .clone();

    let mut records = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| TransformError::Failed(e.to_string()))?;
        let mut obj = serde_json::Map::new();

        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                obj.insert(
                    header.to_string(),
                    serde_json::Value::String(field.to_string()),
                );
            }
        }

        records.push(serde_json::Value::Object(obj));
    }

    serde_json::to_string_pretty(&records).map_err(|e| TransformError::Json(e))
}

/// Convert JSON array to CSV
pub fn json_to_csv(json: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(json)?;

    let array = value.as_array()
        .ok_or_else(|| TransformError::Failed("Expected JSON array".to_string()))?;

    if array.is_empty() {
        return Ok(String::new());
    }

    // Get headers from first object
    let first = array.first().unwrap();
    let headers: Vec<&String> = first.as_object()
        .ok_or_else(|| TransformError::Failed("Expected objects in array".to_string()))?
        .keys()
        .collect();

    let mut output = headers.iter().map(|h| h.as_str()).collect::<Vec<_>>().join(",");
    output.push('\n');

    for item in array {
        if let Some(obj) = item.as_object() {
            let row: Vec<String> = headers.iter()
                .map(|h| {
                    obj.get(*h)
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            output.push_str(&row.join(","));
            output.push('\n');
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_yaml() {
        let json = r#"{"name": "test", "value": 42}"#;
        let yaml = transform_data(
            json.as_bytes(),
            Format::Json,
            Format::Yaml,
            &TransformOptions::default(),
        ).unwrap();
        let yaml_str = String::from_utf8(yaml).unwrap();
        assert!(yaml_str.contains("name: test"));
    }

    #[test]
    fn test_yaml_to_json() {
        let yaml = "name: test\nvalue: 42";
        let json = transform_data(
            yaml.as_bytes(),
            Format::Yaml,
            Format::Json,
            &TransformOptions::default(),
        ).unwrap();
        let json_str = String::from_utf8(json).unwrap();
        assert!(json_str.contains("\"name\""));
    }

    #[test]
    fn test_merge_json() {
        let base = r#"{"a": 1, "b": {"c": 2}}"#;
        let overlay = r#"{"b": {"d": 3}, "e": 4}"#;
        let merged = merge_json(base, overlay).unwrap();

        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"]["c"], 2);
        assert_eq!(value["b"]["d"], 3);
        assert_eq!(value["e"], 4);
    }
}
