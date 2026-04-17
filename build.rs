use std::env;
use std::fs;
use std::path::PathBuf;

const GENERATED_DATASET: &str = "data/generated/dictionary.json";
const EMPTY_DATASET: &str = "{\"entries\":[]}";

fn main() {
    println!("cargo:rerun-if-changed={GENERATED_DATASET}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should exist"));
    let embedded_dataset = out_dir.join("embedded_dictionary.json");

    let contents = fs::read_to_string(GENERATED_DATASET).unwrap_or_else(|_| EMPTY_DATASET.to_string());
    fs::write(&embedded_dataset, contents).expect("should write embedded dictionary dataset");
}
