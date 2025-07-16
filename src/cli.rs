//! # CLI Module / CLI 模块
//!
//! This module defines the command-line interface for Matrix Runner.
//! It processes command line arguments and dispatches to the appropriate commands.
//!
//! 此模块定义了 Matrix Runner 的命令行界面。
//! 它处理命令行参数并分派到适当的命令。

pub mod commands;

use crate::infra::t;
use clap::{Arg, ArgMatches, Command};
use std::path::PathBuf;

/// Builds the CLI structure using clap's builder pattern.
///
/// This function constructs the entire command-line interface dynamically,
/// allowing for internationalization of help messages and descriptions.
pub fn build_cli() -> Command {
    Command::new("matrix-runner")
        .about(t!("cli.about").to_string())
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("lang")
                .long("lang")
                .help(t!("cli.lang.help").to_string())
                .global(true)
                .value_parser(clap::value_parser!(String)),
        )
        .subcommand(
            Command::new("run")
                .about(t!("cli.run.about").to_string())
                .arg(
                    Arg::new("jobs")
                        .short('j')
                        .long("jobs")
                        .help(t!("cli.run.jobs").to_string())
                        .value_parser(clap::value_parser!(usize)),
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .help(t!("cli.run.config").to_string())
                        .default_value("TestMatrix.toml")
                        .value_parser(clap::value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("project_dir")
                        .short('p')
                        .long("project-dir")
                        .help(t!("cli.run.project_dir").to_string())
                        .default_value(".")
                        .value_parser(clap::value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("total_runners")
                        .long("total-runners")
                        .help(t!("cli.run.total_runners").to_string())
                        .value_parser(clap::value_parser!(usize)),
                )
                .arg(
                    Arg::new("runner_index")
                        .long("runner-index")
                        .help(t!("cli.run.runner_index").to_string())
                        .value_parser(clap::value_parser!(usize)),
                )
                .arg(
                    Arg::new("html")
                        .long("html")
                        .help(t!("cli.run.html").to_string())
                        .value_parser(clap::value_parser!(PathBuf)),
                ),
        )
        .subcommand(
            Command::new("init")
                .about(t!("cli.init.about").to_string())
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .help(t!("cli.init.output").to_string())
                        .default_value("TestMatrix.toml")
                        .value_parser(clap::value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help(t!("cli.init.force").to_string())
                        .action(clap::ArgAction::SetTrue),
                ),
        )
}

/// Process the parsed CLI command and dispatch to the appropriate handler.
///
/// This function takes the matches from the parsed command line and calls the
/// corresponding command logic.
pub async fn process_command(matches: ArgMatches) -> anyhow::Result<()> {
    let lang = matches.get_one::<String>("lang").cloned();

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let jobs = sub_matches.get_one::<usize>("jobs").copied();
            let config = sub_matches
                .get_one::<PathBuf>("config")
                .expect("default value should be present")
                .clone();
            let project_dir = sub_matches
                .get_one::<PathBuf>("project_dir")
                .expect("default value should be present")
                .clone();
            let total_runners = sub_matches.get_one::<usize>("total_runners").copied();
            let runner_index = sub_matches.get_one::<usize>("runner_index").copied();
            let html = sub_matches.get_one::<PathBuf>("html").cloned();

            commands::run::execute(
                jobs,
                config,
                project_dir,
                total_runners,
                runner_index,
                html,
                lang,
            )
            .await
        }
        Some(("init", sub_matches)) => {
            let output = sub_matches
                .get_one::<PathBuf>("output")
                .expect("default value should be present")
                .clone();
            let force = sub_matches.get_flag("force");

            commands::init::execute(output, force, lang).await
        }
        _ => unreachable!("clap should have handled this because subcommand_required is set"),
    }
} 