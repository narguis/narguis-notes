import { describe, expect, it } from "vitest"
import { AUTOSAVE_DELAY_MS } from "../src/planner/autosave"
import { parseCivilDate, shiftCivilDate } from "../src/planner/civil-date"
import {
  PlannerIpcError,
  parseDailyPage,
  parseNotes,
  parsePlannerLines,
  parseTaskTemplates,
} from "../src/planner/ipc"
import { flattenVisiblePlannerLines } from "../src/planner/outline"

describe("civil-date navigation", () => {
  it("rejects non-civil values without UTC conversion", () => {
    // Given: values that contain a time zone or an impossible calendar day
    // When: each value crosses the civil-date parser
    // Then: no planner identity is created
    expect(parseCivilDate("2026-07-30T00:00:00Z")).toBeNull()
    expect(parseCivilDate("2026-02-30")).toBeNull()
  })

  it("moves across leap days and year boundaries", () => {
    // Given: canonical civil dates at calendar boundaries
    // When: previous and next day navigation is applied
    // Then: the result follows calendar arithmetic exactly
    const leapDay = parseCivilDate("2024-02-29")
    const newYear = parseCivilDate("2026-01-01")
    const newYearsEve = parseCivilDate("2026-12-31")

    expect(leapDay).not.toBeNull()
    expect(newYear).not.toBeNull()
    expect(newYearsEve).not.toBeNull()
    if (leapDay === null || newYear === null || newYearsEve === null) {
      return
    }

    expect(shiftCivilDate(leapDay, -1).value).toBe("2024-02-28")
    expect(shiftCivilDate(leapDay, 1).value).toBe("2024-03-01")
    expect(shiftCivilDate(newYear, -1).value).toBe("2025-12-31")
    expect(shiftCivilDate(newYearsEve, 1).value).toBe("2027-01-01")
  })
})

describe("planner autosave", () => {
  it("uses the required 750ms idle window", () => {
    // Given: the Todo 7 autosave contract
    // When: the shared debounce window is read
    // Then: prose and line text use 750ms of idle time
    expect(AUTOSAVE_DELAY_MS).toBe(750)
  })
})

describe("daily page IPC responses", () => {
  it("rejects mismatched dates and invalid timestamp order", () => {
    // Given: a response that does not belong to the requested day
    // When: the typed response parser validates it
    // Then: the renderer rejects the response before it enters planner state
    const expectedDate = parseCivilDate("2026-07-30")
    expect(expectedDate).not.toBeNull()
    if (expectedDate === null) {
      return
    }

    expect(() =>
      parseDailyPage(
        { date: "2026-07-29", content: "old", createdAtMs: 1, updatedAtMs: 2 },
        expectedDate,
      ),
    ).toThrow(PlannerIpcError)
    expect(() =>
      parseDailyPage(
        { date: "2026-07-30", content: "bad", createdAtMs: -1, updatedAtMs: 0 },
        expectedDate,
      ),
    ).toThrow(PlannerIpcError)
    expect(() =>
      parseDailyPage(
        { date: "2026-07-30", content: "bad", createdAtMs: 3, updatedAtMs: 2 },
        expectedDate,
      ),
    ).toThrow(PlannerIpcError)
  })
})

