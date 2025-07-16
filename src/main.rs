use matrix_runner::{cli, init};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Manually parse for --lang before building the full CLI
    // to ensure help messages are translated correctly.
    let args: Vec<String> = std::env::args().collect();
    let mut lang_override = None;
    if let Some(pos) = args.iter().position(|s| s == "--lang") {
        if let Some(val) = args.get(pos + 1) {
            lang_override = Some(val.clone());
        }
    }

    if let Some(lang) = lang_override {
        rust_i18n::set_locale(&lang);
    } else {
        init(); // Default to system locale
    }

    // Now that locale is set, build the CLI and process commands
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
