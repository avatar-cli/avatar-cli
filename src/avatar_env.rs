use std::env;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::process::exit;

pub(crate) struct AvatarEnv {
    session_token: String,
    project_path: PathBuf,
    config_path: PathBuf,
    config_lock_path: PathBuf,
}

impl AvatarEnv {
    pub fn read() -> Self {
        Self {
            session_token: Self::get_var("AVATAR_CLI_SESSION_TOKEN"),
            project_path: PathBuf::from(Self::get_var("AVATAR_CLI_PROJECT_PATH")),
            config_path: PathBuf::from(Self::get_var("AVATAR_CLI_CONFIG_PATH")),
            config_lock_path: PathBuf::from(Self::get_var("AVATAR_CLI_CONFIG_LOCK_PATH")),
        }
    }

    pub fn get_session_token(&self) -> &String {
        &self.session_token
    }

    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_path
    }

    pub fn get_config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn get_config_lock_path(&self) -> &PathBuf {
        &self.config_lock_path
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
    let first_arg = match env::args().nth(0) {
        Some(a) => a,
        None => {
            eprintln!(
              "Due to an unknown reason, it was impossible to retrieve the command arguments list"
          );
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
