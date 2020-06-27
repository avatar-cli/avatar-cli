/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::path::PathBuf;
use std::process::{exit, Command};

extern crate atty;
extern crate exitcode;
extern crate nix;
extern crate which;

use crate::avatar_env::AvatarEnv;
use crate::directories::check_if_inside_project_dir;
use crate::project_config::{get_config_lock, get_config_lock_vec, ImageBinaryConfigLock};

fn run_docker_command(
    project_env: AvatarEnv,
    binary_configuration: &ImageBinaryConfigLock,
    current_dir: PathBuf,
) -> () {
    let docker_client_path = match which::which("docker") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("docker client is not available");
            exit(exitcode::UNAVAILABLE)
        }
    };

    let mut interactive_options: Vec<&str> = vec!["-i"]; // TODO: Check if stdin is open
    if atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout) {
        interactive_options.push("-t")
    }

    let project_path = project_env.get_project_path();
    let working_dir = match current_dir.strip_prefix(project_path) {
        Ok(wd) => wd,
        Err(_) => {
            eprintln!("A precondition of run_docker_command does not hold: working directory inside project directory");
            exit(exitcode::SOFTWARE)
        }
    };

    Command::new(docker_client_path)
        .args(&["run", "--rm", "--init"])
        .args(interactive_options)
        .args(&[
            "--user",
            &format!("{}:{}", nix::unistd::getuid(), nix::unistd::getgid()),
            "--mount",
            &format!(
                "type=bind,source={},target=/playground",
                project_path.display() // TODO: Escape commas?
            ),
            "--workdir",
            &format!("/playground/{}", working_dir.display()),
        ])
        .arg(format!(
            "{}@sha256:{}",
            binary_configuration.getOCIImageName(),
            binary_configuration.getOCIImageHash()
        ))
        .arg(binary_configuration.getPath())
        .args(env::args().skip(1))
        .exec(); // Only for UNIX
}

pub(crate) fn run_in_subshell_mode(used_program_name: String) -> () {
    let project_env = AvatarEnv::read();
    let project_path = project_env.get_project_path();
    let current_dir = match env::current_dir() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Unable to get current working directory");
            exit(exitcode::NOINPUT)
        }
    };

    check_if_inside_project_dir(project_path, &current_dir);

    // TODO: should read the volatile/state.yml file instead of avatar-cli.lock.yml
    let config_lock_path = project_env.get_config_lock_path();
    let config_lock_vec = get_config_lock_vec(config_lock_path);
    let config_lock = get_config_lock(&config_lock_vec, config_lock_path);

    let binary_configuration = match config_lock.getBinaryConfiguration(&used_program_name) {
        Some(c) => c,
        None => {
            eprintln!(
                "Binary '{}' not properly configure in lock file '{}'",
                used_program_name,
                config_lock_path.display()
            );
            exit(1)
        }
    };

    run_docker_command(project_env, binary_configuration, current_dir);
}
