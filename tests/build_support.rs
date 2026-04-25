use std::fs;

#[path = "../build_support.rs"]
mod build_support;

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("offline-dict-cli-{nanos}-{name}"))
}

#[test]
fn missing_dataset_is_rejected_by_default() {
    let root = unique_temp_dir("missing-dataset");
    fs::create_dir_all(&root).expect("create temp root");
    let dataset_path = root.join("dictionary.json");

    let error = build_support::load_dataset_for_embedding(&dataset_path, false)
        .expect_err("missing generated dataset should fail release builds");

    fs::remove_dir_all(&root).ok();

    assert!(error.contains("generated dataset"));
    assert!(error.contains("cargo run --bin generate_dataset"));
}

#[test]
fn missing_dataset_can_be_explicitly_allowed_for_development() {
    let root = unique_temp_dir("allowed-empty-dataset");
    fs::create_dir_all(&root).expect("create temp root");
    let dataset_path = root.join("dictionary.json");

    let contents = build_support::load_dataset_for_embedding(&dataset_path, true)
        .expect("explicit development opt-in should allow an empty dataset");

    fs::remove_dir_all(&root).ok();

    assert_eq!(contents, "{\"entries\":[]}");
}

#[test]
fn existing_dataset_is_loaded_for_embedding() {
    let root = unique_temp_dir("existing-dataset");
    fs::create_dir_all(&root).expect("create temp root");
    let dataset_path = root.join("dictionary.json");
    fs::write(
        &dataset_path,
        "{\"entries\":[{\"headword\":\"abandon\",\"definitions\":[\"放弃\"],\"tags\":[]}]}",
    )
    .expect("write dataset");

    let contents = build_support::load_dataset_for_embedding(&dataset_path, false)
        .expect("existing generated dataset should be embedded");

    fs::remove_dir_all(&root).ok();

    assert!(contents.contains("abandon"));
    assert!(contents.contains("放弃"));
}

#[test]
fn existing_empty_dataset_is_rejected_by_default() {
    let root = unique_temp_dir("empty-dataset");
    fs::create_dir_all(&root).expect("create temp root");
    let dataset_path = root.join("dictionary.json");
    fs::write(&dataset_path, "{\"entries\":[]}").expect("write empty dataset");

    let error = build_support::load_dataset_for_embedding(&dataset_path, false)
        .expect_err("empty generated dataset should fail release builds");

    fs::remove_dir_all(&root).ok();

    assert!(error.contains("generated dataset"));
    assert!(error.contains("missing or empty"));
}
