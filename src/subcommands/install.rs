/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::{
    collections::BTreeMap,
    env,
    fs::{copy, create_dir_all, remove_dir_all},
    os::unix::fs::symlink,
    path::PathBuf,
    process::{exit, Command},
    str::from_utf8,
};

use ring::digest::{digest, Digest, SHA256};

use crate::{
    avatar_env::SESSION_TOKEN,
    directories::get_project_path,
    project_config::{
        get_config, get_config_lock, merge_run_configs, save_config_lock, ImageBinaryConfigLock,
        OCIContainerRunConfig, OCIImageConfig, OCIImageConfigLock, ProjectConfig,
        ProjectConfigLock, VolumeConfigLock,
    },
};

pub(crate) fn install_subcommand() -> (PathBuf, PathBuf, PathBuf, PathBuf, ProjectConfigLock) {
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

    let config_path = project_path.join(".avatar-cli").join("avatar-cli.yml");
    let config_lock_path = project_path.join(".avatar-cli").join("avatar-cli.lock.yml");
    let project_state_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("state.yml");

    let (project_state, changed_state) =
        check_project_settings(&config_path, &config_lock_path, &project_state_path);
    let pulled_oci_images = check_oci_images_availability(&project_state);
    check_managed_volumes_availability(&project_state);
    populate_volatile_bin_dir(
        &project_path,
        &project_state,
        pulled_oci_images || changed_state,
    );

    (
        project_path,
        config_path,
        config_lock_path,
        project_state_path,
        project_state,
    )
}

fn check_project_settings(
    config_path: &PathBuf,
    config_lock_path: &PathBuf,
    project_state_path: &PathBuf,
) -> (ProjectConfigLock, bool) {
    let mut changed_state = false;
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

            if config_hash.as_ref() != &_config_lock.get_project_config_hash()[..] {
                changed_state = true;
                generate_config_lock(config_lock_path, &config, &config_hash)
            } else {
                (_config_lock, _config_lock_hash)
            }
        }
        false => {
            changed_state = true;
            generate_config_lock(config_lock_path, &config, &config_hash)
        }
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

            let (_project_state, _) = get_config_lock(&project_state_path);

            if config_lock_hash.as_ref() != &_project_state.get_project_config_hash()[..] {
                changed_state = true;
                update_project_state(project_state_path, config_lock, config_lock_hash.as_ref())
            } else {
                _project_state
            }
        }
        false => {
            changed_state = true;

            let volatile_dir = project_state_path.parent().unwrap();
            if !volatile_dir.exists() && create_dir_all(volatile_dir).is_err() {
                eprintln!("Unable to create directory {}", volatile_dir.display());
                exit(exitcode::CANTCREAT)
            }

            update_project_state(project_state_path, config_lock, config_lock_hash.as_ref())
        }
    };

    (project_state, changed_state)
}

fn generate_config_lock(
    config_lock_path: &PathBuf,
    config: &ProjectConfig,
    config_hash: &Digest,
) -> (ProjectConfigLock, Digest) {
    let image_configs = get_image_compiled_configs(config);
    let binaries_settings = get_binaries_settings(config, &image_configs);

    let config_lock = ProjectConfigLock::new(
        Vec::<u8>::from(config_hash.as_ref()),
        config.get_project_internal_id().clone(),
        image_configs,
        binaries_settings,
    );

    let config_lock_bytes = save_config_lock(config_lock_path, &config_lock);
    (config_lock, digest(&SHA256, &config_lock_bytes))
}

fn update_project_state(
    project_state_path: &PathBuf,
    mut project_state: ProjectConfigLock,
    config_lock_hash: &[u8],
) -> ProjectConfigLock {
    project_state = project_state.update_project_config_hash(config_lock_hash);
    save_config_lock(project_state_path, &project_state);
    project_state
}

fn get_image_compiled_configs(
    config: &ProjectConfig,
) -> BTreeMap<String, BTreeMap<String, OCIImageConfigLock>> {
    match config.get_images() {
        Some(images) => images.iter().map(compile_image_configs).collect(),
        None => BTreeMap::new(),
    }
}

fn compile_image_configs(
    (image_name, image_tags): (&String, &BTreeMap<String, OCIImageConfig>),
) -> (String, BTreeMap<String, OCIImageConfigLock>) {
    if image_tags.is_empty() {
        eprintln!("No tags are defined for image {}", image_name);
        exit(exitcode::DATAERR)
    }

    (
        image_name.clone(),
        image_tags
            .iter()
            .map(|(image_tag, image_config)| {
                (
                    image_tag,
                    format!("{}:{}", image_name, image_tag),
                    image_config.get_run_config().clone(),
                )
            })
            .map(get_image_config_by_tag)
            .collect(),
    )
}

