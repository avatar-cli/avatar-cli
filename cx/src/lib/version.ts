import { parse as tomlParse, JsonMap } from '@iarna/toml'
import { cxExec } from './exec'
import { CommitMessage } from './git'

export async function getPackageJsonVersionFromCommit(ref: string): Promise<[number, number, number]> {
  const packageJson = JSON.parse(await cxExec(`git show ${ref}:cx/package.json`, { GIT_PAGER: '' }))
  const strVersion = (packageJson?.version || '0.0.0') as string
  const strVersionParts = strVersion.split('.')

  if (strVersionParts.length !== 3) {
    throw new Error(`package.json has an invalid version (${strVersion})`)
  }

  return strVersionParts.map(n => Number(n)) as [number, number, number]
}

export async function getCargoTomlVersionFromCommit(ref: string): Promise<[number, number, number]> {
  const cargoToml = tomlParse(await cxExec(`git show ${ref}:Cargo.toml`, { GIT_PAGER: '' }))
  const strVersion = ((cargoToml.package as JsonMap).version || '0.0.0') as string
  const strVersionParts = strVersion.split('.')

  if (strVersionParts.length !== 3) {
    throw new Error(`Cargo.toml has an invalid version (${strVersion})`)
  }

  return strVersionParts.map(n => Number(n)) as [number, number, number]
}

export async function computeVersion(
  baseVersion: [number, number, number],
  commitMessages: CommitMessage[]
): Promise<[number, number, number]> {
  let majorChange = false
  let minorChange = false
  let patchChange = false
  let belowOne = baseVersion[0] === 0

  for (const commitMessage of commitMessages) {
    if (commitMessage.title.match(/!:\s/) || commitMessage.body.match(/BREAKING CHANGE/)) {
      majorChange = true
      if (belowOne && commitMessage.title.match(/^feat!: release 1.0$/)) {
        belowOne = false
      }
    } else if (commitMessage.title.match(/^(feat)(\([^\)]+\))?:/)) {
      minorChange = true
    } else if (commitMessage.title.match(/^(fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^\)]+\))?:/)) {
      patchChange = true
    }
  }

  if (majorChange) {
    return belowOne ? [baseVersion[0], baseVersion[1] + 1, 0] : [baseVersion[0] + 1, 0, 0]
  } else if (minorChange) {
    return [baseVersion[0], baseVersion[1] + 1, 0]
  } else if (patchChange) {
    return [baseVersion[0], baseVersion[1], baseVersion[2] + 1]
  }
  return baseVersion
}
