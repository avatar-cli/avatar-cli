#!/usr/bin/env ts-node-script

import { existsSync } from 'fs'
import { symlink, unlink } from 'fs/promises'
import { execSync } from 'child_process'
import { getCxEnvVars } from '../lib/cxEnv'

async function run(): Promise<void> {
  if (process.env.SKIP_PREPARE_COMMIT_MSG === '1') {
    return
  }

  const env = await getCxEnvVars()

  if (existsSync(`${env.CX_PROJECT_DIR}/.git/rebase-merge`) || existsSync(`${env.CX_PROJECT_DIR}/.git/rebase-apply`)) {
    // We skip the Commitizen wizard if we are in the middle of a rebase
    return
  }

  const fakeGitDir = `${env.CX_PROJECT_DIR}/cx/.git`
  await symlink(`${env.CX_PROJECT_DIR}/.git`, fakeGitDir)
  try {
    execSync('exec < /dev/tty && npm run git-cz -- --hook', { stdio: ['inherit', 'inherit', 'inherit'] })
  } catch {
    // Do nothing if it fails
  }

  if (existsSync(fakeGitDir)) {
    await unlink(fakeGitDir)
  }
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in prepare_commit_msg.ts')
    process.exit(1)
  })
