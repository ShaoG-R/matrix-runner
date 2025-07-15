use matrix_runner::cli;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Parse command line arguments
    let cli_args = cli::parse_args();
    
    // Process the command
    match cli::process_command(cli_args).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
