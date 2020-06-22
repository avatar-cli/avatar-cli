/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::process::exit;

extern crate clap;
use clap::{App, AppSettings, SubCommand};

pub(crate) fn run_in_subcommand_mode() -> () {
    println!("This code path has not been defined yet");

    let matches = App::new("avatar")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("shell")
                .about("Starts a new subshell exposing the wrapped project tools"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Executes a wrapped project tool without having to enter into a subshell"),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some(subcommand_name) => match subcommand_name {
            "avatar" => exit(exitcode::OK),
            "avatar-cli" => exit(exitcode::OK),
            "run" => {}
            "shell" => {}
            _ => {
                eprintln!("Invalid subcommand");
                exit(exitcode::USAGE)
            }
        },
        None => exit(exitcode::SOFTWARE), // This branch should be unreachable
    };
}
