use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("offline-dict-cli-{nanos}-{name}"))
}

fn fixture_json() -> String {
    serde_json::json!({
        "entries": [
            {
                "headword": "abandon",
                "definitions": ["放弃", "遗弃", "沉湎于"],
                "tags": ["CET4", "CET6"]
            },
            {
                "headword": "quit",
                "definitions": ["放弃"],
                "tags": ["COMMON_3500"]
            },
            {
                "headword": "renounce",
                "definitions": ["放弃"],
                "tags": ["GRE"]
            },
            {
                "headword": "give up",
                "definitions": ["放弃"],
                "tags": []
            }
        ]
    })
    .to_string()
}

#[test]
fn empty_input_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_dict"))
        .output()
        .expect("binary should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("dict [--all] <query>"));
}

#[test]
fn missing_query_prints_not_found_and_exits_non_zero() {
    let dataset_path = unique_temp_file("fixture.json");
    fs::write(&dataset_path, fixture_json()).expect("write fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_dict"))
        .env("OFFLINE_DICT_DATASET", &dataset_path)
        .arg("missing")
        .output()
        .expect("binary should run");

    fs::remove_file(&dataset_path).ok();

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("未找到精确匹配: missing"));
}

#[test]
fn all_flag_expands_reverse_lookup_results() {
    let dataset_path = unique_temp_file("fixture.json");
    fs::write(&dataset_path, fixture_json()).expect("write fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_dict"))
        .env("OFFLINE_DICT_DATASET", &dataset_path)
        .args(["--all", "放弃"])
        .output()
        .expect("binary should run");

    fs::remove_file(&dataset_path).ok();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("1. quit"));
    assert!(stdout.contains("4. give up"));
    assert!(!stdout.contains("5 of"));
}