describe("planner line IPC responses", () => {
  it("parses a typed line list and rejects malformed time values", () => {
    // Given: a Todo 6 line response with a local minute time
    // When: the response crosses the frontend command boundary
    // Then: the line remains typed, while malformed minutes are rejected
    const date = parseCivilDate("2026-07-30")
    expect(date).not.toBeNull()
    if (date === null) {
      return
    }

    const lines = parsePlannerLines(
      [
        {
          id: "root",
          date: "2026-07-30",
          parentId: null,
          siblingKey: "0001",
          title: "Plan",
          description: null,
          timeOfDayMinutes: 571,
          isCollapsed: false,
        },
      ],
      date,
    )

    expect(lines[0]?.timeOfDayMinutes).toBe(571)
    expect(() =>
      parsePlannerLines(
        [
          {
            id: "root",
            date: "2026-07-30",
            parentId: null,
            siblingKey: "0001",
            title: "Plan",
            description: null,
            timeOfDayMinutes: 1440,
            isCollapsed: false,
          },
        ],
        date,
      ),
    ).toThrow(PlannerIpcError)
  })

  it("rejects duplicate, orphaned, and cyclic hierarchy responses", () => {
    // Given: line responses that cannot form a safe daily tree
    // When: the full line collection crosses the typed boundary
    // Then: malformed graph data is rejected before DOM traversal
    const date = parseCivilDate("2026-07-30")
    expect(date).not.toBeNull()
    if (date === null) {
      return
    }

    const line = (id: string, parentId: string | null) => ({
      id,
      date: "2026-07-30",
      parentId,
      siblingKey: "0001",
      title: id,
      description: null,
      timeOfDayMinutes: null,
      isCollapsed: false,
    })

    expect(() => parsePlannerLines([line("root", null), line("root", null)], date)).toThrow(
      PlannerIpcError,
    )
    expect(() => parsePlannerLines([line("child", "missing")], date)).toThrow(PlannerIpcError)
    expect(() => parsePlannerLines([line("a", "b"), line("b", "a")], date)).toThrow(PlannerIpcError)
    expect(() => parsePlannerLines([line("bad]id", null)], date)).toThrow(PlannerIpcError)
  })
})

describe("normal note and task IPC responses", () => {
  it("parses isolated notes and rejects malformed records", () => {
    // Given: a typed normal-note response and a malformed response
    // When: both values cross the renderer command boundary
    // Then: only the valid note is admitted into application state
    expect(
      parseNotes([{ id: "note-1", title: "Loose thought", body: "Keep this separate" }]),
    ).toEqual([{ id: "note-1", title: "Loose thought", body: "Keep this separate" }])
    expect(() => parseNotes([{ id: "note-1", title: "", body: 7 }])).toThrow(PlannerIpcError)
  })

  it("parses reusable one-line task templates and preserves optional time", () => {
    // Given: a reusable task template returned by native storage
    // When: the template crosses the typed command boundary
    // Then: its title, body, and optional local minute remain independent fields
    expect(
      parseTaskTemplates([
        { id: "template-1", title: "Review", body: "Confirm owners", timeOfDayMinutes: 571 },
      ]),
    ).toEqual([
      { id: "template-1", title: "Review", body: "Confirm owners", timeOfDayMinutes: 571 },
    ])
    expect(() =>
      parseTaskTemplates([
        { id: "template-1", title: "Review", body: "Confirm owners", timeOfDayMinutes: 1440 },
      ]),
    ).toThrow(PlannerIpcError)
  })
})

describe("visible planner tree", () => {
  it("keeps descendants out of the visible keyboard order when a parent is collapsed", () => {
    // Given: a root, child, and grandchild line with the root collapsed
    // When: the tree computes visible nodes
    // Then: only the collapsed root remains keyboard-visible
    const visible = flattenVisiblePlannerLines([
      {
        id: "root",
        date: { value: "2026-07-30" },
        parentId: null,
        siblingKey: "0001",
        title: "Plan",
        description: null,
        timeOfDayMinutes: null,
        isCollapsed: true,
      },
      {
        id: "child",
        date: { value: "2026-07-30" },
        parentId: "root",
        siblingKey: "0001",
        title: "Step",
        description: null,
        timeOfDayMinutes: null,
        isCollapsed: false,
      },
      {
        id: "grandchild",
        date: { value: "2026-07-30" },
        parentId: "child",
        siblingKey: "0001",
        title: "Detail",
        description: null,
        timeOfDayMinutes: null,
        isCollapsed: false,
      },
    ])

    expect(visible.map((line) => line.id)).toEqual(["root"])
  })
})
