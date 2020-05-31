#!/usr/bin/env node

// eslint-disable-next-line @typescript-eslint/no-var-requires
const fs = require('fs')
// eslint-disable-next-line @typescript-eslint/no-var-requires
const path = require('path')

const tsconfigFilePath = path.join(__dirname, '..', '..', 'tsconfig.json')

// To be able to run Typescript without prior transpilation
require('ts-node').register({ project: tsconfigFilePath })

const commandName = process.argv[2]
const commandPath = path.join(__dirname, `${commandName}.ts`)

if (fs.existsSync(commandPath)) {
  require(commandPath)
} else {
  console.error(`Command ${commandName} is not defined`)
  process.exit(1)
}
