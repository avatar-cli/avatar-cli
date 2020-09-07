/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::env;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::process::exit;

pub(crate) const CONFIG_LOCK_PATH: &str = "AVATAR_CLI_CONFIG_LOCK_PATH";
pub(crate) const CONFIG_PATH: &str = "AVATAR_CLI_CONFIG_PATH";
pub(crate) const FORCE_PROJECT_PATH: &str = "AVATAR_CLI_FORCE_PROJECT_PATH";
pub(crate) const MOUNT_TMP_PATHS: &str = "AVATAR_CLI_MOUNT_TMP_PATHS";
pub(crate) const PROCESS_ID: &str = "AVATAR_CLI_PROCESS_ID";
pub(crate) const PROJECT_PATH: &str = "AVATAR_CLI_PROJECT_PATH";
pub(crate) const PROJECT_INTERNAL_ID: &str = "AVATAR_CLI_PROJECT_INTERNAL_ID";
pub(crate) const SESSION_TOKEN: &str = "AVATAR_CLI_SESSION_TOKEN";
pub(crate) const STATE_PATH: &str = "AVATAR_CLI_STATE_PATH";

pub(crate) struct AvatarEnv {
    project_path: PathBuf,
    session_token: String,
}

impl AvatarEnv {
    pub fn read() -> Self {
        Self {
            project_path: PathBuf::from(Self::get_var(PROJECT_PATH)),
            session_token: Self::get_var(SESSION_TOKEN),
        }
    }

    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_path
    }

    pub fn get_session_token(&self) -> &String {
        &self.session_token
    }

    fn get_var(var_name: &str) -> String {
        match env::var(var_name) {
            Ok(v) => v,
            Err(_) => {
                eprintln!("The '{}' environment variable is not defined", var_name);
                exit(exitcode::CONFIG)
            }
        }
    }
}

pub(crate) fn get_used_program_name() -> String {
    let first_arg = match env::args().next() {
        Some(a) => a,
        None => {
            eprintln!("Due to an unknown reason, it was impossible to retrieve the command arguments list");
            exit(exitcode::OSERR);
        }
    };
    match first_arg.split(MAIN_SEPARATOR).last() {
      Some(pname) => pname,
      None => {
          eprintln!("Due to an unknown reason, an empty first command argument was passed to this process");
          exit(exitcode::OSERR)
      }
  }.to_string()
}
