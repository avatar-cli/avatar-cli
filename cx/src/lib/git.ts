/*
 *  Copyright (C) 2019-2020  Andres Correa Casablanca
 *  License: GPL 3.0 (See the LICENSE file in the repository root directory)
 */

import { cxExec, trimmedCxExec } from './exec'

export type CommitMessage = {
  title: string
  body: string
}

export async function checkIfSigned(ref: string): Promise<boolean> {
  const gitShowResult = await cxExec(`git show -s --show-signature --format="" ${ref}`)
  return !!gitShowResult.match(/^gpg: Signature made/)
}

export async function fetch(depth = 150): Promise<void> {
  await cxExec(`git fetch --all --depth=${depth}`)
}

export async function getCommonAncestor(ref1: string, ref2: string): Promise<string> {
  return await trimmedCxExec(`git merge-base "${ref1}" "${ref2}"`)
}

export async function getCommitHashesList(fromRef: string, toRef: string): Promise<string[]> {
  return (await trimmedCxExec(`git rev-list "${fromRef}..${toRef}"`)).split(/\s+/).filter(hash => hash.length > 0)
}

export async function getCommitMessageTitle(ref: string): Promise<string> {
  return await trimmedCxExec(`git show -s --format="%s" ${ref}`, { GIT_PAGER: '' })
}

export async function getCommitMessageBody(ref: string): Promise<string> {
  return await trimmedCxExec(`git show -s --format="%b" ${ref}`, { GIT_PAGER: '' })
}

export async function getCommitMessages(commitHashes: string[]): Promise<CommitMessage[]> {
  return Promise.all(
    commitHashes.map(async hash => {
      return {
        title: await getCommitMessageTitle(hash),
        body: await getCommitMessageBody(hash),
      }
    })
  )
}
