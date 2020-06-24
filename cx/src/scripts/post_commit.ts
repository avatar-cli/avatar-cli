#!/usr/bin/env ts-node-script

import { join as pathJoin } from 'path'
import { cxExec } from '../lib/exec'

async function run(): Promise<void> {
  const preCommitScriptPath = pathJoin(__dirname, 'pre_commit.ts')
  const preCommitOutput = await cxExec(`ts-node ${preCommitScriptPath}`)

  if (!preCommitOutput.match(/Updated/)) {
    return
  }

  const packageJsonPath = pathJoin(__dirname, '..', '..', 'package.json')
  const cargoTomlPath = pathJoin(__dirname, '..', '..', '..', 'Cargo.toml')

  await cxExec(`git commit --no-verify --amend -C HEAD ${packageJsonPath} ${cargoTomlPath}`, {
    SKIP_PREPARE_COMMIT_MSG: '1',
    FROM_POSTCOMMIT_HOOK: '1',
  })
  console.log('Updated previous commit to use the correct package version')
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in post_commit.ts')
    process.exit(1)
  })
