#!/usr/bin/env ts-node-script

import { getCxEnvVars } from '../lib/cxEnv'
import { getPackageJsonVersionFromCommit } from '../lib/version'

async function run(): Promise<void> {
  if (process.argv.length != 3) {
    throw new Error('pkg_version_info admits exactly 1 argument, no more, no less')
  }

  const env = await getCxEnvVars()

  const gitRef = env.CX_GIT_COMMIT_HASH
  const versionComponents = await getPackageJsonVersionFromCommit(gitRef)

  const versionPart = process.argv[2]

  if (versionPart === 'major') {
    console.log(versionComponents[0])
  } else if (versionPart === 'minor') {
    console.log(versionComponents[1])
  } else if (versionPart === 'patch') {
    console.log(versionComponents[2])
  } else {
    throw new Error('only "major", "minor" and "patch" are accepted as arguments')
  }
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in pkg_version_info.ts')
    process.exit(1)
  })
