#!/usr/bin/env ts-node-script

import fetch from 'node-fetch'

import { getCxEnvVars } from '../lib/cxEnv'
import { getPackageJsonVersionFromCommit } from '../lib/version'

async function createNewTag(ciProjectId: string, newTag: string, gitRef: string, releaseToken: string) {
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
}

async function createNewRelease(ciProjectId: string, newTag: string, releaseToken: string) {
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

async function createCratesIoLink(
  ciProjectId: string,
  newTag: string,
  newVersion: string,
  releaseToken: string
): Promise<void> {
  const cratesIoLinkResponse = await fetch(
    `https://gitlab.com/api/v4/projects/${ciProjectId}/releases/${newTag}/assets/links`,
    {
      method: 'POST',
      body: JSON.stringify({
        link_type: 'package',
        name: 'Crates.io "Binary" Package',
        url: `https://crates.io/crates/avatar-cli/${newVersion}`,
      }),
      headers: {
        Authorization: `Bearer ${releaseToken}`,
        'Content-Type': 'application/json',
      },
    }
  )

  if (!cratesIoLinkResponse.ok || cratesIoLinkResponse.status >= 300) {
    console.error(`Crates.io Link Creation Response's HTTP Status:\n\r${cratesIoLinkResponse.status}\n`)
    console.error(`Crates.io Link Creation Response's Body:\n${await cratesIoLinkResponse.text()}\n`)
    throw new Error('Error while creating new Crates.io link')
  }
}

async function run(): Promise<void> {
  const env = await getCxEnvVars()

  const gitRef = env.CX_GIT_COMMIT_HASH
  const versionComponents = await getPackageJsonVersionFromCommit(gitRef)
  const newVersion = versionComponents.join('.')
  const newTag = `v${newVersion}`

  const ciProjectId = env.CI_PROJECT_ID ?? ''
  if (ciProjectId === '') {
    throw new Error('Project ID not defined')
  }
  const releaseToken = env.GITLAB_RELEASE_TOKEN ?? ''
  if (releaseToken === '') {
    throw new Error('CI token not defined')
  }

  await createNewTag(ciProjectId, newTag, gitRef, releaseToken)
  await createNewRelease(ciProjectId, newTag, releaseToken)
  await createCratesIoLink(ciProjectId, newTag, newVersion, releaseToken)
}

run()
  .then(() => {
    //
  })
  .catch(reason => {
    console.error(reason?.toString() ?? 'Unknown Error in gitlab_publish.ts')
    process.exit(1)
  })