fn get_image_config_by_tag(
    (image_tag, image_fqn, run_config): (&String, String, Option<OCIContainerRunConfig>),
) -> (String, OCIImageConfigLock) {
    match Command::new("docker")
        .args(&["inspect", "--format={{index .RepoDigests 0}}", &image_fqn])
        .output()
    {
        Ok(output) => match output.status.success() {
            true => match from_utf8(&output.stdout) {
                Ok(stdout) => match stdout.trim().split(':').nth(1) {
                    Some(hash) => (
                        image_tag.clone(),
                        OCIImageConfigLock::new(hash.to_string(), run_config),
                    ),
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
                    true => get_image_config_by_tag((image_tag, image_fqn, run_config)),
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
    images_name_tag_hash_rel: &BTreeMap<String, BTreeMap<String, OCIImageConfigLock>>,
) -> BTreeMap<String, ImageBinaryConfigLock> {
    let mut dst_binaries: BTreeMap<String, ImageBinaryConfigLock> = BTreeMap::new();

    if let Some(images) = config.get_images() {
        for (image_name, image_tags) in images {
            for (image_tag, image_config) in image_tags {
                match image_config.get_binaries() {
                    Some(src_binaries) => {
                        for (binary_name, binary_config) in src_binaries {
                            let image_config = match images_name_tag_hash_rel.get(image_name) {
                                Some(images_tag_config_rel) => {
                                    match images_tag_config_rel.get(image_tag) {
                                        Some(_image_config) => _image_config,
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

                            if dst_binaries.contains_key(binary_name) {
                                eprintln!("Duplicated binary definition for '{}'", binary_name);
                                exit(exitcode::DATAERR)
                            }

                            dst_binaries.insert(
                                binary_name.clone(),
                                ImageBinaryConfigLock::new(
                                    image_name.clone(),
                                    image_config.get_hash().clone(),
                                    binary_config.get_path().clone(),
                                    merge_run_configs(
                                        image_config.get_run_config(),
                                        binary_config.get_run_config(),
                                        config.get_project_internal_id(),
                                        &format!("{}-{}", image_name, image_tag),
                                        binary_name,
                                    ),
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

fn check_oci_images_availability(project_state: &ProjectConfigLock) -> bool {
    let images = project_state.get_images();

    if which::which("docker").is_err() {
        eprintln!("docker client is not available");
        exit(exitcode::UNAVAILABLE)
    }

    let mut changed_state = false;

    for (image_name, image_tags) in images.iter() {
        for (_, image_config) in image_tags.iter() {
            let inspect_output = Command::new("docker")
                .args(&[
                    "inspect",
                    &format!("{}@sha256:{}", image_name, image_config.get_hash()),
                ])
                .output();

            match inspect_output {
                Ok(output) => {
                    if !output.status.success() {
                        pull_oci_image_by_fqn(format!(
                            "{}@sha256:{}",
                            image_name,
                            image_config.get_hash()
                        ));
                        changed_state = true;
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Unable to use docker to inspect image {}@sha256:{}.\n\n{}\n",
                        image_name,
                        image_config.get_hash(),
                        err.to_string()
                    );
                    exit(exitcode::OSERR)
                }
            }
        }
    }

    changed_state
}

fn pull_oci_image_by_fqn(image_ref: String) {
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

fn populate_volatile_bin_dir(
    project_path: &PathBuf,
    project_state: &ProjectConfigLock,
    changed_state: bool,
) {
    let bin_path = project_path
        .join(".avatar-cli")
        .join("volatile")
        .join("bin");

    if bin_path.exists() {
        if !bin_path.is_dir() {
            eprintln!("");
            exit(exitcode::USAGE)
        }

        if !changed_state {
            return;
        }

        if let Err(e) = remove_dir_all(&bin_path) {
            eprintln!(
                "Unable to delete broken bin directory {}\n\n{}\n",
                bin_path.display(),
                e.to_string()
            );
            exit(exitcode::IOERR)
        }
    }

    if create_dir_all(&bin_path).is_err() {
        eprintln!("Unable to create directory {}", bin_path.display());
        exit(exitcode::CANTCREAT)
    }

    let avatar_path = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "Unable to retrieve avatar's binary path.\n\n{}\n",
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    };

    let managed_avatar_path = bin_path.join("avatar");

    if let Err(e) = copy(&avatar_path, &managed_avatar_path) {
        eprintln!(
            "Unable to copy avatar binary to {}\n\n{}\n",
            bin_path.display(),
            e.to_string()
        );
        exit(exitcode::IOERR)
    }

    for binary_name in project_state.get_binary_names() {
        if symlink(&managed_avatar_path, bin_path.join(binary_name)).is_err() {
            eprintln!("Unable to create symlink to {} binary", binary_name);
            exit(exitcode::CANTCREAT)
        }
    }
}

fn check_managed_volumes_availability(project_state: &ProjectConfigLock) {
    for (_, binary_config) in project_state.get_binaries_configs() {
        if let Some(run_config) = binary_config.get_run_config() {
            if let Some(volume_configs) = run_config.get_volumes() {
                volume_configs
                    .iter()
                    .for_each(check_managed_volume_existence);
            }
        }
    }
}

fn check_managed_volume_existence(volume_config: &VolumeConfigLock) {
    match Command::new("docker")
        .args(&["volume", "inspect", volume_config.get_name()])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                create_volume(volume_config.get_name(), volume_config.get_container_path());
            }
        }
        Err(e) => {
            eprintln!(
                "Unable to inspect volume {}\n\n{}\n",
                volume_config.get_name(),
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }
}

fn create_volume(volume_name: &str, container_path: &PathBuf) {
    match Command::new("docker")
        .args(&["volume", "create", volume_name])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Unable to create volume {}", volume_name);
                exit(exitcode::SOFTWARE);
            }

            change_volume_permissions(volume_name, container_path)
        }
        Err(e) => {
            eprintln!(
                "Unable to create volume {}\n\n{}\n",
                volume_name,
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }
}

fn change_volume_permissions(volume_name: &str, container_path: &PathBuf) {
    match Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--volume",
            &format!("{}:{}", volume_name, container_path.display()),
            "alpine:3.12",
            "sh",
            "-c",
            &format!(
                "chown -R {}:{} {}",
                nix::unistd::getuid(),
                nix::unistd::getgid(),
                container_path.display()
            ),
        ])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Unable to change permissions for volume {}", volume_name);
                exit(exitcode::SOFTWARE);
            }
        }
        Err(e) => {
            eprintln!(
                "Unable to change permissions for volume {}\n\n{}\n",
                volume_name,
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }
}
