/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::{
    collections::BTreeMap,
    env,
    fs::{create_dir_all, remove_dir_all, write},
    os::unix::fs::symlink,
    path::PathBuf,
    process::{exit, Command},
    str::from_utf8,
};

use duct::cmd;
use ring::digest::{digest, Digest, SHA256};

use crate::{
    avatar_env::SESSION_TOKEN,
    directories::{
        get_project_path, AVATARFILE_LOCK_NAME, AVATARFILE_NAME, CONFIG_DIR_NAME,
        CONTAINER_HOME_PATH, STATEFILE_NAME, VOLATILE_DIR_NAME,
    },
    docker::ERROR_MSG_DOCKER_INSPECT_OUTPUT,
    project_config::{
        get_config, get_config_lock, merge_run_and_shell_configs, save_config_lock,
        ImageBinaryConfig, ImageBinaryConfigLock, OCIContainerRunConfig, OCIImageConfig,
        OCIImageTagConfigLock, ProjectConfig, ProjectConfigLock, VolumeConfigLock,
    },
};

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

fn check_etc_passwd_files(
    volatile_path: &PathBuf,
    project_state: &ProjectConfigLock,
    changed_state: bool,
) {
    if which::which("tar").is_err() {
        eprintln!("WARNING: tar tool is not available, and passwd files won't be generated to improve integration with ssh-agent");
        return;
    }

    let images_path = match recreate_volatile_subdir(volatile_path, "images", changed_state) {
        Some(_images_path) => _images_path,
        None => return,
    };

    let project_internal_id = project_state.get_project_internal_id();
    let project_filter = format!("{}.byid.projects.avatar-cli", project_internal_id);

    let uid = nix::unistd::getuid();
    let (username, gid) = match nix::unistd::User::from_uid(uid) {
        Ok(Some(user)) => (user.name, user.gid),
        _ => {
            eprintln!("Unable to get current user name");
            exit(exitcode::OSERR)
        }
    };

    let mut errors = false;
    for (image_name, image_tags) in project_state.get_images() {
        for (image_tag, image_config) in image_tags {
            let image_hash = image_config.get_hash();
            let image_ref = format!("{}@sha256:{}", image_name, image_hash);
            let image_config_path = images_path.join(&image_ref);

            if create_dir_all(&image_config_path).is_err() {
                eprintln!("Unable to create directory {}", image_config_path.display());
                exit(exitcode::CANTCREAT)
            }

            let install_container_name = format!(
                "{}_{}_{}_{}_passwd",
                project_internal_id,
                image_name.replace('/', "."),
                image_tag,
                image_hash
            );
            match Command::new("docker")
                .args(&[
                    "create",
                    "--name",
                    &install_container_name,
                    "--label",
                    "avatar_cli",
                    "--label",
                    &project_filter,
                    "--label",
                    "install_helper.container_role.avatar-cli",
                    &image_ref,
                ])
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!(
                            "Unable to create temporary install container\n\n{}",
                            from_utf8(&output.stderr).unwrap()
                        );
                        errors = true;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Unable to create temporary install container\n\n{}\n",
                        e.to_string()
                    );
                    errors = true;
                    break;
                }
            }

            let container_files_list = match cmd!("docker", "export", &install_container_name)
                .pipe(cmd!("tar", "t"))
                .read()
            {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(
                        "Unable to list contents of container {}\n\n{}\n",
                        &install_container_name,
                        e.to_string()
                    );
                    errors = true;
                    break;
                }
            };

            // TODO: fish, and others
            let mut found_passwd = false;
            let mut found_bash = false;
            let mut found_csh = false;
            let mut found_dash = false;
            let mut found_ksh = false;
            let mut found_zsh = false;
            for file_name in container_files_list.lines() {
                match file_name.trim() {
                    "etc/passwd" => found_passwd = true,
                    "bin/bash" => found_bash = true,
                    "bin/csh" => found_csh = true,
                    "bin/dash" => found_dash = true,
                    "bin/ksh" => found_ksh = true,
                    "bin/zsh" => found_zsh = true,
                    _ => {}
                }
            }
            let inferred_passwd_shell = if found_bash {
                "/bin/bash"
            } else if found_zsh {
                "/bin/zsh"
            } else if found_dash {
                "/bin/dash"
            } else if found_ksh {
                "/bin/ksh"
            } else if found_csh {
                "/bin/csh"
            } else {
                "/bin/sh"
            };

            let local_etc_passwd_path = image_config_path.join("passwd");
            if !found_passwd {
                if let Err(e) = write(
                    &local_etc_passwd_path,
                    format!(
                        "{}:x:{}:{}::{}:{}\n",
                        username, uid, gid, CONTAINER_HOME_PATH, inferred_passwd_shell
                    )
                    .as_bytes(),
                ) {
                    eprintln!(
                        "Unable to create custom passwd file for {}\n\n{}\n",
                        &image_ref,
                        e.to_string()
                    );
                    errors = true;
                    break;
                }
            } else {
                let passwd_src_contents = match cmd!("docker", "export", &install_container_name)
                    .pipe(cmd!("tar", "--extract", "-O", "etc/passwd"))
                    .read()
                {
                    Ok(_contents) => _contents,
                    Err(e) => {
                        eprintln!(
                            "Unable to export passwd file from {} image\n\n{}\n",
                            image_ref,
                            e.to_string()
                        );
                        errors = true;
                        break;
                    }
                };

                let mut found_user_line = false;
                let mut passwd_dst_contents = String::with_capacity(passwd_src_contents.len());

                for user_line in passwd_src_contents.lines() {
                    let trimmed_user_line = user_line.trim();
                    let mut user_line_parts = trimmed_user_line.split(':');
                    if let Some(passwd_uid) = user_line_parts.nth(2) {
                        if passwd_uid == uid.to_string() {
                            let passwd_shell = match user_line_parts.last() {
                                Some(_passwd_shell) => _passwd_shell,
                                None => inferred_passwd_shell,
                            };

                            found_user_line = true;
                            passwd_dst_contents.push_str(&format!(
                                "{}:x:{}:{}::{}:{}\n",
                                username, uid, gid, CONTAINER_HOME_PATH, passwd_shell
                            ))
                        } else {
                            passwd_dst_contents.push_str(trimmed_user_line);
                            passwd_dst_contents.push('\n')
                        }
                    } else {
                        eprintln!("Unable to process exported passwd file from {} image, found corrupted line:\n\n{}\n", image_ref, user_line);
                        errors = true;
                        break;
                    }
                }
                if !found_user_line {
                    passwd_dst_contents.push_str(&format!(
                        "{}:x:{}:{}::{}:{}\n",
                        username, uid, gid, CONTAINER_HOME_PATH, inferred_passwd_shell
                    ))
                }
                if let Err(e) = write(&local_etc_passwd_path, passwd_dst_contents.as_bytes()) {
                    eprintln!(
                        "Unable to create custom passwd file for {}\n\n{}\n",
                        &image_ref,
                        e.to_string()
                    );
                    errors = true;
                    break;
                }
            }
        }
    }

    if let Err(e) = Command::new("docker")
        .args(&[
            "container",
            "prune",
            "--force",
            "--filter",
            &format!("label={}", project_filter),
            "--filter",
            "label=install_helper.container_role.avatar-cli",
        ])
        .output()
    {
        eprintln!(
            "Unable to prune containers generated during install step\n\n{}\n",
            e.to_string()
        );
        errors = true;
    }

    if errors {
        exit(exitcode::IOERR)
    }
}

