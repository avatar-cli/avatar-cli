/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

extern crate exitcode;

use std::env;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::process::{exit, Command};

mod project_config;

use project_config::ProjectConfigLock;

fn main() {
    let cmd_args: Vec<String> = env::args().collect();
    if cmd_args.is_empty() {
        eprintln!(
            "Due to an unknown reason, it was impossible to retrieve the command arguments list"
        );
        exit(exitcode::OSERR);
    }
    let first_arg: &String = &cmd_args[0];
    let used_program_name = match first_arg.split(MAIN_SEPARATOR).last() {
        Some(pname) => pname,
        None => {
            eprintln!("Due to an unknown reason, an empty first command argument was passed to this process");
            exit(exitcode::OSERR)
        }
    };

    if used_program_name == "avatar" || used_program_name == "avatar-cli" {
        println!("This code path has not been defined yet");
        exit(exitcode::SOFTWARE)
    }

    let config_lock_filepath = PathBuf::from(match env::var("AVATAR_CLI_CONFIG_LOCK_PATH") {
        Ok(fp) => fp,
        Err(_) => {
            eprintln!("The AVATAR_CLI_CONFIG_LOCK_PATH environment variable is not defined");
            exit(exitcode::CONFIG)
        }
    });

    if !config_lock_filepath.exists() || !config_lock_filepath.is_file() {
        eprintln!(
            "The lock file {} is not available",
            &config_lock_filepath.display()
        );
        exit(exitcode::NOINPUT)
    }

    let config_lock_fd = match File::open(&config_lock_filepath) {
        Ok(s) => s,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                eprintln!(
                    "The lock file {} is not available",
                    &config_lock_filepath.display()
                );
                exit(exitcode::NOINPUT)
            }
            ErrorKind::PermissionDenied => {
                eprintln!(
                    "The lock file {} is not readable due to filesystem permissions",
                    &config_lock_filepath.display()
                );
                exit(exitcode::IOERR)
            }
            _ => {
                eprintln!(
                    "Unknown IO error while reading the lock file {}",
                    &config_lock_filepath.display()
                );
                exit(exitcode::IOERR)
            }
        },
    };

    let config_lock = match serde_yaml::from_reader::<File, ProjectConfigLock>(config_lock_fd) {
        Ok(_config_lock) => _config_lock,
        Err(e) => {
            let error_msg = match e.location() {
                Some(l) => format!(
                    "Malformed lock file '{}', line {}, column {}:\n\t{}",
                    &config_lock_filepath.display(),
                    l.line(),
                    l.column(),
                    e.to_string(),
                ),
                None => format!(
                    "Malformed lock file '{}':\n\t{}",
                    &config_lock_filepath.display(),
                    e.to_string(),
                ),
            };

            eprintln!("{}", error_msg);
            exit(exitcode::DATAERR)
        }
    };

    let binary_configuration = match config_lock.getBinaryConfiguration(used_program_name) {
        Some(c) => c,
        None => {
            eprintln!("Binary '{}' not properly configure in lock file '{}'", used_program_name, &config_lock_filepath.display());
            exit(1)
        }
    };

    Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-i")
        .arg("-t")
        .arg(format!(
            "{}@sha256:{}",
            binary_configuration.getOCIImageName().unpack(),
            binary_configuration.getOCIImageHash().unpack()
        ))
        .arg(binary_configuration.getPath())
        .args(env::args().skip(1))
        .exec(); // Only for UNIX
}
