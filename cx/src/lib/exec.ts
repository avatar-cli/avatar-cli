import { promisify } from 'util'
import { exec as cbExec } from 'child_process'

const __exec = promisify(cbExec)

async function execWithStringReturn(
  command: string,
  env?: NodeJS.ProcessEnv | null,
  mergeEnvs = true
): Promise<string> {
  const commandEnv = mergeEnvs ? { ...process.env, ...(env ?? {}) } : env ?? process.env
  try {
    const { stdout } = await __exec(command, { env: commandEnv })
    return stdout
  } catch (reason) {
    if (reason?.stdout) {
      console.log(reason.stdout.toString ? reason.stdout.toString() : `${reason.stdout}`)
    }
    if (reason?.stderr) {
      console.error(reason.stderr.toString ? reason.stderr.toString() : `${reason.stderr}`)
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
