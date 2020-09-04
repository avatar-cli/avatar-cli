/*
 *  Avatar CLI: Magic wrapper to run containerized CLI tools
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

use std::{
    process::{exit, Command},
    str::from_utf8,
};

pub(crate) const ERROR_MSG_DOCKER_INSPECT_OUTPUT: &str =
    "The command `docker inspect` returned an unexpected output";

pub(crate) fn get_path_env_var_from_oci_image(image_fqn: &str) -> Option<String> {
    if let Ok(output) = Command::new("docker")
        .args(&[
            "inspect",
            "--format={{range .ContainerConfig.Env}}{{println .}}{{end}}",
            &image_fqn,
        ])
        .output()
    {
        if !output.status.success() {
            eprintln!("docker inspect call failed to return image env vars");
            exit(exitcode::SOFTWARE)
        }

        if let Ok(stdout) = from_utf8(&output.stdout) {
            for var_def in stdout.trim().split('\n') {
                let mut var_def_parts = var_def.splitn(2, '=');
                let var_name = var_def_parts.next().unwrap_or_else(|| {
                    eprintln!("{}", ERROR_MSG_DOCKER_INSPECT_OUTPUT);
                    exit(exitcode::PROTOCOL)
                });
                if var_name != "PATH" {
                    continue;
                }
                if let Some(image_path) = var_def_parts.next() {
                    return Some(image_path.to_string());
                }
            }

            return None;
        } else {
            eprintln!("{}", ERROR_MSG_DOCKER_INSPECT_OUTPUT);
            exit(exitcode::PROTOCOL)
        }
    }

    eprintln!("unable to call docker inspect command");
    exit(exitcode::OSERR)
}
