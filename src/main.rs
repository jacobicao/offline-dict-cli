use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use offline_dict_cli::{format_result, help_text, version_text, Dictionary, LookupError};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut show_all = false;
    let mut positionals = Vec::new();

    for argument in env::args().skip(1) {
        match argument.as_str() {
            "--all" => show_all = true,
            "--help" => {
                print!("{}", help_text());
                return Ok(ExitCode::SUCCESS);
            }
            "--version" => {
                println!("{}", version_text());
                return Ok(ExitCode::SUCCESS);
            }
            flag if flag.starts_with('-') => {
                return Err(format!("error: unexpected argument '{flag}' found"));
            }
            _ => positionals.push(argument),
        }
    }

    if positionals.is_empty() {
        print!("{}", help_text());
        return Ok(ExitCode::SUCCESS);
    }

    let query = positionals.join(" ");
    let dictionary = load_dictionary()?;

    match dictionary.lookup(&query, show_all) {
        Ok(result) => {
            println!("{}", format_result(&result, show_all));
            Ok(ExitCode::SUCCESS)
        }
        Err(LookupError::EmptyQuery) => {
            print!("{}", help_text());
            Ok(ExitCode::SUCCESS)
        }
        Err(LookupError::NotFound { .. }) => {
            println!("未找到精确匹配: {query}");
            Ok(ExitCode::from(1))
        }
    }
}

fn load_dictionary() -> Result<Dictionary, String> {
    if let Some(path) = env::var_os("OFFLINE_DICT_DATASET") {
        return Dictionary::from_json_path(&PathBuf::from(path));
    }

    Dictionary::embedded()
}
