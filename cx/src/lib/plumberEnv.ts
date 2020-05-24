import { randomBytes } from 'crypto'

import { cleanExecWithStringReturn } from './exec'

type GitlabEnv = {
  /** current git branch name */
  CI_COMMIT_REF_NAME: string
  /** current git commit hash */
  CI_COMMIT_SHA: string
  /** unique pipeline identifier */
  CI_CONCURRENT_ID: string
  /** absolute path of the cloned repository during the pipeline execution */
  CI_PROJECT_DIR: string
  /** docker image name associated to the gitlab repository */
  CI_REGISTRY_IMAGE: string
}

type PlumberBaseEnv = {
  /** reference to the git master branch */
  PLUMBER_GIT_MASTER_REF: string
  /** '1' if the process is running in a CI pipeline, undefined otherwise */
  PLUMBER_IN_CI?: string
  /** key used to decrypt local files */
  PLUMBER_ENV_KEY?: string
}

type PlumberEnv = NodeJS.ProcessEnv & GitlabEnv & PlumberBaseEnv

async function getCiRegistryImage(): Promise<string> {
  const gitOriginMatcher = /^.+:(.+)\.git$/
  const gitOrigin = await cleanExecWithStringReturn('git remote get-url origin')
  const match = gitOriginMatcher.exec(gitOrigin)
  if (!match || !match[1]) {
    throw new Error('Unable to infer CI_REGISTRY_IMAGE environment variable')
  }
  const baseName = match[1]
  return `registry.gitlab.com/${baseName}`
}

async function getPlumberEnvVars(): Promise<PlumberEnv> {
  const penv = process.env
  const CI_PROJECT_DIR = penv.CI_PROJECT_DIR ?? (await cleanExecWithStringReturn('git rev-parse --show-toplevel'))
  const CI_ENVIRONMENT_NAME = penv.CI_ENVIRONMENT_NAME ?? 'local'
  const PLUMBER_IN_CI = penv.PLUMBER_IN_CI ?? penv.CI_PROJECT_ID ? '1' : undefined // It must be a string, or undefined

  if (penv.PLUMBER_IN_CI) {
    // It's a warning (and not an error) because we allow it for local experiments
    console.log('WARNING: The loaded environment defines the PLUMBER_IN_CI var, which should be defined at runtime')
  }

  return {
    ...penv,

    // Gitlab variables
    CI_COMMIT_REF_NAME: penv.CI_COMMIT_REF_NAME ?? (await cleanExecWithStringReturn('git rev-parse --abbrev-ref HEAD')),
    CI_COMMIT_SHA: penv.CI_COMMIT_SHA ?? (await cleanExecWithStringReturn('git rev-parse HEAD')),
    CI_CONCURRENT_ID: penv.CI_CONCURRENT_ID ?? randomBytes(8).toString('hex'),
    CI_ENVIRONMENT_NAME: CI_ENVIRONMENT_NAME,
    CI_PROJECT_DIR: CI_PROJECT_DIR,
    CI_REGISTRY_IMAGE: penv.CI_REGISTRY_IMAGE ?? (await getCiRegistryImage()),

    // Plumber variables
    PLUMBER_GIT_MASTER_REF: penv.CI_PROJECT_ID ? 'origin/master' : 'master',
    PLUMBER_IN_CI: PLUMBER_IN_CI,
  }
}

export { getPlumberEnvVars, PlumberEnv }
