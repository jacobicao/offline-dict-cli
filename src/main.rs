use std::process::ExitCode;

use offline_dict_cli::{execute_command, parse_command, runtime_config_from_env, SystemClock};

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
    let command = parse_command(std::env::args().skip(1))?;
    let runtime = runtime_config_from_env();
    let output = execute_command(command, &runtime, &SystemClock)?;

    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }

    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }

    Ok(ExitCode::from(output.exit_code))
}
