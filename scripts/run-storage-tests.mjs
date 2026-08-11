import { spawnSync } from "node:child_process"
import { homedir } from "node:os"
import { join } from "node:path"

const rawArguments = process.argv.slice(2)
const suppliedArguments = rawArguments[0] === "--" ? rawArguments.slice(1) : rawArguments
const expectedArguments = ["--case", "migration-failure"]
const usesFailureFixture = suppliedArguments.length > 0

if (
  usesFailureFixture &&
  (suppliedArguments.length !== expectedArguments.length ||
    suppliedArguments.some((argument, index) => argument !== expectedArguments[index]))
) {
  console.error(`Expected arguments: ${expectedArguments.join(" ")}`)
  process.exit(2)
}

const cargoArguments = ["test", "--manifest-path", "src-tauri/Cargo.toml"]

if (usesFailureFixture) {
  cargoArguments.push(
    "--test",
    "outline_migration",
    "preserves_v2_database_when_v3_migration_is_interrupted",
  )
}

const result = spawnSync(process.env.CARGO ?? "cargo", cargoArguments, { stdio: "inherit" })
const fallbackResult =
  result.error?.code === "ENOENT" && process.env.CARGO === undefined
    ? spawnSync(join(homedir(), ".cargo", "bin", "cargo"), cargoArguments, { stdio: "inherit" })
    : result

if (fallbackResult.error !== undefined) {
  console.error(`Unable to start Cargo: ${fallbackResult.error.message}`)
  process.exit(1)
}

process.exit(fallbackResult.status ?? 1)
