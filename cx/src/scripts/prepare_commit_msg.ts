#!/usr/bin/env ts-node-script

import { execSync } from 'child_process'
import { getPlumberEnvVars } from '../lib/plumberEnv'

async function run(): Promise<void> {
  if (process.env.SKIP_PREPARE_COMMIT_MSG === '1') {
    return
  }

  const env = await getPlumberEnvVars()
  execSync(`ln -fs "${env.CI_PROJECT_DIR}/.git" "${env.CI_PROJECT_DIR}/cx/.git"`)
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
