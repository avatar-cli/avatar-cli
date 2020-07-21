/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::os::unix::process::CommandExt; // Brings trait that allows us to use exec
use std::{
    collections::HashMap,
    path::PathBuf,
    process::{exit, Command},
    str::from_utf8,
};

extern crate exitcode;
extern crate rand;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::avatar_env::{
    CONFIG_LOCK_PATH, CONFIG_PATH, PROJECT_INTERNAL_ID, PROJECT_PATH, SESSION_TOKEN, STATE_PATH,
};
use crate::directories::get_project_path;
use crate::project_config::{
    get_config, get_config_lock, save_config_lock, ImageBinaryConfigLock,
    OCIImageConfig, ProjectConfig, ProjectConfigLock,
};
use ring::digest::{digest, Digest, SHA256};

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

    // We do not check again if config_path exists, since it was implicitly checked by `get_project_path`.
    let config_path = project_path.join(".avatar-cli").join("avatar-cli.yml");
    let config_lock_path = project_path.join(".avatar-cli").join("avatar-cli.lock.yml");
    let project_state_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("state.yml");

    let project_state =
        check_project_settings(&config_path, &config_lock_path, &project_state_path);
    check_oci_images_availability(&project_state);

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
        .env(PROJECT_INTERNAL_ID, project_state.getProjectInternalId())
        .env(SESSION_TOKEN, session_token)
        .env(STATE_PATH, project_state_path)
        .exec();
}

fn check_project_settings(
    config_path: &PathBuf,
    config_lock_path: &PathBuf,
    project_state_path: &PathBuf,
) -> ProjectConfigLock {
    let (config, config_hash) = get_config(&config_path);

    let (config_lock, config_lock_hash) = match config_lock_path.exists() {
        true => {
            if !config_lock_path.is_file() {
                eprintln!(
                    "The path {} must point to a regular file, found something else",
                    project_state_path.display()
                );
                exit(exitcode::DATAERR)
            }

            let (_config_lock, _config_lock_hash) = get_config_lock(&config_lock_path);

            if config_hash.as_ref() != &_config_lock.getProjectConfigHash()[..] {
                generate_config_lock(config_lock_path, &config, &config_hash)
            } else {
                (_config_lock, _config_lock_hash)
            }
        }
        false => generate_config_lock(config_lock_path, &config, &config_hash),
    };

    let project_state = match project_state_path.exists() {
        true => {
            if !project_state_path.is_file() {
                eprintln!(
                    "The path {} must point to a regular file, found something else",
                    project_state_path.display()
                );
                exit(exitcode::DATAERR)
            }

            let (mut _project_state, _) = get_config_lock(&project_state_path);

            if config_lock_hash.as_ref() != &_project_state.getProjectConfigHash()[..] {
                _project_state = update_project_state(
                    project_state_path,
                    _project_state,
                    config_lock_hash.as_ref(),
                );
            }

            _project_state
        }
        false => update_project_state(
            project_state_path,
            config_lock.clone(),
            config_lock_hash.as_ref(),
        ),
    };

    return project_state;
}

fn update_project_state(
    project_state_path: &PathBuf,
    mut project_state: ProjectConfigLock,
    config_lock_hash: &[u8],
) -> ProjectConfigLock {
    project_state = project_state.updateProjectConfigHash(config_lock_hash);
    save_config_lock(project_state_path, &project_state);
    project_state
}

fn generate_config_lock(
    config_lock_path: &PathBuf,
    config: &ProjectConfig,
    config_hash: &Digest,
) -> (ProjectConfigLock, Digest) {
    let image_hashes = get_image_hashes(config);
    let binaries_settings = get_binaries_settings(config, &image_hashes);

    let config_lock = ProjectConfigLock::new(
        Vec::<u8>::from(config_hash.as_ref()),
        config.getProjectInternalId().clone(),
        image_hashes,
        binaries_settings,
    );

    let config_lock_bytes = save_config_lock(config_lock_path, &config_lock);
    (config_lock, digest(&SHA256, &config_lock_bytes))
}

fn get_image_hashes(config: &ProjectConfig) -> HashMap<String, HashMap<String, String>> {
    match config.getImages() {
        Some(images) => images.iter().map(replace_configs_by_hashes).collect(),
        None => HashMap::new(),
    }
}

fn replace_configs_by_hashes(
    (image_name, image_tags): (&String, &HashMap<String, OCIImageConfig>),
) -> (String, HashMap<String, String>) {
    if image_tags.is_empty() {
        eprintln!("No tags are defined for image {}", image_name);
        exit(exitcode::DATAERR)
    }

    (
        image_name.clone(),
        image_tags
            .iter()
            .map(|(image_tag, _)| (image_tag, format!("{}:{}", image_name, image_tag)))
            .map(get_image_hash_by_tag)
            .collect(),
    )
}

