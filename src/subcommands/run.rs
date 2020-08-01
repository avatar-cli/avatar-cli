/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::path::PathBuf;
use std::{
    process::{exit, Command},
    str::from_utf8,
};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::avatar_env::{AvatarEnv, PROCESS_ID, PROJECT_INTERNAL_ID, SESSION_TOKEN};
use crate::directories::{check_if_inside_project_dir, get_project_path};
use crate::project_config::{get_config, get_config_lock, ImageBinaryConfigLock};

pub(crate) fn run_subcommand() {
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

pub(crate) fn run_in_subshell_mode(used_program_name: &str) {
    let project_env = AvatarEnv::read();
    let project_path = project_env.get_project_path();

    run(
        project_path,
        used_program_name,
        project_env.get_session_token(),
        1,
    );
}

fn run(project_path: &PathBuf, used_program_name: &str, session_token: &str, skip_args: usize) {
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

    if config_hash.as_ref() != &config_lock.get_project_config_hash()[..] {
        eprintln!(
        "The hash for the file '{}' does not match with the one in '{}', considering exiting the avatar subshell and entering again",
        config_path.display(),
        config_lock_path.display()
    );
        exit(exitcode::DATAERR)
    }

    let (project_state, _) = get_config_lock(&project_state_path);

    if config_lock_hash.as_ref() != &project_state.get_project_config_hash()[..] {
        eprintln!(
        "The hash for the file '{}' does not match with the one in '{}', considering exiting the avatar subshell and entering again",
        config_lock_path.display(),
        project_state_path.display()
    );
        exit(exitcode::DATAERR)
    }

    let binary_configuration = match project_state.get_binary_configuration(&used_program_name) {
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
        project_state.get_project_internal_id(),
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
) {
    if which::which("docker").is_err() {
        eprintln!("docker client is not available");
        exit(exitcode::UNAVAILABLE)
    }

    let mut interactive_options: Vec<&str> = vec!["-i"]; // TODO: Check if stdin is open
    if atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout) {
        interactive_options.push("-t")
    }

    let mut dynamic_env: Vec<String> = Vec::new();
    let mut dynamic_mounts: Vec<String> = Vec::new();
    if let Some(run_config) = binary_configuration.get_run_config() {
        if let Some(used_defined_env_vars) = run_config.get_env() {
            for (var_name, var_value) in used_defined_env_vars {
                if var_name == "PATH" {
                    eprintln!("Passing a custom PATH environment variable is forbidden");
                    exit(exitcode::USAGE)
                }

                dynamic_env.push("--env".to_string());
                dynamic_env.push(format!("{}={}", var_name, var_value));
            }
        }

        if let Some(host_var_names) = run_config.get_env_from_host() {
            for var_name in host_var_names {
                if var_name == "PATH" {
                    eprintln!("Passing a custom PATH environment variable is forbidden");
                    exit(exitcode::USAGE)
                }

                if let Ok(var_value) = env::var(var_name) {
                    dynamic_env.push("--env".to_string());
                    dynamic_env.push(format!("{}={}", var_name, var_value));
                }
            }
        }

        if let Some(volumes) = run_config.get_volumes() {
            for volume_config in volumes {
                dynamic_mounts.push("--volume".to_string());
                dynamic_mounts.push(format!(
                    "{}:{}",
                    volume_config.get_name(),
                    volume_config.get_container_path().display()
                ));
            }
        }

        if let Some(bindings) = run_config.get_bindings() {
            for (container_path, host_path) in bindings {
                dynamic_mounts.push("--mount".to_string());
                dynamic_mounts.push(format!(
                    "type=bind,source={},target={}",
                    host_path.display(),
                    container_path.display()
                ));
            }
        }
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
    let program_name = match binary_configuration
        .get_path()
        .file_name()
        .unwrap()
        .to_str()
    {
        Some(pn) => pn,
        None => "yyy",
    };

    let uid = nix::unistd::getuid();
    let home_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("home");

    Command::new("docker")
        .args(&["run", "--rm", "--init"])
        .args(interactive_options)
        .args(dynamic_env)
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
            &format!("{}:{}", uid, nix::unistd::getgid()),
            "--mount",
            &format!(
                "type=bind,source={},target=/playground",
                project_path.display() // TODO: Escape commas?
            ),
            "--workdir",
            &format!("/playground/{}", working_dir.display()),
            "--mount",
            &format!(
                "type=bind,source={},target=/home/avatar-cli",
                home_path.display() // TODO: Escape commas?
            ),
            "--env",
            "HOME=/home/avatar-cli",
        ])
        .args(dynamic_mounts)
        .args(get_user_integration_args(uid))
        .arg(format!(
            "{}@sha256:{}",
            binary_configuration.get_oci_image_name(),
            binary_configuration.get_oci_image_hash()
        ))
        .arg(binary_configuration.get_path())
        .args(env::args().skip(skip_args))
        .exec(); // Only for UNIX
}

