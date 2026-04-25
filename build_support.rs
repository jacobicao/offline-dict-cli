use std::fs;
use std::path::Path;

const EMPTY_DATASET: &str = "{\"entries\":[]}";

pub fn load_dataset_for_embedding(path: &Path, allow_empty: bool) -> Result<String, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if allow_empty {
                return Ok(EMPTY_DATASET.to_string());
            }
            return Err(missing_dataset_message(path));
        }
        Err(error) => {
            return Err(format!(
                "failed to read generated dataset {}: {error}",
                path.display()
            ));
        }
    };

    if dataset_has_entries(&contents)? || allow_empty {
        Ok(contents)
    } else {
        Err(missing_dataset_message(path))
    }
}

fn dataset_has_entries(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|error| format!("failed to parse generated dataset: {error}"))?;
    let Some(entries) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Err("generated dataset must contain an 'entries' array".to_string());
    };

    Ok(!entries.is_empty())
}

fn missing_dataset_message(path: &Path) -> String {
    format!(
        "generated dataset {} is missing or empty; run `cargo run --bin generate_dataset -- <source-root>` before building, or set OFFLINE_DICT_ALLOW_EMPTY_DATASET=1 for development-only empty builds",
        path.display()
    )
}
