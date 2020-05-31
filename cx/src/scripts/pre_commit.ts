#!/usr/bin/env ts-node-script

import { readFile, writeFile } from 'fs/promises'
import { join as pathJoin } from 'path'

import { parse as tomlParse, stringify as tomlStringify, JsonMap } from '@iarna/toml'

import { fetch as gitFetch, getCommonAncestor, getCommitMessages, getCommitHashesList, CommitMessage } from '../lib/git'
import { getPlumberEnvVars, PlumberEnv } from '../lib/plumberEnv'
import { getPackageJsonVersionFromCommit, computeVersion } from '../lib/version'
import { execWithStringReturn } from '../lib/exec'

async function getNextCommitMessage(): Promise<CommitMessage | null> {
  const nextCommitMessageFilePath = pathJoin(__dirname, '..', '..', '..', '.git', 'COMMIT_EDITMSG')
  const nextCommitMessageContent = await readFile(nextCommitMessageFilePath, { encoding: 'utf8' })
  const nextCommitMessageLines = nextCommitMessageContent.split(/\r?\n/).filter(l => !l.match(/^\s*$/))

  if (nextCommitMessageLines.length === 0) {
    return null
  }

  const nextCommitMessageTitle = nextCommitMessageLines.shift() as string
  let nextCommitMessageBody = ''
  for (const line of nextCommitMessageLines) {
    if (line.match(/^#/)) {
      break
    }
    nextCommitMessageBody += `${line}\n`
  }

  return { title: nextCommitMessageTitle, body: nextCommitMessageBody }
}

async function computeAvatarVersion(env: PlumberEnv, hookCommitMsg = false): Promise<string> {
  // We do this so we can "compare" branches
  await gitFetch()

  const ancestorGitRef = await getCommonAncestor(env.PLUMBER_GIT_MASTER_REF, env.CI_COMMIT_SHA)

  const commitHashes = await getCommitHashesList(ancestorGitRef, env.CI_COMMIT_SHA)
  const commitMessages = await getCommitMessages(commitHashes)

  if (hookCommitMsg) {
    const nextCommitMessage = await getNextCommitMessage()
    if (nextCommitMessage !== null) {
      commitMessages.push(nextCommitMessage)
    }
  }

  const oldVersion = await getPackageJsonVersionFromCommit(ancestorGitRef)
  const newVersion = await computeVersion(oldVersion, commitMessages)

  return newVersion.join('.')
}

async function updatePackageJson(newVersion: string): Promise<boolean> {
  const filePath = pathJoin(__dirname, '..', '..', 'package.json')
  const packageJson: { [key: string]: any } = JSON.parse(await readFile(filePath, { encoding: 'utf8' }))

  if (packageJson?.version === newVersion) {
    return false
  }

  packageJson.version = newVersion
  await writeFile(filePath, JSON.stringify(packageJson, null, 2))
  await execWithStringReturn(`git add ${filePath}`)

  console.log('pre-commit hook: Updated package.json version')
  return true
}

async function updateCargoToml(newVersion: string): Promise<boolean> {
  const filePath = pathJoin(__dirname, '..', '..', '..', 'Cargo.toml')
  const cargoToml = tomlParse(await readFile(filePath, { encoding: 'utf8' }))

  if ((cargoToml.package as JsonMap).version === newVersion) {
    return false
  }

  ;(cargoToml.package as JsonMap).version = newVersion
  await writeFile(filePath, tomlStringify(cargoToml))
  await execWithStringReturn(`git add ${filePath}`)

  console.log('pre-commit hook: Updated Cargo.toml package version')
  return true
}

async function run(): Promise<void> {
  const hookCommitMsg = process.argv.includes('--hook-commit-msg')
  const env = await getPlumberEnvVars()
  const newVersion = await computeAvatarVersion(env, hookCommitMsg)

  const updatedVersion = (await updatePackageJson(newVersion)) || (await updateCargoToml(newVersion))
  if (hookCommitMsg && updatedVersion) {
    throw new Error("Package version was updated in the commit-msg hook, git commit can't continue")
  }
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.log(reason)
    process.exit(1)
  })