fn check_managed_volumes_availability(project_state: &ProjectConfigLock) {
    for (_, binary_config) in project_state.get_binaries_configs() {
        if let Some(run_config) = binary_config.get_run_config() {
            if let Some(volume_configs) = run_config.get_volumes() {
                volume_configs.iter().for_each(|vc| {
                    check_managed_volume_existence(vc, project_state.get_project_internal_id())
                });
            }
        }
    }
}

fn check_managed_volume_existence(volume_config: &VolumeConfigLock, project_internal_id: &str) {
    match Command::new("docker")
        .args(&["volume", "inspect", volume_config.get_name()])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                create_volume(
                    volume_config.get_name(),
                    volume_config.get_container_path(),
                    project_internal_id,
                );
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

fn check_oci_images_availability(project_state: &ProjectConfigLock, show_output: bool) -> bool {
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
                        pull_oci_image_by_fqn(
                            &format!("{}@sha256:{}", image_name, image_config.get_hash()),
                            show_output,
                        );
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

fn check_project_settings(
    config_path: &PathBuf,
    config_lock_path: &PathBuf,
    project_state_path: &PathBuf,
    show_output: bool,
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
                generate_config_lock(config_lock_path, &config, &config_hash, show_output)
            } else {
                (_config_lock, _config_lock_hash)
            }
        }
        false => {
            changed_state = true;
            generate_config_lock(config_lock_path, &config, &config_hash, show_output)
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

fn compile_image_configs(
    (image_name, image_config, show_output): (&String, &OCIImageConfig, bool),
) -> (String, BTreeMap<String, OCIImageTagConfigLock>) {
    let tags = image_config.get_tags();

    if tags.is_empty() {
        eprintln!("No tags are defined for image {}", image_name);
        exit(exitcode::DATAERR)
    }

    (
        image_name.clone(),
        tags.iter()
            .map(|(image_tag, image_tag_config)| {
                (
                    image_name,
                    image_tag,
                    image_tag_config.get_run_config().clone(),
                    show_output,
                )
            })
            .map(get_image_config_by_tag)
            .collect(),
    )
}

fn create_volume(volume_name: &str, container_path: &PathBuf, project_internal_id: &str) {
    let project_filter = format!("{}.byid.projects.avatar-cli", project_internal_id);

    match Command::new("docker")
        .args(&[
            "volume",
            "create",
            volume_name,
            "--label",
            "avatar_cli",
            "--label",
            &project_filter,
        ])
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

fn generate_config_lock(
    config_lock_path: &PathBuf,
    config: &ProjectConfig,
    config_hash: &Digest,
    show_output: bool,
) -> (ProjectConfigLock, Digest) {
    let image_configs = get_image_compiled_configs(config, show_output);
    let binaries_settings = get_binaries_settings(config, &image_configs);

    let config_lock = ProjectConfigLock::new(
        Vec::<u8>::from(config_hash.as_ref()),
        config.get_project_internal_id().clone(),
        config.get_shell_config().clone(),
        image_configs,
        binaries_settings,
    );

    let config_lock_bytes = save_config_lock(config_lock_path, &config_lock);
    (config_lock, digest(&SHA256, &config_lock_bytes))
}

fn get_binaries_settings(
    config: &ProjectConfig,
    images_name_tag_hash_rel: &BTreeMap<String, BTreeMap<String, OCIImageTagConfigLock>>,
) -> BTreeMap<String, ImageBinaryConfigLock> {
    let mut dst_binaries: BTreeMap<String, ImageBinaryConfigLock> = BTreeMap::new();

    if let Some(images) = config.get_images() {
        for (image_name, image_config) in images {
            set_binaries_settings_from_image_tags(
                &mut dst_binaries,
                image_name,
                image_config,
                config,
                images_name_tag_hash_rel,
            );
        }
    }

    dst_binaries
}

fn get_image_compiled_configs(
    config: &ProjectConfig,
    show_output: bool,
) -> BTreeMap<String, BTreeMap<String, OCIImageTagConfigLock>> {
    match config.get_images() {
        Some(images) => images
            .iter()
            .map(|(image_name, image_tags)| {
                compile_image_configs((image_name, image_tags, show_output))
            })
            .collect(),
        None => BTreeMap::new(),
    }
}

fn get_image_config_by_tag(
    (image_name, image_tag, run_config, show_output): (
        &String,
        &String,
        Option<OCIContainerRunConfig>,
        bool,
    ),
) -> (String, OCIImageTagConfigLock) {
    let image_fqn = format!("{}:{}", image_name, image_tag);

    match Command::new("docker")
        .args(&[
            "inspect",
            "--format={{range .RepoDigests}}{{println .}}{{end}}",
            &image_fqn,
        ])
        .output()
    {
        Ok(output) => match output.status.success() {
            true => match from_utf8(&output.stdout) {
                Ok(stdout) => {
                    let hash = get_hash_from_repo_digests_str(stdout, image_name);
                    (
                        image_tag.clone(),
                        OCIImageTagConfigLock::new(hash, run_config),
                    )
                }
                Err(e) => {
                    eprintln!(
                        "{}.\n\n{}\n",
                        ERROR_MSG_DOCKER_INSPECT_OUTPUT,
                        e.to_string()
                    );
                    exit(exitcode::PROTOCOL)
                }
            },
            false => {
                pull_oci_image_by_fqn(&image_fqn, show_output);
                get_image_config_by_tag((image_name, image_tag, run_config, show_output))
            }
        },
        Err(e) => {
            eprintln!(
                "Unknow error while trying to inspect OCI image {}:\n\n{}\n",
                &image_fqn,
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }
}

fn get_hash_from_repo_digests_str(repo_difests_str: &str, image_name: &str) -> String {
    let repo_digests = repo_difests_str.trim().split('\n');

    for repo_digest in repo_digests {
        if let Some(repo_digest_name) = repo_digest.split('@').next() {
            if repo_digest_name == image_name {
                match repo_digest.split(':').nth(1) {
                    Some(hash) => return hash.to_string(),
                    None => {
                        eprintln!("{}", ERROR_MSG_DOCKER_INSPECT_OUTPUT);
                        exit(exitcode::PROTOCOL)
                    }
                }
            }
        }
    }

    eprintln!("{}", ERROR_MSG_DOCKER_INSPECT_OUTPUT);
    exit(exitcode::PROTOCOL)
}

pub(crate) fn install_subcommand(
    show_output: bool,
) -> (PathBuf, PathBuf, PathBuf, PathBuf, ProjectConfigLock) {
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

    let project_data_path = project_path.join(CONFIG_DIR_NAME);
    let config_path = project_data_path.join(AVATARFILE_NAME);
    let config_lock_path = project_data_path.join(AVATARFILE_LOCK_NAME);
    let volatile_path = project_data_path.join(VOLATILE_DIR_NAME);
    let project_state_path = volatile_path.join(STATEFILE_NAME);

    let (project_state, changed_state) = check_project_settings(
        &config_path,
        &config_lock_path,
        &project_state_path,
        show_output,
    );
    let pulled_oci_images = check_oci_images_availability(&project_state, show_output);
    check_managed_volumes_availability(&project_state);
    populate_volatile_bin_dir(
        &volatile_path,
        &project_state,
        pulled_oci_images || changed_state,
    );
    populate_volatile_home_dir(&volatile_path, pulled_oci_images || changed_state);
    check_etc_passwd_files(
        &volatile_path,
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

fn populate_volatile_bin_dir(
    volatile_path: &PathBuf,
    project_state: &ProjectConfigLock,
    changed_state: bool,
) {
    let bin_path = match recreate_volatile_subdir(volatile_path, "bin", changed_state) {
        Some(_bin_path) => _bin_path,
        None => return,
    };

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

    for binary_name in project_state.get_binary_names() {
        if symlink(&avatar_path, bin_path.join(binary_name)).is_err() {
            eprintln!("Unable to create symlink to {} binary", binary_name);
            exit(exitcode::CANTCREAT)
        }
    }
}

fn populate_volatile_home_dir(volatile_path: &PathBuf, changed_state: bool) {
    recreate_volatile_subdir(volatile_path, "home", changed_state);
}

fn pull_oci_image_by_fqn(image_ref: &str, show_output: bool) {
    // This code assumes that the existence of the docker command has been checked before
    if show_output {
        match Command::new("docker").args(&["pull", image_ref]).status() {
            Ok(status) => {
                if !status.success() {
                    eprintln!("Unable to pull OCI image {}", image_ref);
                    exit(exitcode::UNAVAILABLE)
                }
            }
            Err(err) => {
                eprintln!(
                    "Unable to pull OCI image {}.\n\n{}\n",
                    image_ref,
                    err.to_string()
                );
                exit(exitcode::OSERR)
            }
        }
    } else {
        match Command::new("docker").args(&["pull", image_ref]).output() {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("Unable to pull OCI image {}", image_ref);
                    exit(exitcode::UNAVAILABLE)
                }
            }
            Err(err) => {
                eprintln!(
                    "Unable to pull OCI image {}.\n\n{}\n",
                    image_ref,
                    err.to_string()
                );
                exit(exitcode::OSERR)
            }
        }
    }
}

fn recreate_volatile_subdir(
    volatile_path: &PathBuf,
    subdir_name: &str,
    changed_state: bool,
) -> Option<PathBuf> {
    let subdir_path = volatile_path.join(subdir_name);

    if subdir_path.exists() {
        if !subdir_path.is_dir() {
            eprintln!(
                "The path {} must be a directory, but found something else",
                subdir_path.display()
            );
            exit(exitcode::USAGE)
        }

        if !changed_state {
            return None;
        }

        if let Err(e) = remove_dir_all(&subdir_path) {
            eprintln!(
                "Unable to delete broken directory {}\n\n{}\n",
                subdir_path.display(),
                e.to_string()
            );
            exit(exitcode::IOERR)
        }
    }

    if create_dir_all(&subdir_path).is_err() {
        eprintln!("Unable to create directory {}", subdir_path.display());
        exit(exitcode::CANTCREAT)
    }

    Some(subdir_path)
}

fn set_binaries_settings_from_binaries_defs(
    dst_binaries: &mut BTreeMap<String, ImageBinaryConfigLock>,
    image_name: &String,
    image_tag: &str,
    src_binaries: &BTreeMap<String, ImageBinaryConfig>,
    config: &ProjectConfig,
    images_name_tag_hash_rel: &BTreeMap<String, BTreeMap<String, OCIImageTagConfigLock>>,
) {
    for (binary_name, binary_config) in src_binaries {
        let image_tag_config = match images_name_tag_hash_rel.get(image_name) {
            Some(images_tag_config_rel) => match images_tag_config_rel.get(image_tag) {
                Some(_image_tag_config) => _image_tag_config,
                None => {
                    eprintln!("A theoretically impossible error just happened.");
                    exit(exitcode::SOFTWARE)
                }
            },
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
                image_tag_config.get_hash().clone(),
                binary_config
                    .get_path()
                    .clone()
                    .unwrap_or(PathBuf::from(binary_name)),
                merge_run_and_shell_configs(
                    image_tag_config.get_run_config(),
                    binary_config.get_run_config(),
                    config.get_shell_config(),
                    config.get_project_internal_id(),
                    image_name,
                    image_tag,
                    image_tag_config.get_hash(),
                    binary_name,
                ),
            ),
        );
    }
}

fn set_binaries_settings_from_image_tags(
    dst_binaries: &mut BTreeMap<String, ImageBinaryConfigLock>,
    image_name: &String,
    image_config: &OCIImageConfig,
    config: &ProjectConfig,
    images_name_tag_hash_rel: &BTreeMap<String, BTreeMap<String, OCIImageTagConfigLock>>,
) {
    for (image_tag, image_tag_config) in image_config.get_tags() {
        match image_tag_config.get_binaries() {
            Some(src_binaries) => {
                set_binaries_settings_from_binaries_defs(
                    dst_binaries,
                    image_name,
                    image_tag,
                    src_binaries,
                    config,
                    images_name_tag_hash_rel,
                );
            }
            None => { /* Do nothing */ }
        }
    }
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
