/*
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

import { promisify } from 'util'
import { exec as _exec } from 'child_process'

const exec = promisify(_exec)

export async function cxExec(command: string, env?: NodeJS.ProcessEnv): Promise<string> {
  const _env = { ...process.env, ...(env ?? {}) }
  const { stdout } = await exec(command, { env: _env })
  return stdout
}

export async function trimmedCxExec(command: string, env?: NodeJS.ProcessEnv): Promise<string> {
  return (await cxExec(command, env)).trim()
}
