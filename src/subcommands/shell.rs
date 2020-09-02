/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::{exit, Command},
};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::avatar_env::{
    CONFIG_LOCK_PATH, CONFIG_PATH, PROJECT_INTERNAL_ID, PROJECT_PATH, SESSION_TOKEN, STATE_PATH,
};
use crate::{
    directories::{CONFIG_DIR_NAME, VOLATILE_DIR_NAME},
    subcommands::install::install_subcommand,
};

pub(crate) fn shell_subcommand() {
    let (project_path, config_path, config_lock_path, project_state_path, project_state) =
        install_subcommand(true);

    let shell_path = match env::var("SHELL") {
        Ok(sp) => sp,
        Err(_) => "/bin/sh".to_string(),
    };

    let (shell_env, shell_extra_paths) = match project_state.get_shell_config() {
        Some(shell_config) => (
            shell_config.get_env().clone().unwrap_or_default(),
            shell_config.get_extra_paths().clone().unwrap_or_default(),
        ),
        None => (
            BTreeMap::<String, String>::new(),
            BTreeSet::<PathBuf>::new(),
        ),
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
        .join(CONFIG_DIR_NAME)
        .join(VOLATILE_DIR_NAME)
        .join("bin");
    let extra_paths = shell_extra_paths
        .iter()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                project_path.join(p)
            }
        })
        .collect::<Vec<PathBuf>>()
        .iter()
        .map(|p| p.to_str())
        .filter(|p| p.is_some())
        .map(|p| p.unwrap())
        .collect::<Vec<&str>>()
        .join(":");
    let path_var = format!("{}:{}:{}", avatar_bin_path.display(), extra_paths, path_var);

    let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();

    Command::new(shell_path)
        .envs(shell_env)
        .env("PATH", path_var)
        .env(CONFIG_PATH, config_path)
        .env(CONFIG_LOCK_PATH, config_lock_path)
        .env(PROJECT_PATH, project_path)
        .env(PROJECT_INTERNAL_ID, project_state.get_project_internal_id())
        .env(SESSION_TOKEN, session_token)
        .env(STATE_PATH, project_state_path)
        .exec();
}

pub(crate) fn export_env_subcommand() {
    let (project_path, config_path, config_lock_path, project_state_path, project_state) =
        install_subcommand(false);

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
        .join(CONFIG_DIR_NAME)
        .join(VOLATILE_DIR_NAME)
        .join("bin");
    let path_var = format!("{}:{}", avatar_bin_path.display(), path_var);

    let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();

    println!("export PATH=\"{}\"", path_var);
    println!("export {}=\"{}\"", CONFIG_PATH, config_path.display());
    println!(
        "export {}=\"{}\"",
        CONFIG_LOCK_PATH,
        config_lock_path.display()
    );
    println!("export {}=\"{}\"", PROJECT_PATH, project_path.display());
    println!(
        "export {}=\"{}\"",
        PROJECT_INTERNAL_ID,
        project_state.get_project_internal_id()
    );
    println!("export {}=\"{}\"", SESSION_TOKEN, session_token);
    println!("export {}=\"{}\"", STATE_PATH, project_state_path.display());
}