fn get_user_integration_args(uid: nix::unistd::Uid) -> Vec<String> {
    let mut dynamic_args: Vec<String> = vec![];

    if let Ok(Some(user)) = nix::unistd::User::from_uid(uid) {
        dynamic_args.push("--env".to_string());
        dynamic_args.push(format!("USER={}", user.name));
        dynamic_args.push("--env".to_string());
        dynamic_args.push(format!("USERNAME={}", user.name));
    }

    // TODO: Detect if running in Mac or WSL, and apply this hack:
    // https://github.com/docker/for-mac/issues/410#issuecomment-536531657
    push_socket_dir_args("SSH_AUTH_SOCK", &mut dynamic_args);
    push_socket_dir_args("GPG_AGENT_INFO", &mut dynamic_args);

    if let Some(home_dir) = dirs::home_dir() {
        push_home_config_args(&home_dir, ".ssh", &mut dynamic_args);
        push_home_config_args(&home_dir, ".gnupg", &mut dynamic_args);
    }

    if let Ok(output) = Command::new("git").args(&["config", "user.name"]).output() {
        if output.status.success() {
            if let Ok(git_user_name) = from_utf8(&output.stdout) {
                let trimmed_name = git_user_name.trim();

                dynamic_args.push("--env".to_string());
                dynamic_args.push(format!("GIT_AUTHOR_NAME={}", trimmed_name));
                dynamic_args.push("--env".to_string());
                dynamic_args.push(format!("GIT_COMMITTER_NAME={}", trimmed_name));
            }
        }
    }

    if let Ok(output) = Command::new("git").args(&["config", "user.email"]).output() {
        if output.status.success() {
            if let Ok(git_user_email) = from_utf8(&output.stdout) {
                let trimmed_email = git_user_email.trim();

                dynamic_args.push("--env".to_string());
                dynamic_args.push(format!("GIT_AUTHOR_EMAIL={}", trimmed_email));
                dynamic_args.push("--env".to_string());
                dynamic_args.push(format!("GIT_COMMITTER_EMAIL={}", trimmed_email));
            }
        }
    }

    dynamic_args
}

fn push_socket_dir_args(socket_var_name: &str, dynamic_args: &mut Vec<String>) {
    if let Ok(v) = env::var(socket_var_name) {
        if let Some(sockets_dir) = PathBuf::from(&v).parent() {
            dynamic_args.push("--mount".to_string());
            dynamic_args.push(format!(
                "type=bind,source={},target={}",
                sockets_dir.display(),
                sockets_dir.display()
            ));
            dynamic_args.push("--env".to_string());
            dynamic_args.push(format!("{}={}", socket_var_name, v));
        }
    }
}

fn push_home_config_args(home_dir: &PathBuf, config_name: &str, dynamic_args: &mut Vec<String>) {
    let config_dir = home_dir.join(config_name);
    if config_dir.exists() && config_dir.is_dir() {
        dynamic_args.push("--mount".to_string());
        dynamic_args.push(format!(
            "type=bind,source={},target=/home/avatar-cli/{}",
            config_dir.display(),
            config_name
        ));
    }
}
