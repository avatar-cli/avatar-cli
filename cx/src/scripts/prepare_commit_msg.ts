#!/usr/bin/env ts-node-script

import { execSync } from 'child_process'
import { getCxEnvVars } from '../lib/cxEnv'

async function run(): Promise<void> {
  if (process.env.SKIP_PREPARE_COMMIT_MSG === '1') {
    return
  }

  const env = await getCxEnvVars()
  execSync(`ln -fs "${env.CX_PROJECT_DIR}/.git" "${env.CX_PROJECT_DIR}/cx/.git"`)
  try {
    execSync('exec < /dev/tty && npm run git-cz -- --hook', { stdio: ['inherit', 'inherit', 'inherit'] })
  } catch {
    // Do nothing if it fails
  }
  execSync('unlink ./.git')
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in prepare_commit_msg.ts')
    process.exit(1)
  })
