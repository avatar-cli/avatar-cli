/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct VolumeConfig {
    containerPath: PathBuf,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BindingConfig {
    hostPath: PathBuf,
    containerPath: PathBuf,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIContainerRunConfig {
    volumes: Option<Vec<VolumeConfig>>,
    bindings: Option<Vec<BindingConfig>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImageBinaryConfig {
    name: Option<String>, // If not set, it will be inferred from path
    path: PathBuf,
    runConfig: Option<OCIContainerRunConfig>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIImageConfig {
    name: String, // fully qualified name, including tag
    binaries: Option<Vec<ImageBinaryConfig>>,
    runConfig: Option<OCIContainerRunConfig>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectConfig {
    version: String,
    images: Option<Vec<OCIImageConfig>>,
}

// -----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OCIImageConfigLock {
    name: String,
    hash: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImageBinaryConfigLock {
    ociImageName: String,
    ociImageHash: String,
    path: PathBuf,
    runConfig: Option<OCIContainerRunConfig>,
}

impl ImageBinaryConfigLock {
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectConfigLock {
    projectConfigHash: String,
    images: Vec<OCIImageConfigLock>,
    binaries: HashMap<String, ImageBinaryConfigLock>,
}

impl ProjectConfigLock {
    pub fn getBinaryConfiguration(&self, binary_name: &str) -> Option<&ImageBinaryConfigLock> {
        self.binaries.get(binary_name)
    }
}