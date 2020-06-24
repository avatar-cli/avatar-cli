#!/usr/bin/env ts-node-script

import { getCxEnvVars } from '../lib/cxEnv'
import { fetch as gitFetch, getCommonAncestor, getCommitHashesList, checkIfSigned, getCommitMessages } from '../lib/git'
import { cxExec } from '../lib/exec'
import { getPackageJsonVersionFromCommit, getCargoTomlVersionFromCommit, computeVersion } from '../lib/version'

async function checkCommitMessages(commonAncestor: string): Promise<void> {
  console.log('Validating commit messages')
  await cxExec(`npm run commitlint -- --from ${commonAncestor}`)
}

async function checkCommitSignatures(commitsList: string[]): Promise<void> {
  console.log('Checking that commits are signed (but not verifying signatures)')
  for (const commitHash of commitsList) {
    if (!(await checkIfSigned(commitHash))) {
      console.error(`ERROR: Commit ${commitHash} is not signed`)
      process.exit(1)
    }
  }
}

async function checkVersions(ancestorGitRef: string, currentGitRef: string, commitHashes: string[]): Promise<void> {
  const packageJsonVersion = await getPackageJsonVersionFromCommit(currentGitRef)
  const cargoTomlVersion = await getCargoTomlVersionFromCommit(currentGitRef)
  const strPackageJsonVersion = packageJsonVersion.join('.')
  const strCargoTomlVersion = cargoTomlVersion.join('.')

  if (strPackageJsonVersion !== strCargoTomlVersion) {
    console.error(
      `ERROR: package.json version (${strPackageJsonVersion}) and Cargo.tom version (${strCargoTomlVersion}) are different`
    )
    process.exit(1)
  }

  const oldVersion = await getPackageJsonVersionFromCommit(ancestorGitRef)
  const commitMessages = await getCommitMessages(commitHashes)

  const newVersion = computeVersion(oldVersion, commitMessages)
  const strNewVersion = (await newVersion).join('.')

  if (strNewVersion !== strPackageJsonVersion) {
    console.error(`ERROR: Current version should be ${strNewVersion}, but is ${strPackageJsonVersion}`)
    process.exit(1)
  }
}

async function run(): Promise<void> {
  const env = await getCxEnvVars()

  // We do this so we can "compare" branches
  await gitFetch()

  const commonAncestor = await getCommonAncestor(env.CX_GIT_MASTER_REF, env.CX_GIT_COMMIT_HASH)
  const commitHashes = await getCommitHashesList(commonAncestor, env.CX_GIT_COMMIT_HASH)

  await checkCommitMessages(commonAncestor)
  await checkCommitSignatures(commitHashes)
  await checkVersions(commonAncestor, env.CX_GIT_COMMIT_HASH, commitHashes)
}

run()
  .then(() => {
    console.log('Finished git checks')
  })
  .catch(reason => {
    console.log(reason)
    process.exit(1)
  })
