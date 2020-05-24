import { cleanExecWithStringReturn, execWithStringReturn } from './exec'

export async function checkIfSigned(ref: string): Promise<boolean> {
  const gitShowResult = await execWithStringReturn(`git show -s --show-signature --format="" ${ref}`)
  return !!gitShowResult.match(/^gpg: Signature made/)
}

export async function fetch(depth = 150): Promise<void> {
  await execWithStringReturn(`git fetch --all --depth=${depth}`)
}

export async function getCommonAncestor(ref1: string, ref2: string): Promise<string> {
  return await cleanExecWithStringReturn(`git merge-base "${ref1}" "${ref2}"`)
}

export async function getCommitHashesList(fromRef: string, toRef: string): Promise<string[]> {
  return (await cleanExecWithStringReturn(`git rev-list "${fromRef}..${toRef}"`))
    .split(/\s+/)
    .filter(hash => hash.length > 0)
}

export async function getCommitMessageTitle(ref: string): Promise<string> {
  return await cleanExecWithStringReturn(`git show -s --format="%s" ${ref}`)
}

export async function getCommitMessageBody(ref: string): Promise<string> {
  return await cleanExecWithStringReturn(`git show -s --format="%b" ${ref}`)
}
