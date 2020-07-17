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

use crate::avatar_env::{AvatarEnv, PROCESS_ID, PROJECT_INTERNAL_ID, SESSION_TOKEN};
use crate::directories::{check_if_inside_project_dir, get_project_path};
use crate::project_config::{get_config, get_config_lock, ImageBinaryConfigLock};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub(crate) fn run_subcommand() -> () {
    let project_path = match get_project_path() {
        Some(p) => p,
        None => {
            eprintln!("The command was not executed inside an Avatar CLI project directory");
            exit(exitcode::USAGE)
        }
    };

    let used_program_name = match env::args().nth(2) {
        Some(n) => n,
        None => {
            eprintln!("A program name must be passed to 'avatar run'");
            exit(exitcode::USAGE)
        }
    };

    let session_token = match env::var(SESSION_TOKEN) {
        Ok(st) => st,
        Err(_) => thread_rng().sample_iter(&Alphanumeric).take(16).collect(),
    };

    run(&project_path, &used_program_name, &session_token, 4)
}

pub(crate) fn run_in_subshell_mode(used_program_name: &str) -> () {
    let project_env = AvatarEnv::read();
    let project_path = project_env.get_project_path();

    run(
        project_path,
        used_program_name,
        project_env.get_session_token(),
        1,
    );
}

fn run(
    project_path: &PathBuf,
    used_program_name: &str,
    session_token: &str,
    skip_args: usize,
) -> () {
    let current_dir = match env::current_dir() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Unable to get current working directory");
            exit(exitcode::NOINPUT)
        }
    };

    check_if_inside_project_dir(project_path, &current_dir);

    let config_path = project_path.join(".avatar-cli").join("avatar-cli.yml");
    if !config_path.exists() || !config_path.is_file() {
        eprintln!("The config file '{}' is not available anymore, please check if there is any background process modifying files in your project directory", config_path.display());
        exit(exitcode::NOINPUT)
    }

    let config_lock_path = project_path.join(".avatar-cli").join("avatar-cli.lock.yml");
    if !config_lock_path.exists() || !config_lock_path.is_file() {
        eprintln!("The config lock file '{}' is not available anymore, please check if there is any background process modifying files in your project directory", config_lock_path.display());
        exit(exitcode::NOINPUT)
    }

    let project_state_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("state.yml");
    if !project_state_path.exists() || !project_state_path.is_file() {
        eprintln!("The project state file '{}' is not available anymore, please check if there is any background process modifying files in your project directory", project_state_path.display());
        exit(exitcode::NOINPUT)
    }

    let (_, config_hash) = get_config(&config_path);
    let (config_lock, config_lock_hash) = get_config_lock(&config_lock_path);

    if &config_hash.as_ref() != &&config_lock.getProjectConfigHash()[..] {
        eprintln!(
        "The hash for the file '{}' does not match with the one in '{}', considering exiting the avatar subshell and entering again",
        config_path.display(),
        config_lock_path.display()
    );
        exit(exitcode::DATAERR)
    }

    let (project_state, _) = get_config_lock(&project_state_path);

    if &config_lock_hash.as_ref() != &&project_state.getProjectConfigHash()[..] {
        eprintln!(
        "The hash for the file '{}' does not match with the one in '{}', considering exiting the avatar subshell and entering again",
        config_lock_path.display(),
        project_state_path.display()
    );
        exit(exitcode::DATAERR)
    }

    let binary_configuration = match project_state.getBinaryConfiguration(&used_program_name) {
        Some(c) => c,
        None => {
            eprintln!(
                "Binary '{}' not properly configured in lock file '{}'",
                used_program_name,
                project_state_path.display()
            );
            exit(1)
        }
    };

    run_docker_command(
        binary_configuration,
        &current_dir,
        project_path,
        project_state.getProjectInternalId(),
        session_token,
        skip_args,
    );
}

fn run_docker_command(
    binary_configuration: &ImageBinaryConfigLock,
    current_dir: &PathBuf,
    project_path: &PathBuf,
    project_internal_id: &str,
    session_token: &str,
    skip_args: usize,
) -> () {
    if let Err(_) = which::which("docker") {
        eprintln!("docker client is not available");
        exit(exitcode::UNAVAILABLE)
    }

    let mut interactive_options: Vec<&str> = vec!["-i"]; // TODO: Check if stdin is open
    if atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout) {
        interactive_options.push("-t")
    }

    let working_dir = match current_dir.strip_prefix(project_path) {
        Ok(wd) => wd,
        Err(_) => {
            eprintln!("A precondition of run_docker_command does not hold: working directory inside project directory");
            exit(exitcode::SOFTWARE)
        }
    };

    let process_id: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();
    let project_name = match project_path.file_name().unwrap().to_str() {
        Some(pn) => pn,
        None => "xxx",
    };
    let program_name = match binary_configuration.getPath().file_name().unwrap().to_str() {
        Some(pn) => pn,
        None => "yyy",
    };

    Command::new("docker")
        .args(&["run", "--rm", "--init"])
        .args(interactive_options)
        .args(&[
            "--name",
            &format!(
                "{}_{}_{}_{}_{}",
                project_name, program_name, project_internal_id, session_token, process_id
            ),
            "--env",
            &format!("{}={}", PROCESS_ID, process_id),
            "--env",
            &format!("{}={}", PROJECT_INTERNAL_ID, project_internal_id),
            "--env",
            &format!("{}={}", SESSION_TOKEN, session_token),
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
        .args(env::args().skip(skip_args))
        .exec(); // Only for UNIX
}
