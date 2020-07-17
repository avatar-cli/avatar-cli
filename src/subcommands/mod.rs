/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

pub(crate) mod run;
pub(crate) mod shell;

use std::process::exit;

extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand};

const AVATAR_CLI_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub(crate) fn select() -> () {
    let matches = App::new("avatar")
        .version(AVATAR_CLI_VERSION)
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("shell")
                .about("Starts a new subshell exposing the wrapped project tools"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Executes a wrapped project tool without having to enter into a subshell")
                .arg(Arg::with_name("program_name").index(1).required(true))
                .arg(
                    Arg::with_name("program_args")
                        .multiple(true)
                        .required(false),
                ),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some(subcommand_name) => match subcommand_name {
            "avatar" => exit(exitcode::OK),
            "avatar-cli" => exit(exitcode::OK),
            "run" => run::run_subcommand(),
            "shell" => shell::shell_subcommand(),
            _ => {
                eprintln!("Invalid subcommand");
                exit(exitcode::USAGE)
            }
        },
        None => exit(exitcode::SOFTWARE), // This branch should be unreachable
    };
}
