/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

extern crate atty;
extern crate exitcode;
extern crate which;

use std::env;
use std::fs::read;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::exit;

mod avatar_env;
mod cmd_run;
mod project_config;

use avatar_env::{get_used_program_name, AvatarEnv};
use cmd_run::run_docker_command;
use project_config::ProjectConfigLock;

fn get_config_lock_vec(config_lock_filepath: &PathBuf) -> Vec<u8> {
    if !config_lock_filepath.exists() || !config_lock_filepath.is_file() {
        eprintln!(
            "The lock file {} is not available",
            &config_lock_filepath.display()
        );
        exit(exitcode::NOINPUT)
    }

    match read(config_lock_filepath) {
        Ok(s) => s,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                eprintln!(
                    "The lock file {} is not available",
                    config_lock_filepath.display()
                );
                exit(exitcode::NOINPUT)
            }
            ErrorKind::PermissionDenied => {
                eprintln!(
                    "The lock file {} is not readable due to filesystem permissions",
                    config_lock_filepath.display()
                );
                exit(exitcode::IOERR)
            }
            _ => {
                eprintln!(
                    "Unknown IO error while reading the lock file {}",
                    config_lock_filepath.display()
                );
                exit(exitcode::IOERR)
            }
        },
    }
}

fn get_config_lock(config_lock_slice: &[u8], config_lock_filepath: &PathBuf) -> ProjectConfigLock {
    match serde_yaml::from_slice::<ProjectConfigLock>(config_lock_slice) {
        Ok(_config_lock) => _config_lock,
        Err(e) => {
            let error_msg = match e.location() {
                Some(l) => format!(
                    "Malformed lock file '{}', line {}, column {}:\n\t{}",
                    config_lock_filepath.display(),
                    l.line(),
                    l.column(),
                    e.to_string(),
                ),
                None => format!(
                    "Malformed lock file '{}':\n\t{}",
                    config_lock_filepath.display(),
                    e.to_string(),
                ),
            };

            eprintln!("{}", error_msg);
            exit(exitcode::DATAERR)
        }
    }
}

fn main() {
    let used_program_name = get_used_program_name();
    if used_program_name == "avatar" || used_program_name == "avatar-cli" {
        println!("This code path has not been defined yet");

        let the_args: Vec<String> = env::args().collect();
        for the_arg in the_args {
            println!("{}", the_arg);
        }

        exit(exitcode::SOFTWARE)
    }

    let project_env = AvatarEnv::read();

    let config_lock_filepath = project_env.get_config_lock_filepath();
    let config_lock_vec = get_config_lock_vec(config_lock_filepath);
    let config_lock = get_config_lock(&config_lock_vec, config_lock_filepath);

    let binary_configuration = match config_lock.getBinaryConfiguration(&used_program_name) {
        Some(c) => c,
        None => {
            eprintln!(
                "Binary '{}' not properly configure in lock file '{}'",
                used_program_name,
                config_lock_filepath.display()
            );
            exit(1)
        }
    };

    run_docker_command(binary_configuration);
}
