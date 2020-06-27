/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::process::{exit, Command};

extern crate exitcode;
extern crate rand;
extern crate ring;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use ring::digest::{digest, SHA256};
use ring::test::from_hex;

use crate::avatar_env::{CONFIG_LOCK_PATH, CONFIG_PATH, PROJECT_PATH, SESSION_TOKEN, STATE_PATH};
use crate::directories::get_project_path;
use crate::project_config::{get_config_lock, get_config_lock_vec};

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
    let config_path = project_path.join(".avatar-cli").join("avatar-cli.yml");
    // We do not check again if config_path exists, since it was implicitly checked by `get_project_path`.

    let project_state_bytes = get_config_lock_vec(&project_state_path);
    let config_lock_bytes = get_config_lock_vec(&config_lock_path);

    let project_state = get_config_lock(&project_state_bytes, &project_state_path);
    let config_lock_hash = digest(&SHA256, &config_lock_bytes);

    let hash_from_state = match from_hex(project_state.getProjectConfigHash()) {
        Ok(h) => h,
        Err(_) => {
            eprintln!(
                "Unable to read the config lock hash from the '{}' file",
                project_state_path.display()
            );
            exit(exitcode::DATAERR)
        }
    };

    if &config_lock_hash.as_ref() != &&hash_from_state[..] {
        eprintln!(
            "The hash for the file '{}' does not match with the one in '{}'",
            config_lock_path.display(),
            project_state_path.display()
        );
        exit(exitcode::DATAERR) // TODO: Update state instead of stopping the process
    }

    let shell_path = match env::var("SHELL") {
        Ok(sp) => sp,
        Err(_) => "/bin/sh".to_string(),
    };

    let path_var = match env::var("PATH") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Unable to load PATH environment variable");
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
        .env(SESSION_TOKEN, session_token)
        .env(STATE_PATH, project_state_path)
        .exec();
}
