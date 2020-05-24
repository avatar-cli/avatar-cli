import gitP, { SimpleGit } from 'simple-git/promise'

async function F() {
  const git: SimpleGit = gitP()

  const show = await git.show(['-s', '--format="%s"', 'abb97d93bd099642f629639aae7afaafc65461e3'])
  console.log(show)
}

F().catch(e => {
  console.log(e)
  process.exit(1)
})
