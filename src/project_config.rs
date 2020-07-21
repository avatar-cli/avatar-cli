/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::fs::{read, write};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::exit;
use std::vec::Vec;

use serde::{Deserialize, Serialize};

extern crate ring;
use ring::digest::{digest, Digest, SHA256};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct VolumeConfig {
    containerPath: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BindingConfig {
    hostPath: PathBuf,
    containerPath: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIContainerRunConfig {
    volumes: Option<Vec<VolumeConfig>>,
    bindings: Option<Vec<BindingConfig>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImageBinaryConfig {
    path: PathBuf,
    runConfig: Option<OCIContainerRunConfig>,
}

impl ImageBinaryConfig {
    pub fn getPath(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIImageConfig {
    binaries: Option<HashMap<String, ImageBinaryConfig>>,
}

impl OCIImageConfig {
    pub fn getBinaries(&self) -> &Option<HashMap<String, ImageBinaryConfig>> {
        &self.binaries
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectConfig {
    version: String,
    projectInternalId: String,
    images: Option<HashMap<String, HashMap<String, OCIImageConfig>>>, // image name -> image tag -> oci image config
}

impl ProjectConfig {
    pub fn getProjectInternalId(&self) -> &String {
        &self.projectInternalId
    }

    pub fn getImages(&self) -> &Option<HashMap<String, HashMap<String, OCIImageConfig>>> {
        &self.images
    }
}

// -----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIImageConfigLock {
    name: String,
    hash: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImageBinaryConfigLock {
    ociImageName: String,
    ociImageHash: String,
    path: PathBuf,
    runConfig: Option<OCIContainerRunConfig>,
}

impl ImageBinaryConfigLock {
    pub fn new(ociImageName: String, ociImageHash: String, path: PathBuf) -> ImageBinaryConfigLock {
        ImageBinaryConfigLock {
            ociImageName: ociImageName,
            ociImageHash: ociImageHash,
            path: path,
            runConfig: None, // TODO
        }
    }

    pub fn getOCIImageName(&self) -> &String {
        &self.ociImageName
    }

    pub fn getOCIImageHash(&self) -> &String {
        &self.ociImageHash
    }

    pub fn getPath(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectConfigLock {
    #[serde(with = "hex")]
    projectConfigHash: Vec<u8>,
    projectInternalId: String,
    images: HashMap<String, HashMap<String, String>>, // image_name -> image_tag -> image_hash
    binaries: HashMap<String, ImageBinaryConfigLock>,
}

impl ProjectConfigLock {
    pub fn getProjectConfigHash(&self) -> &Vec<u8> {
        &self.projectConfigHash
    }

    pub fn updateProjectConfigHash(mut self, new_hash: &[u8]) -> ProjectConfigLock {
        self.projectConfigHash = Vec::from(new_hash);
        self
    }

    pub fn getProjectInternalId(&self) -> &String {
        &self.projectInternalId
    }

    pub fn getImages(&self) -> &HashMap<String, HashMap<String, String>> {
        &self.images
    }

    pub fn getBinaryConfiguration(&self, binary_name: &str) -> Option<&ImageBinaryConfigLock> {
        self.binaries.get(binary_name)
    }

    pub fn new(
        projectConfigHash: Vec<u8>,
        projectInternalId: String,
        images: HashMap<String, HashMap<String, String>>,
        binaries: HashMap<String, ImageBinaryConfigLock>,
    ) -> ProjectConfigLock {
        ProjectConfigLock {
            projectConfigHash: projectConfigHash,
            projectInternalId: projectInternalId,
            images: images,
            binaries: binaries,
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
