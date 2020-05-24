import { parse as tomlParse } from 'toml'
import { execWithStringReturn } from './exec'

export async function getPackageJsonVersionFromCommit(ref: string): Promise<[number, number, number]> {
  const packageJson = JSON.parse(await execWithStringReturn(`git show ${ref}:cx/package.json`, { GIT_PAGER: '' }))
  const strVersion = (packageJson?.version || '0.0.0') as string
  const strVersionParts = strVersion.split('.')

  if (strVersionParts.length !== 3) {
    throw new Error(`package.json has an invalid version (${strVersion})`)
  }

  return strVersionParts.map(n => Number(n)) as [number, number, number]
}

export async function getCargoTomlVersionFromCommit(ref: string): Promise<[number, number, number]> {
  const cargoToml = tomlParse(await execWithStringReturn(`git show ${ref}:Cargo.toml`, { GIT_PAGER: '' }))
  const strVersion = (cargoToml?.package?.version || '0.0.0') as string
  const strVersionParts = strVersion.split('.')

  if (strVersionParts.length !== 3) {
    throw new Error(`Cargo.toml has an invalid version (${strVersion})`)
  }

  return strVersionParts.map(n => Number(n)) as [number, number, number]
}
