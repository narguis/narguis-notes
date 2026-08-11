import type { CivilDate } from "./civil-date"
import { parseCivilDate } from "./civil-date"

export class PlannerIpcError extends Error {
  constructor(message: string) {
    super(message)
    this.name = "PlannerIpcError"
  }
}

export type DailyPage = {
  date: CivilDate
  content: string
  createdAtMs: number
  updatedAtMs: number
}
export type PlannerLine = {
  id: string
  date: CivilDate
  parentId: string | null
  siblingKey: string
  title: string
  description: string | null
  timeOfDayMinutes: number | null
  isCollapsed: boolean
}
export type Note = { id: string; title: string; body: string }
export type TaskTemplate = {
  id: string
  title: string
  body: string
  timeOfDayMinutes: number | null
}

function civil(value: unknown): CivilDate {
  const result = parseCivilDate(value)
  if (result === null) throw new PlannerIpcError("Invalid civil date")
  return result
}
function text(value: unknown, field: string): string {
  if (typeof value !== "string") throw new PlannerIpcError(`Invalid ${field}`)
  return value
}
function minute(value: unknown): number | null {
  if (value === null || value === undefined) return null
  if (!Number.isInteger(value) || (value as number) < 0 || (value as number) >= 1440)
    throw new PlannerIpcError("Invalid time")
  return value as number
}
function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    throw new PlannerIpcError("Invalid IPC response")
  return value as Record<string, unknown>
}

export function parseDailyPage(value: unknown, expectedDate: CivilDate): DailyPage {
  const item = record(value)
  const date = civil(item["date"])
  if (date.value !== expectedDate.value) throw new PlannerIpcError("Daily page date mismatch")
  const createdAtMs = item["createdAtMs"]
  const updatedAtMs = item["updatedAtMs"]
  if (
    !Number.isSafeInteger(createdAtMs) ||
    !Number.isSafeInteger(updatedAtMs) ||
    (createdAtMs as number) < 0 ||
    (updatedAtMs as number) < (createdAtMs as number)
  )
    throw new PlannerIpcError("Invalid timestamps")
  return {
    date,
    content: text(item["content"], "content"),
    createdAtMs: createdAtMs as number,
    updatedAtMs: updatedAtMs as number,
  }
}

export function parseNotes(value: unknown): Note[] {
  if (!Array.isArray(value)) throw new PlannerIpcError("Invalid notes")
  return value.map((entry) => {
    const item = record(entry)
    return {
      id: text(item["id"], "note id"),
      title: text(item["title"], "note title"),
      body: text(item["body"], "note body"),
    }
  })
}

export function parseTaskTemplates(value: unknown): TaskTemplate[] {
  if (!Array.isArray(value)) throw new PlannerIpcError("Invalid task templates")
  return value.map((entry) => {
    const item = record(entry)
    return {
      id: text(item["id"], "template id"),
      title: text(item["title"], "template title"),
      body: text(item["body"], "template body"),
      timeOfDayMinutes: minute(item["timeOfDayMinutes"]),
    }
  })
}

export function parsePlannerLines(value: unknown, expectedDate: CivilDate): PlannerLine[] {
  if (!Array.isArray(value)) throw new PlannerIpcError("Invalid planner lines")
  const ids = new Set<string>()
  const lines = value.map((entry) => {
    const item = record(entry)
    const id = text(item["id"], "line id")
    const parentValue = item["parentId"]
    const descriptionValue = item["description"]
    if (!/^[A-Za-z0-9_-]+$/.test(id) || ids.has(id))
      throw new PlannerIpcError("Invalid or duplicate line id")
    ids.add(id)
    return {
      id,
      date: civil(item["date"]),
      parentId: parentValue === null ? null : text(parentValue, "parent id"),
      siblingKey: text(item["siblingKey"], "sibling key"),
      title: text(item["title"], "line title"),
      description: descriptionValue === null ? null : text(descriptionValue, "description"),
      timeOfDayMinutes: minute(item["timeOfDayMinutes"]),
      isCollapsed: item["isCollapsed"] === true,
    }
  })
  for (const line of lines) {
    if (
      line.date.value !== expectedDate.value ||
      (line.parentId !== null && !ids.has(line.parentId))
    )
      throw new PlannerIpcError("Invalid planner hierarchy")
    const seen = new Set<string>()
    let current: PlannerLine | undefined = line
    while (current !== undefined && current.parentId !== null) {
      if (seen.has(current.id)) throw new PlannerIpcError("Cyclic planner hierarchy")
      seen.add(current.id)
      current = lines.find((candidate) => candidate.id === current?.parentId)
    }
  }
  return lines
}
