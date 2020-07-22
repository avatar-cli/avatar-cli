/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::fs::{create_dir, remove_dir_all};
use std::{path::PathBuf, process::exit};

extern crate exitcode;

use crate::{
    directories::get_project_path,
    project_config::{save_config, ProjectConfig},
};

pub(crate) fn init_subcommand(project_path: &PathBuf) {
    if let Some(p) = get_project_path() {
        eprintln!(
            "avatar init cannot create a new project over an existing one, in {}",
            p.display()
        );
        exit(exitcode::USAGE)
    }

    let settings_dir = project_path.join(".avatar-cli");
    if settings_dir.exists() {
        if settings_dir.is_file() {
            eprintln!(
                "The path {} must point to a directory, found something else",
                settings_dir.display()
            );
            exit(exitcode::USAGE)
        }

        if let Err(e) = remove_dir_all(&settings_dir) {
            eprintln!(
                "Unable to delete broken settings directory {}\n\n{}\n",
                settings_dir.display(),
                e.to_string()
            );
            exit(exitcode::OSERR)
        }
    }

    if let Err(e) = create_dir(&settings_dir) {
        eprintln!(
            "Unable to create settings directory {}\n\n{}\n",
            settings_dir.display(),
            e.to_string()
        );
        exit(exitcode::CANTCREAT)
    }

    let config = ProjectConfig::new();
    let config_filepath = settings_dir.join("avatar-cli.yml");
    save_config(&config_filepath, &config);
}
