#!/bin/sh

# Avatar CLI: Magic wrapper to run containerized CLI tools
# Copyright (C) 2019-2020  Andres Correa Casablanca
# License: GPL 3.0 (See the LICENSE file in the repository root directory)

# WARNING:
# Don't add this file to your $PATH environment variable, use `avatar shell`,
# `avatar export-env` or `avatar run` when possible. This wrapper exists only
# for very rigid environments where is almost impossible to customize anything,
# and it has an important performance impact.

if command -v avatar > /dev/null; then
  # shellcheck disable=SC2155
  AVATAR_CLI_BIN="$(command -v avatar)";
else
  AVATAR_CLI_BIN="/avatar/bin/path";
fi

export AVATAR_CLI_MOUNT_TMP_PATHS="true";
export AVATAR_CLI_FORCE_PROJECT_PATH="/avatar/prj/path";

exec "${AVATAR_CLI_BIN}" run "${0##*/}" -- "$@";
