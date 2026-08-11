import { readFileSync } from "node:fs"

const REQUIRED_TODO_FIELDS = [
  "What to do / Must",
  "Parallelization:",
  "References",
  "Acceptance criteria",
  "QA scenarios",
  "Commit:",
]

const STOP_WORDS = new Set([
  "add",
  "all",
  "and",
  "are",
  "as",
  "be",
  "by",
  "do",
  "for",
  "from",
  "in",
  "is",
  "it",
  "local",
  "must",
  "no",
  "not",
  "of",
  "on",
  "or",
  "the",
  "to",
  "use",
  "with",
])

const planPath = process.argv[2]

if (planPath === undefined || process.argv.length !== 3) {
  console.error("Usage: node scripts/verify-plan.mjs <plan-path>")
  process.exit(2)
}

const text = readFileSync(planPath, "utf8")
const errors = []
const mustHave = sectionBullets(text, "### Must have", "### Must NOT have")
const todos = parseTodos(text)
const matrix = parseDependencyMatrix(text)

if (mustHave.length === 0) {
  errors.push("Missing Must have scope items.")
}
if (todos.length === 0) {
  errors.push("Missing todo items.")
}

for (const todo of todos) {
  for (const field of REQUIRED_TODO_FIELDS) {
    if (!todo.body.includes(field)) {
      errors.push(`Todo ${todo.id} is missing required field: ${field}`)
    }
  }
}

for (const item of mustHave) {
  const mappedTodos = todos
    .filter((todo) => sharedTerms(item, todo.body) >= 2)
    .map((todo) => todo.id)
  if (mappedTodos.length === 0) {
    errors.push(`Must-have item has no todo mapping: ${item}`)
  } else {
    console.log(`MAP_OK ${item} -> ${mappedTodos.join(",")}`)
  }
}

const todoIds = new Set(todos.map((todo) => todo.id))
const matrixIds = new Set(matrix.map((row) => row.id))

for (const todo of todos) {
  if (!matrixIds.has(todo.id)) {
    errors.push(`Dependency matrix is missing Todo ${todo.id}.`)
  }
}
for (const row of matrix) {
  if (!todoIds.has(row.id)) {
    errors.push(`Dependency matrix references unknown Todo ${row.id}.`)
  }
  for (const dependency of dependenciesFrom(row.dependsOn)) {
    if (!todoIds.has(dependency)) {
      errors.push(`Todo ${row.id} depends on unknown Todo ${dependency}.`)
    }
  }
  if (row.blocks === "" || row.parallel === "") {
    errors.push(`Dependency matrix row for Todo ${row.id} is incomplete.`)
  }
}
if (matrix.length !== matrixIds.size) {
  errors.push("Dependency matrix contains duplicate todo rows.")
}

if (errors.length > 0) {
  for (const error of errors) {
    console.error(`PLAN_ERROR ${error}`)
  }
  process.exit(1)
}

console.log(
  `PLAN_OK must_have=${mustHave.length} todos=${todos.length} matrix_rows=${matrix.length}`,
)

function sectionBullets(source, startHeading, endHeading) {
  const start = source.indexOf(startHeading)
  const end = source.indexOf(endHeading, start)
  if (start < 0 || end < 0 || end <= start) {
    return []
  }
  return source
    .slice(start + startHeading.length, end)
    .split("\n")
    .flatMap((line) => {
      const match = line.match(/^\s*-\s+(.+)$/)
      return match === null ? [] : [match[1]]
    })
}

function parseTodos(source) {
  const matches = [...source.matchAll(/^- \[[ x]\] (\d+)\. .+$/gm)]
  return matches.map((match, index) => {
    const start = match.index ?? 0
    const end = matches[index + 1]?.index ?? source.length
    return { id: Number(match[1]), body: source.slice(start, end) }
  })
}

function parseDependencyMatrix(source) {
  const heading = "### Dependency matrix"
  const start = source.indexOf(heading)
  const end = source.indexOf("## Todos", start)
  if (start < 0 || end < 0 || end <= start) {
    return []
  }
  return source
    .slice(start + heading.length, end)
    .split("\n")
    .flatMap((line) => {
      const cells = line.split("|").map((cell) => cell.trim())
      if (cells.length !== 6 || !/^\d+$/.test(cells[1] ?? "")) {
        return []
      }
      return [
        {
          id: Number(cells[1]),
          dependsOn: cells[2] ?? "",
          blocks: cells[3] ?? "",
          parallel: cells[4] ?? "",
        },
      ]
    })
}

function dependenciesFrom(value) {
  if (value === "—") {
    return []
  }
  return value.split(",").flatMap((part) => {
    const trimmed = part.trim()
    const range = trimmed.match(/^(\d+)-(\d+)$/)
    if (range !== null) {
      const start = Number(range[1])
      const end = Number(range[2])
      return Array.from({ length: end - start + 1 }, (_, index) => start + index)
    }
    return /^\d+$/.test(trimmed) ? [Number(trimmed)] : []
  })
}

function sharedTerms(left, right) {
  const leftTerms = terms(left)
  const rightTerms = new Set(terms(right))
  return [...leftTerms].filter((term) => rightTerms.has(term)).length
}

function terms(value) {
  return new Set(
    value
      .toLowerCase()
      .replaceAll(/[^a-z0-9]+/g, " ")
      .split(" ")
      .map((term) => term.replace(/s$/, ""))
      .filter((term) => term.length >= 4 && !STOP_WORDS.has(term)),
  )
}
