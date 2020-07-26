/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::collections::{HashMap, HashSet};
use std::fs::{read, write};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::exit;
use std::vec::Vec;

extern crate ring;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use ring::digest::{digest, Digest, SHA256};
use serde::{Deserialize, Serialize};

use crate::subcommands::AVATAR_CLI_VERSION;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VolumeConfig {
    container_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BindingConfig {
    host_path: PathBuf,
    container_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OCIContainerRunConfig {
    env: Option<HashMap<String, String>>,
    env_from_host: Option<HashSet<String>>,
    volumes: Option<Vec<VolumeConfig>>,
    bindings: Option<Vec<BindingConfig>>,
}

impl OCIContainerRunConfig {
    pub fn get_env(&self) -> &Option<HashMap<String, String>> {
        &self.env
    }

    pub fn get_env_from_host(&self) -> &Option<HashSet<String>> {
        &self.env_from_host
    }

    pub fn get_volumes(&self) -> &Option<Vec<VolumeConfig>> {
        &self.volumes
    }

    pub fn get_bindings(&self) -> &Option<Vec<BindingConfig>> {
        &self.bindings
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageBinaryConfig {
    path: PathBuf,
    run_config: Option<OCIContainerRunConfig>,
}

impl ImageBinaryConfig {
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_run_config(&self) -> &Option<OCIContainerRunConfig> {
        &self.run_config
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OCIImageConfig {
    binaries: Option<HashMap<String, ImageBinaryConfig>>,
    run_config: Option<OCIContainerRunConfig>,
}

impl OCIImageConfig {
    pub fn get_binaries(&self) -> &Option<HashMap<String, ImageBinaryConfig>> {
        &self.binaries
    }

    pub fn get_run_config(&self) -> &Option<OCIContainerRunConfig> {
        &self.run_config
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectConfig {
    version: String,
    project_internal_id: String,
    images: Option<HashMap<String, HashMap<String, OCIImageConfig>>>, // image name -> image tag -> oci image config
}

impl ProjectConfig {
    pub fn new() -> ProjectConfig {
        let prj_internal_id: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();

        ProjectConfig {
            version: AVATAR_CLI_VERSION.to_string(),
            project_internal_id: prj_internal_id,
            images: None,
        }
    }

    pub fn get_project_internal_id(&self) -> &String {
        &self.project_internal_id
    }

    pub fn get_images(&self) -> &Option<HashMap<String, HashMap<String, OCIImageConfig>>> {
        &self.images
    }
}

// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VolumeConfigLock {
    container_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OCIContainerRunConfigLock {
    env: Option<HashMap<String, String>>,
    env_from_host: Option<HashSet<String>>,
    volumes: Option<Vec<VolumeConfigLock>>,
    bindings: Option<Vec<BindingConfig>>,
}

impl OCIContainerRunConfigLock {
    pub fn get_env(&self) -> &Option<HashMap<String, String>> {
        &self.env
    }

    pub fn get_env_from_host(&self) -> &Option<HashSet<String>> {
        &self.env_from_host
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageBinaryConfigLock {
    oci_image_name: String,
    oci_image_hash: String,
    path: PathBuf,
    run_config: Option<OCIContainerRunConfigLock>,
}

impl ImageBinaryConfigLock {
    pub fn new(
        oci_image_name: String,
        oci_image_hash: String,
        path: PathBuf,
        run_config: Option<OCIContainerRunConfigLock>,
    ) -> ImageBinaryConfigLock {
        ImageBinaryConfigLock {
            oci_image_name,
            oci_image_hash,
            path,
            run_config,
        }
    }

    pub fn get_oci_image_name(&self) -> &String {
        &self.oci_image_name
    }

    pub fn get_oci_image_hash(&self) -> &String {
        &self.oci_image_hash
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_run_config(&self) -> &Option<OCIContainerRunConfigLock> {
        &self.run_config
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OCIImageConfigLock {
    hash: String,
    run_config: Option<OCIContainerRunConfig>,
}

impl OCIImageConfigLock {
    pub fn new(hash: String, run_config: Option<OCIContainerRunConfig>) -> OCIImageConfigLock {
        OCIImageConfigLock { hash, run_config }
    }

    pub fn get_hash(&self) -> &String {
        &self.hash
    }

    pub fn get_run_config(&self) -> &Option<OCIContainerRunConfig> {
        &self.run_config
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectConfigLock {
    #[serde(with = "hex")]
    project_config_hash: Vec<u8>,
    project_internal_id: String,
    images: HashMap<String, HashMap<String, OCIImageConfigLock>>, // image_name -> image_tag -> image_hash
    binaries: HashMap<String, ImageBinaryConfigLock>,
}

impl ProjectConfigLock {
    pub fn get_project_config_hash(&self) -> &Vec<u8> {
        &self.project_config_hash
    }

    pub fn update_project_config_hash(mut self, new_hash: &[u8]) -> ProjectConfigLock {
        self.project_config_hash = Vec::from(new_hash);
        self
    }

    pub fn get_project_internal_id(&self) -> &String {
        &self.project_internal_id
    }

    pub fn get_images(&self) -> &HashMap<String, HashMap<String, OCIImageConfigLock>> {
        &self.images
    }

    pub fn get_binary_configuration(&self, binary_name: &str) -> Option<&ImageBinaryConfigLock> {
        self.binaries.get(binary_name)
    }

    pub fn get_binary_names(
        &self,
    ) -> std::collections::hash_map::Keys<'_, std::string::String, ImageBinaryConfigLock> {
        self.binaries.keys()
    }

    pub fn new(
        project_config_hash: Vec<u8>,
        project_internal_id: String,
        images: HashMap<String, HashMap<String, OCIImageConfigLock>>,
        binaries: HashMap<String, ImageBinaryConfigLock>,
    ) -> ProjectConfigLock {
        ProjectConfigLock {
            project_config_hash,
            project_internal_id,
            images,
            binaries,
        }
    }
}

// Functions:
// -----------------------------------------------------------------------------

fn get_file_bytes(filepath: &PathBuf) -> Vec<u8> {
    if !filepath.exists() || !filepath.is_file() {
        eprintln!("The file {} is not available", &filepath.display());
        exit(exitcode::NOINPUT)
    }

    match read(filepath) {
        Ok(s) => s,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                eprintln!("The file {} is not available", filepath.display());
                exit(exitcode::NOINPUT)
            }
            ErrorKind::PermissionDenied => {
                eprintln!(
                    "The file {} is not readable due to filesystem permissions",
                    filepath.display()
                );
                exit(exitcode::IOERR)
            }
            _ => {
                eprintln!(
                    "Unknown IO error while reading the file {}",
                    filepath.display()
                );
                exit(exitcode::IOERR)
            }
        },
    }
}

pub(crate) fn get_config_lock(config_lock_filepath: &PathBuf) -> (ProjectConfigLock, Digest) {
    let config_lock_bytes = get_file_bytes(config_lock_filepath);

    (
        match serde_yaml::from_slice::<ProjectConfigLock>(&config_lock_bytes) {
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
        },
        digest(&SHA256, &config_lock_bytes),
    )
}

pub(crate) fn get_config(config_filepath: &PathBuf) -> (ProjectConfig, Digest) {
    let config_bytes = get_file_bytes(config_filepath);

    (
        match serde_yaml::from_slice::<ProjectConfig>(&config_bytes) {
            Ok(_config) => _config,
            Err(e) => {
                let error_msg = match e.location() {
                    Some(l) => format!(
                        "Malformed config file '{}', line {}, column {}:\n\t{}",
                        config_filepath.display(),
                        l.line(),
                        l.column(),
                        e.to_string(),
                    ),
                    None => format!(
                        "Malformed config file '{}':\n\t{}",
                        config_filepath.display(),
                        e.to_string(),
                    ),
                };

                eprintln!("{}", error_msg);
                exit(exitcode::DATAERR)
            }
        },
        digest(&SHA256, &config_bytes),
    )
}

pub(crate) fn save_config(config_filepath: &PathBuf, config: &ProjectConfig) {
    match serde_yaml::to_vec(config) {
        Ok(serialized_config) => {
            if let Err(e) = write(config_filepath, &serialized_config) {
                eprintln!(
                    "Unknown error while persisting project config:\n\n{}\n",
                    e.to_string()
                );
                exit(exitcode::IOERR)
            }
        }
        Err(e) => {
            eprintln!(
                "Unknown error while serializing project config:\n\n{}\n",
                e.to_string()
            );
            exit(exitcode::SOFTWARE)
        }
    }
}

pub(crate) fn save_config_lock(
    config_lock_filepath: &PathBuf,
    config_lock: &ProjectConfigLock,
) -> Vec<u8> {
    match serde_yaml::to_vec(config_lock) {
        Ok(serialized_config_lock) => {
            if let Err(e) = write(config_lock_filepath, &serialized_config_lock) {
                eprintln!(
                    "Unknown error while persisting project state:\n\n{}\n",
                    e.to_string()
                );
            }
            serialized_config_lock
        }
        Err(e) => {
            eprintln!(
                "Unknown error while serializing project state:\n\n{}\n",
                e.to_string()
            );
            exit(exitcode::SOFTWARE)
        }
    }
}

pub(crate) fn merge_run_configs(
    base_config: &Option<OCIContainerRunConfig>,
    new_config: &Option<OCIContainerRunConfig>,
) -> Option<OCIContainerRunConfigLock> {
    match base_config {
        Some(_base_config) => match new_config {
            Some(_new_config) => Some(OCIContainerRunConfigLock {
                bindings: merge_bindings(_base_config.get_bindings(), _new_config.get_bindings()),
                volumes: merge_volumes(_base_config.get_volumes(), _new_config.get_volumes()),
                env: merge_envs(_base_config.get_env(), _new_config.get_env()),
                env_from_host: merge_envs_from_host(
                    _base_config.get_env_from_host(),
                    _new_config.get_env_from_host(),
                ),
            }),
            None => Some(OCIContainerRunConfigLock {
                bindings: _base_config.bindings.clone(),
                volumes: generate_volume_config_lock(&_base_config.volumes),
                env: _base_config.env.clone(),
                env_from_host: _base_config.env_from_host.clone(),
            }),
        },
        None => match new_config {
            Some(_new_config) => Some(OCIContainerRunConfigLock {
                bindings: _new_config.bindings.clone(),
                volumes: generate_volume_config_lock(&_new_config.volumes),
                env: _new_config.env.clone(),
                env_from_host: _new_config.env_from_host.clone(),
            }),
            None => Option::<OCIContainerRunConfigLock>::None,
        },
    }
}

fn merge_bindings(
    base_bindings: &Option<Vec<BindingConfig>>,
    new_bindings: &Option<Vec<BindingConfig>>,
) -> Option<Vec<BindingConfig>> {
    // TODO: Improve merge strategy
    match new_bindings {
        Some(_) => new_bindings.clone(),
        None => base_bindings.clone(),
    }
}

fn merge_volumes(
    base_volumes: &Option<Vec<VolumeConfig>>,
    new_volumes: &Option<Vec<VolumeConfig>>,
) -> Option<Vec<VolumeConfigLock>> {
    // TODO: Improve merge strategy
    match new_volumes {
        Some(_) => generate_volume_config_lock(new_volumes),
        None => generate_volume_config_lock(base_volumes),
    }
}

fn generate_volume_config_lock(image_volume_configs: &Option<Vec<VolumeConfig>>) -> Option<Vec<VolumeConfigLock>> {
    match image_volume_configs {
        Some(_src_volume_config) => Some(
            _src_volume_config
                .iter()
                .map(|volume_config| VolumeConfigLock {
                    container_path: volume_config.container_path.clone(),
                })
                .collect(),
        ),
        None => Option::<Vec<VolumeConfigLock>>::None,
    }
}

fn merge_envs(
    base_env: &Option<HashMap<String, String>>,
    new_env: &Option<HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    match base_env {
        Some(_base_env) => match new_env {
            Some(_new_env) => {
                let mut merged_env = _base_env.clone();
                for (var_name, var_value) in _new_env {
                    merged_env.insert(var_name.clone(), var_value.clone());
                }
                Some(merged_env)
            }
            None => base_env.clone(),
        },
        None => new_env.clone(),
    }
}

fn merge_envs_from_host(
    base_env: &Option<HashSet<String>>,
    new_env: &Option<HashSet<String>>,
) -> Option<HashSet<String>> {
    match base_env {
        Some(_base_env) => match new_env {
            Some(_new_env) => Some(_base_env.union(_new_env).cloned().collect()),
            None => base_env.clone(),
        },
        None => new_env.clone(),
    }
}
