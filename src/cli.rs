// src/cli.rs
use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use std::{env, path::PathBuf};

use crate::{commands, t};

/// Pre-parses the command line arguments to find the language setting.
/// This allows i18n to be initialized before the full CLI is built.
/// It looks for a `--lang <VALUE>` argument.
fn pre_parse_language() -> String {
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--lang") {
        if let Some(lang) = args.get(pos + 1) {
            return lang.clone();
        }
    }
    // Fallback to system language detection
    sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
}

fn build_cli(locale: &str) -> Command {
    Command::new("matrix-runner")
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(t!("cli_about", locale = locale).to_string())
        .arg(
            Arg::new("lang")
                .long("lang")
                .help(t!("cli_lang", locale = locale).to_string())
                .value_name("LANGUAGE")
                .global(true)
                .action(ArgAction::Set),
        )
        .subcommand(
            Command::new("run")
                .about(t!("cmd_run_about", locale = locale).to_string())
                .arg(
                    Arg::new("jobs")
                        .short('j')
                        .long("jobs")
                        .help(t!("arg_jobs", locale = locale).to_string())
                        .value_name("JOBS")
                        .value_parser(clap::value_parser!(usize))
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .help(t!("arg_config", locale = locale).to_string())
                        .value_name("CONFIG")
                        .default_value("TestMatrix.toml")
                        .value_parser(clap::value_parser!(PathBuf))
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("project-dir")
                        .long("project-dir")
                        .help(t!("arg_project_dir", locale = locale).to_string())
                        .value_name("PROJECT_DIR")
                        .default_value(".")
                        .value_parser(clap::value_parser!(PathBuf))
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("total-runners")
                        .long("total-runners")
                        .help(t!("arg_total_runners", locale = locale).to_string())
                        .value_name("TOTAL_RUNNERS")
                        .value_parser(clap::value_parser!(usize))
                        .action(ArgAction::Set)
                        .requires("runner-index"),
                )
                .arg(
                    Arg::new("runner-index")
                        .long("runner-index")
                        .help(t!("arg_runner_index", locale = locale).to_string())
                        .value_name("RUNNER_INDEX")
                        .value_parser(clap::value_parser!(usize))
                        .action(ArgAction::Set)
                        .requires("total-runners"),
                )
                .arg(
                    Arg::new("html")
                        .long("html")
                        .help(t!("arg_html", locale = locale).to_string())
                        .value_name("HTML")
                        .value_parser(clap::value_parser!(PathBuf))
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("init")
                .about(t!("cmd_init_about", locale = locale).to_string())
                .arg(
                    Arg::new("non-interactive")
                        .long("non-interactive")
                        .help("Create a default config file without launching the interactive wizard.")
                        .action(ArgAction::SetTrue),
                ),
        )
}

pub async fn run() -> Result<()> {
    // Pre-parse language and initialize i18n first.
    let language = pre_parse_language();
    rust_i18n::set_locale(&language);

    let matches = build_cli(&language).get_matches();

    match matches.subcommand() {
        Some(("run", run_matches)) => {
            let jobs = run_matches.get_one::<usize>("jobs").copied();
            let config = run_matches
                .get_one::<PathBuf>("config")
                .unwrap() // Has default
                .clone();
            let project_dir = run_matches
                .get_one::<PathBuf>("project-dir")
                .unwrap() // Has default
                .clone();
            let total_runners = run_matches.get_one::<usize>("total-runners").copied();
            let runner_index = run_matches.get_one::<usize>("runner-index").copied();
            let html = run_matches.get_one::<PathBuf>("html").cloned();

            // This will be moved to commands::run::execute
            commands::run::execute(
                jobs,
                config,
                project_dir,
                total_runners,
                runner_index,
                html,
            )
            .await?;
        }
        Some(("init", init_matches)) => {
            let non_interactive = init_matches.get_flag("non-interactive");

            // Show language detection message if it was auto-detected
            if env::args().all(|arg| arg != "--lang") {
                println!(
                    "🌐 {}",
                    t!("system_language_detected", locale = &language, lang = &language)
                );
            }
            commands::init::run_init_wizard(&language, non_interactive)?;
        }
        _ => {
            // This case handles when no subcommand is given.
            // Clap will have already printed help info.
        }
    }
    Ok(())
} 