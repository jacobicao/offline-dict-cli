use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use offline_dict_cli::importer::{
    build_persisted_dictionary, load_source_documents_from_directory,
};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<String, String> {
    let mut args = env::args_os().skip(1);
    let Some(source_root) = args.next() else {
        return Err(
            "Usage: cargo run --bin generate_dataset -- <source-root> [output-path]".to_string(),
        );
    };

    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/generated/dictionary.json"));

    if args.next().is_some() {
        return Err(
            "Usage: cargo run --bin generate_dataset -- <source-root> [output-path]".to_string(),
        );
    }

    let documents = load_source_documents_from_directory(&PathBuf::from(&source_root))?;
    let persisted = build_persisted_dictionary(&documents);
    let json = serde_json::to_vec(&persisted)
        .map_err(|error| format!("failed to serialize dataset: {error}"))?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create output directory {}: {error}",
                parent.display()
            )
        })?;
    }

    fs::write(&output_path, json).map_err(|error| {
        format!(
            "failed to write generated dataset {}: {error}",
            output_path.display()
        )
    })?;

    Ok(format!(
        "generated {} entries into {}",
        persisted.entries.len(),
        output_path.display()
    ))
}
