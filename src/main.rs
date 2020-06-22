/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

extern crate atty;
extern crate exitcode;
extern crate which;

mod avatar_env;
mod cmd_run;
mod project_config;
mod subcommand;
use avatar_env::get_used_program_name;
use cmd_run::run_in_subshell_mode;
use subcommand::run_in_subcommand_mode;

fn main() {
    let used_program_name = get_used_program_name();
    if used_program_name == "avatar" || used_program_name == "avatar-cli" {
        run_in_subcommand_mode()
    } else {
        run_in_subshell_mode(used_program_name)
    }
}
