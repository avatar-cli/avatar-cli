import { promisify } from 'util'
import { exec as cbExec } from 'child_process'

const exec = promisify(cbExec)

async function execWithStringReturn(
  command: string,
  env?: NodeJS.ProcessEnv | null,
  mergeEnvs = true
): Promise<string> {
  const commandEnv = mergeEnvs ? { ...process.env, ...(env ?? {}) } : env ?? process.env
  try {
    const { stdout } = await exec(command, { env: commandEnv })
    return stdout
  } catch (reason) {
    if (reason?.stdout) {
      console.log(reason.stdout.toString())
    }
    if (reason?.stderr) {
      console.error(reason.stderr.toString())
    }
    throw reason
  }
}

async function cleanExecWithStringReturn(
  command: string,
  env?: NodeJS.ProcessEnv | null,
  mergeEnvs = true
): Promise<string> {
  return (await execWithStringReturn(command, env, mergeEnvs)).trim()
}

export { execWithStringReturn, cleanExecWithStringReturn }
