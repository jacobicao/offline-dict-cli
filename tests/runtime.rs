use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use offline_dict_cli::{execute_command, Clock, Command, RuntimeConfig};

struct FixedClock {
    today: NaiveDate,
}

impl FixedClock {
    fn new(date: &str) -> Self {
        Self {
            today: NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("valid date"),
        }
    }
}

impl Clock for FixedClock {
    fn today_local(&self) -> NaiveDate {
        self.today
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("offline-dict-cli-{nanos}-{name}"))
}

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
                "headword": "search-log",
                "definitions": ["日志搜索"],
                "tags": ["GRE"]
            }
        ]
    })
    .to_string()
}

fn test_runtime(dataset_path: Option<PathBuf>, history_dir: Option<PathBuf>) -> RuntimeConfig {
    RuntimeConfig {
        dataset_path,
        history_dir,
        local_app_data: None,
    }
}

#[test]
fn search_log_defaults_to_last_seven_days_and_deduplicates_lines() {
    let history_dir = unique_temp_dir("history-default");
    fs::create_dir_all(&history_dir).expect("create history dir");
    fs::write(
        history_dir.join("2026-04-18.txt"),
        "abandon\napple\nabandon\n \n放弃\n",
    )
    .expect("write day file");
    fs::write(history_dir.join("2026-04-14.txt"), "focus\nfocus\n").expect("write day file");

    let output = execute_command(
        Command::SearchLog {
            from: None,
            to: None,
        },
        &test_runtime(None, Some(history_dir.clone())),
        &FixedClock::new("2026-04-18"),
    )
    .expect("command should succeed");

    fs::remove_dir_all(&history_dir).ok();

    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        "2026-04-18\n1. abandon\n2. apple\n\n2026-04-17\n(no queries)\n\n2026-04-16\n(no queries)\n\n2026-04-15\n(no queries)\n\n2026-04-14\n1. focus\n\n2026-04-13\n(no queries)\n\n2026-04-12\n(no queries)\n"
    );
}

#[test]
fn search_log_reads_explicit_inclusive_range_newest_first() {
    let history_dir = unique_temp_dir("history-range");
    fs::create_dir_all(&history_dir).expect("create history dir");
    fs::write(history_dir.join("2026-04-10.txt"), "alpha\n").expect("write day file");
    fs::write(history_dir.join("2026-04-11.txt"), "beta\n").expect("write day file");

    let output = execute_command(
        Command::SearchLog {
            from: Some("2026-04-10".to_string()),
            to: Some("2026-04-11".to_string()),
        },
        &test_runtime(None, Some(history_dir.clone())),
        &FixedClock::new("2026-04-18"),
    )
    .expect("command should succeed");

    fs::remove_dir_all(&history_dir).ok();

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "2026-04-11\n1. beta\n\n2026-04-10\n1. alpha\n"
    );
}

#[test]
fn search_log_treats_missing_directory_as_empty_history() {
    let history_dir = unique_temp_dir("history-missing");

    let output = execute_command(
        Command::SearchLog {
            from: Some("2026-04-17".to_string()),
            to: Some("2026-04-18".to_string()),
        },
        &test_runtime(None, Some(history_dir.clone())),
        &FixedClock::new("2026-04-18"),
    )
    .expect("command should succeed");

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "2026-04-18\n(no queries)\n\n2026-04-17\n(no queries)\n"
    );
}

#[test]
fn successful_english_lookup_records_once_per_day() {
    let dataset_path = unique_temp_file("fixture.json");
    let history_dir = unique_temp_dir("history-write");
    fs::write(&dataset_path, fixture_json()).expect("write dataset");

    let runtime = test_runtime(Some(dataset_path.clone()), Some(history_dir.clone()));
    let clock = FixedClock::new("2026-04-18");

    let first = execute_command(
        Command::Lookup {
            query: "Abandon".to_string(),
            show_all: false,
        },
        &runtime,
        &clock,
    )
    .expect("lookup should succeed");
    let duplicate = execute_command(
        Command::Lookup {
            query: "abandon".to_string(),
            show_all: false,
        },
        &runtime,
        &clock,
    )
    .expect("lookup should succeed");
    let chinese = execute_command(
        Command::Lookup {
            query: "放弃".to_string(),
            show_all: false,
        },
        &runtime,
        &clock,
    )
    .expect("reverse lookup should succeed");
    let missing = execute_command(
        Command::Lookup {
            query: "missing".to_string(),
            show_all: false,
        },
        &runtime,
        &clock,
    )
    .expect("missing lookup should still return an output");

    let contents = fs::read_to_string(history_dir.join("2026-04-18.txt")).expect("read history");

    fs::remove_file(&dataset_path).ok();
    fs::remove_dir_all(&history_dir).ok();

    assert_eq!(first.exit_code, 0);
    assert_eq!(duplicate.exit_code, 0);
    assert_eq!(chinese.exit_code, 0);
    assert_eq!(missing.exit_code, 1);
    assert_eq!(contents, "abandon\n");
}

#[test]
fn lookup_warns_but_succeeds_when_history_write_fails() {
    let dataset_path = unique_temp_file("fixture.json");
    let history_path = unique_temp_file("history-file.txt");
    fs::write(&dataset_path, fixture_json()).expect("write dataset");
    fs::write(&history_path, "not a directory").expect("write file");

    let output = execute_command(
        Command::Lookup {
            query: "abandon".to_string(),
            show_all: false,
        },
        &test_runtime(Some(dataset_path.clone()), Some(history_path.clone())),
        &FixedClock::new("2026-04-18"),
    )
    .expect("lookup should succeed");

    fs::remove_file(&dataset_path).ok();
    fs::remove_file(&history_path).ok();

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("abandon"));
    assert!(output.stderr.contains("warning:"));
    assert!(output.stderr.contains("history"));
}

#[test]
fn search_log_errors_when_history_override_points_to_file() {
    let history_path = unique_temp_file("history-file.txt");
    fs::write(&history_path, "not a directory").expect("write file");

    let error = execute_command(
        Command::SearchLog {
            from: None,
            to: None,
        },
        &test_runtime(None, Some(history_path.clone())),
        &FixedClock::new("2026-04-18"),
    )
    .expect_err("search-log should fail");

    fs::remove_file(&history_path).ok();

    assert!(error.contains("history"));
    assert!(error.contains("directory"));
}
