use matrix_runner::{cli, init};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize i18n based on system locale
    init();

    // Build and parse CLI arguments
    let matches = cli::build_cli().get_matches();

    // Process the command
    match cli::process_command(matches).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
