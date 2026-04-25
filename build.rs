use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod build_support;

const GENERATED_DATASET: &str = "data/generated/dictionary.json";
const ALLOW_EMPTY_DATASET_ENV: &str = "OFFLINE_DICT_ALLOW_EMPTY_DATASET";

fn main() {
    println!("cargo:rerun-if-changed={GENERATED_DATASET}");
    println!("cargo:rerun-if-env-changed={ALLOW_EMPTY_DATASET_ENV}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should exist"));
    let embedded_dataset = out_dir.join("embedded_dictionary.json");

    let contents = build_support::load_dataset_for_embedding(
        Path::new(GENERATED_DATASET),
        allow_empty_dataset(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    fs::write(&embedded_dataset, contents).expect("should write embedded dictionary dataset");
}

fn allow_empty_dataset() -> bool {
    env::var(ALLOW_EMPTY_DATASET_ENV)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
