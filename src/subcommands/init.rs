/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::fs::{create_dir, remove_dir_all, read, write};
use std::{path::PathBuf, process::exit};

extern crate exitcode;

use crate::{
    directories::get_project_path,
    project_config::{save_config, ProjectConfig},
};

pub(crate) fn init_subcommand(project_path: &PathBuf) -> () {
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

    patch_gitignore(project_path);
}

fn patch_gitignore(project_path: &PathBuf) -> () {
  let gitignore_path = project_path.join(".gitignore");

  if gitignore_path.exists() {
    if !gitignore_path.is_file() {
      eprintln!("The file .gitignore must be a file, but found something else.");
      exit(exitcode::USAGE)
    }

    let mut gitignore_bytes = match read(&gitignore_path) {
      Ok(t) => t,
      Err(e) => {
        eprintln!("Unable to read .gitignore file due to unknwon reasons.\n\n{}\n", e.to_string());
        exit(exitcode::IOERR)
      }
    };

    if String::from_utf8_lossy(&gitignore_bytes).contains(".avatar-cli/volatile") == false {
      // TODO: Optimize this, just append, instead of rewriting the entire file
      gitignore_bytes.extend("\n# Avatar-CLI\n.avatar-cli/volatile/\n".as_bytes());
      if let Err(e) = write(&gitignore_path, gitignore_bytes) {
        eprintln!("Unable to modify .gitignore file due to unknown reasons.\n\n{}\n", e.to_string());
        exit(exitcode::IOERR);
      }
    }
  } else {
    if !project_path.join(".git").exists() {
      return
    }

    if let Err(e) = write(&gitignore_path, "# Avatar-CLI\n.avatar-cli/volatile/\n".as_bytes()) {
      eprintln!("Unable to create .gitignore file due to unknown reasons.\n\n{}\n", e.to_string());
      exit(exitcode::CANTCREAT);
    }
  }
}
