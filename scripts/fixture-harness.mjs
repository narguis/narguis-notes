import { spawnSync } from "node:child_process"
import { homedir } from "node:os"
import { join } from "node:path"

const fixtureNames = {
  ipc: "malformed-inputs",
  "storage-failure": "delete-rollback",
}

const [fixture, ...rawArgumentsAfterFixture] = process.argv.slice(2)
const argumentsAfterFixture =
  rawArgumentsAfterFixture[0] === "--"
    ? rawArgumentsAfterFixture.slice(1)
    : rawArgumentsAfterFixture
const expectedCase = fixtureNames[fixture]

if (expectedCase === undefined) {
  console.error(`Unknown fixture: ${fixture ?? "<missing>"}`)
  process.exit(2)
}

const expectedArguments = ["--case", expectedCase]
const suppliedArguments =
  argumentsAfterFixture.length === 0 ? expectedArguments : argumentsAfterFixture

if (
  suppliedArguments.length !== expectedArguments.length ||
  suppliedArguments.some((argument, index) => argument !== expectedArguments[index])
) {
  console.error(`Expected arguments: ${expectedArguments.join(" ")}`)
  process.exit(2)
}

if (fixture === "ipc" || fixture === "storage-failure") {
  const cargoArguments =
    fixture === "ipc"
      ? ["test", "--manifest-path", "src-tauri/Cargo.toml", "--test", "ipc"]
      : [
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "--test",
          "task_template_copy",
          "delete_failure_keeps_template_and_inserted_line_unchanged",
          "--",
          "--exact",
          "--test-threads=1",
        ]
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
}

console.log(`FIXTURE_OK ${fixture} case=${expectedCase}`)
