/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::process::{exit, Command};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::avatar_env::{
    CONFIG_LOCK_PATH, CONFIG_PATH, PROJECT_INTERNAL_ID, PROJECT_PATH, SESSION_TOKEN, STATE_PATH,
};
use crate::subcommands::install::install_subcommand;

pub(crate) fn shell_subcommand() {
    let (project_path, config_path, config_lock_path, project_state_path, project_state) =
        install_subcommand();

    let shell_path = match env::var("SHELL") {
        Ok(sp) => sp,
        Err(_) => "/bin/sh".to_string(),
    };

    let path_var = match env::var("PATH") {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "Unable to load PATH environment variable\n\n{}\n",
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    };
    let avatar_bin_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("bin");
    let path_var = format!("{}:{}", avatar_bin_path.display(), path_var);

    let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();

    Command::new(shell_path)
        .env("PATH", path_var)
        .env(CONFIG_PATH, config_path)
        .env(CONFIG_LOCK_PATH, config_lock_path)
        .env(PROJECT_PATH, project_path)
        .env(PROJECT_INTERNAL_ID, project_state.get_project_internal_id())
        .env(SESSION_TOKEN, session_token)
        .env(STATE_PATH, project_state_path)
        .exec();
}
