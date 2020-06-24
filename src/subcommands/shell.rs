/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::process::exit;

extern crate exitcode;

use crate::avatar_env::SESSION_TOKEN;
use crate::directories::get_project_path;

pub(crate) fn shell_subcommand() -> () {
    if let Ok(session_token) = env::var(SESSION_TOKEN) {
        eprintln!(
            "You are already in an Avatar CLI session (with token '{}').\nIf the environment changed, consider typing 'exit' and trying again.",
            session_token
        );
        exit(exitcode::USAGE)
    }

    let project_path = match get_project_path() {
        Some(p) => p,
        None => {
            eprintln!("The command was not executed inside an Avatar CLI project directory");
            exit(exitcode::USAGE)
        }
    };
    let project_state_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("state.yml");
    if !project_state_path.exists() || !project_state_path.is_file() {
        eprintln!("Avatar CLI does not yet implement the implicit 'install' step");
        exit(exitcode::SOFTWARE) // TODO: Trigger implicit "install" step
    }
    let config_lock_path = project_path.join(".avatar-cli").join("avatar-cli.lock.yml");
    if !config_lock_path.exists() || !config_lock_path.is_file() {
        eprintln!("Avatar CLI does not yet implement the implicit 'install' step");
        exit(exitcode::SOFTWARE) // TODO: Trigger implicit "install" step (but here it will do more stuff than in the previous case)
    }
    let _config_path = project_path.join(".avatar-cli").join("avatar-cli.yml");
    // We do not check again if config_path exists, since it was implicitly checked by `get_project_path`.

    // TODO: Continue
}
