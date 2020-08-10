#!/usr/bin/env ts-node-script

import fetch from 'node-fetch'

import { getCxEnvVars } from '../lib/cxEnv'
import { getPackageJsonVersionFromCommit } from '../lib/version'

async function run(): Promise<void> {
  const env = await getCxEnvVars()

  const gitRef = env.CX_GIT_COMMIT_HASH
  const versionComponents = await getPackageJsonVersionFromCommit(gitRef)
  const newTag = `v${versionComponents.join('.')}`

  const ciProjectId = env.CI_PROJECT_ID ?? ''
  if (ciProjectId === '') {
    throw new Error('Project ID not defined')
  }
  const releaseToken = env.GITLAB_RELEASE_TOKEN ?? ''
  if (releaseToken === '') {
    throw new Error('CI token not defined')
  }

  const tagCreationResponse = await fetch(
    `https://gitlab.com/api/v4/projects/${ciProjectId}/repository/tags?tag_name=${newTag}&ref=${gitRef}`,
    {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${releaseToken}`,
      },
    }
  )

  if (!tagCreationResponse.ok || tagCreationResponse.status >= 300) {
    console.error(`Tag Creation Response's HTTP Status:\n\t${tagCreationResponse.status}\n`)
    console.error(`Tag Creation Response's Body:\n${await tagCreationResponse.text()}\n`)
    throw new Error('Error while creating new tag')
  }

  const releaseCreationResponse = await fetch(`https://gitlab.com/api/v4/projects/${ciProjectId}/releases`, {
    method: 'POST',
    body: JSON.stringify({
      tag_name: newTag,
    }),
    headers: {
      Authorization: `Bearer ${releaseToken}`,
      'Content-Type': 'application/json',
    },
  })

  if (!releaseCreationResponse.ok || releaseCreationResponse.status >= 300) {
    console.error(`Release Creation Response's HTTP Status:\n\t${releaseCreationResponse.status}\n`)
    console.error(`Release Creation Response's Body:\n${await releaseCreationResponse.text()}\n`)
    throw new Error('Error while creating new release')
  }
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in gitlab_publish.ts')
    process.exit(1)
  })
