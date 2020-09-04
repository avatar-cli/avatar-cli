/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

mod avatar_env;
mod directories;
mod docker;
mod project_config;
mod subcommands;

fn main() {
    let used_program_name = avatar_env::get_used_program_name();
    if used_program_name == "avatar" {
        subcommands::select()
    } else {
        subcommands::run::run_in_subshell_mode(&used_program_name)
    }
}