fn get_image_hash_by_tag((image_tag, image_fqn): (&String, String)) -> (String, String) {
    match Command::new("docker")
        .args(&["inspect", "--format='{{index .RepoDigests 0}}'", &image_fqn])
        .output()
    {
        Ok(output) => match output.status.success() {
            true => match from_utf8(&output.stdout) {
                Ok(stdout) => match stdout.trim().split(":").nth(1) {
                    Some(hash) => (image_tag.clone(), hash.to_string()),
                    None => {
                        eprintln!("The command `docker inspect --format='{{index .RepoDigests 0}}' {}` returned an unexpected output", image_fqn);
                        exit(exitcode::PROTOCOL)
                    }
                },
                Err(e) => {
                    eprintln!("The command `docker inspect --format='{{index .RepoDigests 0}}' {}` returned an unexpected output.\n\n{}\n", image_fqn, e.to_string());
                    exit(exitcode::PROTOCOL)
                }
            },
            false => match Command::new("docker")
                .args(&["image", "pull", &image_fqn])
                .status()
            {
                Ok(status) => match status.success() {
                    true => get_image_hash_by_tag((image_tag, image_fqn)),
                    false => {
                        eprintln!("Unable to pull OCI image {}", image_fqn);
                        exit(exitcode::UNAVAILABLE)
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Unknow error while trying to pull OCI image {}:\n\n{}\n",
                        image_fqn,
                        e.to_string()
                    );
                    exit(exitcode::OSERR)
                }
            },
        },
        Err(e) => {
            eprintln!(
                "Unknow error while trying to inspect OCI image {}:\n\n{}\n",
                image_fqn,
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }
}

fn get_binaries_settings(
    config: &ProjectConfig,
    images_name_tag_hash_rel: &HashMap<String, HashMap<String, String>>,
) -> HashMap<String, ImageBinaryConfigLock> {
    let mut dst_binaries: HashMap<String, ImageBinaryConfigLock> = HashMap::new();

    if let Some(images) = config.getImages() {
        for (image_name, image_tags) in images {
            for (image_tag, image_config) in image_tags {
                match image_config.getBinaries() {
                    Some(src_binaries) => {
                        for (binary_name, binary_config) in src_binaries {
                            let image_hash = match images_name_tag_hash_rel.get(image_name) {
                                Some(images_tag_hash_rel) => {
                                    match images_tag_hash_rel.get(image_tag) {
                                        Some(_image_hash) => _image_hash,
                                        None => {
                                            eprintln!(
                                                "A theoretically impossible error just happened."
                                            );
                                            exit(exitcode::SOFTWARE)
                                        }
                                    }
                                }
                                None => {
                                    eprintln!("A theoretically impossible error just happened.");
                                    exit(exitcode::SOFTWARE)
                                }
                            };

                            dst_binaries.insert(
                                binary_name.clone(),
                                ImageBinaryConfigLock::new(
                                    image_name.clone(),
                                    image_hash.clone(),
                                    binary_config.getPath().clone(),
                                ),
                            );
                        }
                    }
                    None => { /* Do nothing */ }
                }
            }
        }
    }

    dst_binaries
}

fn check_oci_images_availability(project_state: &ProjectConfigLock) -> () {
    let images = project_state.getImages();

    if let Err(_) = which::which("docker") {
        eprintln!("docker client is not available");
        exit(exitcode::UNAVAILABLE)
    }

    for (image_name, image_tags) in images.iter() {
        for (_, image_hash) in image_tags.iter() {
            let inspect_output = Command::new("docker")
                .args(&["inspect", &format!("{}@sha256:{}", image_name, image_hash)])
                .output();

            match inspect_output {
                Ok(output) => {
                    if !output.status.success() {
                        pull_oci_image_by_hash(format!("{}@sha256:{}", image_name, image_hash))
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Unable to use docker to inspect image {}@sha256:{}.\n\n{}\n",
                        image_name,
                        image_hash,
                        err.to_string()
                    );
                    exit(exitcode::OSERR)
                }
            }
        }
    }
}

fn pull_oci_image_by_hash(image_ref: String) -> () {
    // This code assumes that the existence of the docker command has been checked before
    if let Err(err) = Command::new("docker").args(&["pull", &image_ref]).status() {
        eprintln!(
            "Unable to pull image {}.\n\n{}\n",
            image_ref,
            err.to_string()
        );
        exit(exitcode::UNAVAILABLE)
    }
}
