import { invoke } from "@tauri-apps/api/core"
import "./styles.css"
import { type CivilDate, parseCivilDate, shiftCivilDate } from "./planner/civil-date"
import {
  type Note,
  type PlannerLine,
  parseNotes,
  parsePlannerLines,
  parseTaskTemplates,
  type TaskTemplate,
} from "./planner/ipc"
import { flattenVisiblePlannerLines } from "./planner/outline"

const appRoot = document.querySelector<HTMLDivElement>("#app")
if (appRoot === null) throw new Error("App root is missing")
const app = appRoot

const localToday = parseCivilDate(new Date().toISOString().slice(0, 10))
if (localToday === null) throw new Error("Unable to determine local date")
const today = localToday

let selectedDate: CivilDate = today
let lines: PlannerLine[] = []
let notes: Note[] = []
let templates: TaskTemplate[] = []
let crossedLines = new Set<string>()
const expandedLines = new Set<string>()
const previewLines = new Set<string>()
let activeView: "planner" | "notes" | "tasks" = "planner"
let draggedLineId: string | undefined
const timeEditingLines = new Set<string>()
let noteEditor: Note | "new" | null = null
let templateEditor: TaskTemplate | "new" | null = null
const DEFAULT_LINE_COUNT = 15

function isDesktop(): boolean {
  return "__TAURI_INTERNALS__" in window
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}

function render(): void {
  app.innerHTML = `
    <div class="planner-shell">
      <header class="planner-topbar">
        <div><p class="eyebrow">Notes planner</p><h1>Today, on paper</h1></div>
        <nav class="day-nav" aria-label="Day navigation">
          <button data-day="-1" type="button">Previous</button>
          <button data-day="0" type="button">Today</button>
          <button data-day="1" type="button">Next</button>
        </nav>
      </header>
      <nav class="workspace-tabs" aria-label="Workspace">
        <button class="${activeView === "planner" ? "selected" : ""}" data-view="planner" type="button" aria-current="${activeView === "planner" ? "page" : "false"}">Planner</button><button class="${activeView === "notes" ? "selected" : ""}" data-view="notes" type="button" aria-current="${activeView === "notes" ? "page" : "false"}">Notes</button><button class="${activeView === "tasks" ? "selected" : ""}" data-view="tasks" type="button" aria-current="${activeView === "tasks" ? "page" : "false"}">Tasks</button>
      </nav>
      <main>${activeView === "planner" ? renderPlanner() : activeView === "notes" ? renderNotes() : renderTasks()}</main>
    </div>`
  bindEvents()
}

function renderPlanner(): string {
  const visibleLines = flattenVisiblePlannerLines(lines)
  return `
        <div class="page-heading"><div><p class="eyebrow">Selected day</p><h2>${selectedDate.value === today.value ? "Today" : "Daily planner"}</h2></div><time datetime="${selectedDate.value}">${formatDate(selectedDate)}</time></div>
        <section class="paper-panel outline-panel" aria-labelledby="outline-title">
          <div class="section-heading"><div><p class="eyebrow">Appointments and tasks</p><h3 id="outline-title">Today’s plan</h3></div><button id="add-line" type="button">+ Add line</button></div>
          <ul class="planner-lines" aria-label="Daily plan">${visibleLines.map(renderLine).join("")}</ul>
          <datalist id="planner-time-options">${Array.from({ length: 96 }, (_, index) => `<option value="${formatMinutes(index * 15)}"></option>`).join("")}</datalist>
        </section>
      `
}

function renderNotes(): string {
  if (noteEditor !== null) return renderNoteEditor()
  return `<div class="workspace-heading"><div><p class="eyebrow">Undated writing</p><h2>Notes</h2></div><button type="button" data-new-note>Create new</button></div>
    <section class="paper-panel workspace-panel" aria-labelledby="notes-list-title"><div class="section-heading"><h3 id="notes-list-title">Your notes</h3><span class="muted">${notes.length}</span></div>${notes.length === 0 ? '<p class="empty-line">No notes yet.</p>' : `<div class="note-grid">${notes.map((note) => `<button class="note-card" type="button" data-note-id="${escapeHtml(note.id)}"><h4>${escapeHtml(note.title)}</h4><p>${escapeHtml(note.body)}</p></button>`).join("")}</div>`}</section>`
}

function renderTasks(): string {
  if (templateEditor !== null) return renderTemplateEditor()
  return `<div class="workspace-heading"><div><p class="eyebrow">Reusable planning lines</p><h2>Tasks</h2></div><button type="button" data-new-template>Create new</button></div>
    <section class="paper-panel workspace-panel" aria-labelledby="template-list-title"><div class="section-heading"><h3 id="template-list-title">Task templates</h3><span class="muted">${templates.length}</span></div>${templates.length === 0 ? '<p class="empty-line">No task templates yet.</p>' : `<div class="note-grid">${templates.map((template) => `<button class="note-card" type="button" data-template-card="${escapeHtml(template.id)}"><h4>${escapeHtml(template.title)}</h4><p>${escapeHtml(template.body)}</p>${template.timeOfDayMinutes === null ? "" : `<time>${formatMinutes(template.timeOfDayMinutes)}</time>`}</button>`).join("")}</div>`}</section>`
}

function renderNoteEditor(): string {
  const note = noteEditor === "new" ? null : noteEditor
  return `<div class="workspace-heading"><div><p class="eyebrow">${note ? "Edit note" : "New note"}</p><h2>Notes</h2></div><button type="button" data-cancel-editor>Back to notes</button></div><section class="paper-panel workspace-panel"><form id="note-form" class="editor-form"><input type="hidden" name="id" value="${note ? escapeHtml(note.id) : ""}" /><input name="title" required maxlength="200" value="${note ? escapeHtml(note.title) : ""}" placeholder="Title" aria-label="Note title" /><textarea name="body" maxlength="10000" placeholder="Write a note...">${note ? escapeHtml(note.body) : ""}</textarea><button type="submit">${note ? "Save changes" : "Create note"}</button></form></section>`
}

function renderTemplateEditor(): string {
  const template = templateEditor === "new" ? null : templateEditor
  return `<div class="workspace-heading"><div><p class="eyebrow">${template ? "Edit task" : "New task"}</p><h2>Tasks</h2></div><button type="button" data-cancel-editor>Back to tasks</button></div><section class="paper-panel workspace-panel"><form id="task-form" class="editor-form"><input type="hidden" name="id" value="${template ? escapeHtml(template.id) : ""}" /><input name="title" required maxlength="200" value="${template ? escapeHtml(template.title) : ""}" placeholder="Title" aria-label="Task title" /><textarea name="body" maxlength="10000" placeholder="Optional task details">${template ? escapeHtml(template.body) : ""}</textarea><label>Time <input name="time" type="text" inputmode="numeric" maxlength="5" pattern="^([01]\\d|2[0-3]):[0-5]\\d$" value="${template?.timeOfDayMinutes === null || template === null ? "" : formatMinutes(template.timeOfDayMinutes)}" placeholder="HH:MM" /></label><button type="submit">${template ? "Save changes" : "Create task"}</button></form></section>`
}

function formatDate(date: CivilDate): string {
  return `${date.value.slice(8, 10)}/${date.value.slice(5, 7)}/${date.value.slice(0, 4)}`
}

function formatMinutes(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`
}

function renderLine(line: PlannerLine): string {
  const depth = Math.min(lineDepth(line), 4)
  const time = line.timeOfDayMinutes === null ? "" : formatMinutes(line.timeOfDayMinutes)
  if (line.title === "")
    return `<li class="planner-line blank-line" style="--depth:${depth}" data-line-id="${escapeHtml(line.id)}" draggable="true"><input class="line-title" value="" aria-label="Blank plan item" data-line-field="title" /></li>`
  const crossed = crossedLines.has(line.id)
  const expanded = expandedLines.has(line.id)
  const preview = previewLines.has(line.id)
  const editingTime = timeEditingLines.has(line.id)
  const templateOptions = templates
    .map(
      (template) =>
        `<button type="button" data-line-action="insert-template" data-template-id="${escapeHtml(template.id)}">${escapeHtml(template.title)}</button>`,
    )
    .join("")
  const timeControl = editingTime
    ? `<input class="line-time" type="text" inputmode="numeric" list="planner-time-options" maxlength="5" pattern="^([01][0-9]|2[0-3]):[0-5][0-9]$" value="${time}" placeholder="HH:MM" aria-label="Time in 24-hour HH:MM format" data-line-field="time" />`
    : `<button class="time-toggle" type="button" data-line-action="toggle-time" aria-label="${time ? `Change time, ${time}` : "Set time"}">${time || "◷"}</button>`
  return `<li class="planner-line ${crossed ? "crossed" : ""}" style="--depth:${depth}" data-line-id="${escapeHtml(line.id)}" draggable="true"><button class="collapse" type="button" aria-label="Toggle ${escapeHtml(line.title || "blank line")}">${line.isCollapsed ? "+" : "−"}</button><span class="drag-handle" title="Drag to reorder" aria-hidden="true">⋮⋮</span>${timeControl}<input class="line-title" value="${escapeHtml(line.title)}" aria-label="Plan item" data-line-field="title" /><button class="details-toggle" type="button" data-line-action="toggle-description">${expanded ? "Hide" : "Details"}</button><button class="cross-line" type="button" aria-label="${crossed ? "Uncross" : "Cross off"} ${escapeHtml(line.title || "blank line")}">${crossed ? "✓" : "□"}</button><details class="line-actions"><summary aria-label="More actions">...</summary><div class="line-menu"><button type="button" data-line-action="next-day">Move to next day</button><button type="button" data-line-action="save-template">Save as template</button>${templateOptions ? `<span class="menu-label">Insert template</span>${templateOptions}` : ""}</div></details>${expanded ? `<div class="line-detail"><div class="detail-tabs"><button type="button" data-line-action="toggle-preview">${preview ? "Edit" : "Preview"}</button></div>${preview ? `<div class="markdown-preview">${renderMarkdown(line.description ?? "")}</div>` : `<textarea class="description-field" data-line-field="description" placeholder="Add details..." aria-label="Details for ${escapeHtml(line.title || "blank line")}">${escapeHtml(line.description ?? "")}</textarea>`}</div>` : ""}</li>`
}

function renderMarkdown(value: string): string {
  let html = escapeHtml(value)
  html = html
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/_(.+?)_/g, "<em>$1</em>")
    .replace(/`(.+?)`/g, "<code>$1</code>")
  return html.replaceAll("\n", "<br />")
}

function lineDepth(line: PlannerLine): number {
  let depth = 0
  let parent = line.parentId
  while (parent !== null) {
    const found = lines.find((candidate) => candidate.id === parent)
    if (found === undefined) break
    depth += 1
    parent = found.parentId
  }
  return depth
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
}

function storageKey(name: string): string {
  return `notes-planner:${name}`
}
function readLocal<T>(name: string, fallback: T): T {
  try {
    const value = localStorage.getItem(storageKey(name))
    return value === null ? fallback : (JSON.parse(value) as T)
  } catch {
    return fallback
  }
}
function writeLocal(name: string, value: unknown): void {
  try {
    localStorage.setItem(storageKey(name), JSON.stringify(value))
  } catch {
    /* Storage is optional. */
  }
}
function clearLocal(name: string): void {
  try {
    localStorage.removeItem(storageKey(name))
  } catch {
    /* Storage is optional. */
  }
}
function draftLine(index: number): PlannerLine {
  return {
    id: `draft-${selectedDate.value}-${index}`,
    date: selectedDate,
    parentId: null,
    siblingKey: String(index + 1).padStart(4, "0"),
    title: "",
    description: null,
    timeOfDayMinutes: null,
    isCollapsed: false,
  }
}
function fillPage(existing: PlannerLine[]): PlannerLine[] {
  const result = [...existing]
  for (let index = result.length; index < DEFAULT_LINE_COUNT; index += 1)
    result.push(draftLine(index))
  return result
}
function loadCrossedLines(): void {
  crossedLines = new Set(readLocal<string[]>(`crossed:${selectedDate.value}`, []))
}
function persistCrossedLines(): void {
  writeLocal(`crossed:${selectedDate.value}`, [...crossedLines])
}
function markDirtyLine(line: PlannerLine): void {
  const dirty = readLocal<PlannerLine[]>(`dirty-lines:${selectedDate.value}`, []).filter(
    (candidate) => candidate.id !== line.id,
  )
  writeLocal(`dirty-lines:${selectedDate.value}`, [...dirty, line])
}
function clearDirtyLine(id: string): void {
  const dirty = readLocal<PlannerLine[]>(`dirty-lines:${selectedDate.value}`, []).filter(
    (line) => line.id !== id,
  )
  if (dirty.length === 0) clearLocal(`dirty-lines:${selectedDate.value}`)
  else writeLocal(`dirty-lines:${selectedDate.value}`, dirty)
}

function reorderLineBefore(sourceId: string, targetId: string): void {
  const visible = flattenVisiblePlannerLines(lines)
  const sourceIndex = visible.findIndex((line) => line.id === sourceId)
  const targetIndex = visible.findIndex((line) => line.id === targetId)
  if (sourceIndex < 0 || targetIndex < 0) return
  const [source] = visible.splice(sourceIndex, 1)
  if (source === undefined) return
  visible.splice(targetIndex, 0, source)
  const grouped = new Map<string | null, PlannerLine[]>()
  for (const line of visible)
    grouped.set(line.parentId, [...(grouped.get(line.parentId) ?? []), line])
  for (const group of grouped.values())
    group.forEach((line, index) => {
      line.siblingKey = String(index + 1).padStart(4, "0")
    })
  writeLocal(`lines:${selectedDate.value}`, lines)
  render()
  for (const line of visible) {
    if (line.id.startsWith("draft-") || line.id.startsWith("line-")) continue
    void call("move_planner_line", {
      request: { id: line.id, parentId: line.parentId, siblingKey: line.siblingKey },
    }).catch(() => markDirtyLine(line))
  }
}

async function moveLineToNextDay(line: PlannerLine): Promise<void> {
  const nextDate = shiftCivilDate(selectedDate, 1)
  const nextLines = readLocal<PlannerLine[]>(`lines:${nextDate.value}`, [])
  const copy: PlannerLine = {
    ...line,
    id: `draft-${nextDate.value}-${nextLines.length}`,
    date: nextDate,
    parentId: null,
    siblingKey: String(nextLines.length + 1).padStart(4, "0"),
  }
  if (isDesktop() && !line.id.startsWith("draft-") && !line.id.startsWith("line-")) {
    try {
      const created = await call<unknown>("create_planner_line", {
        request: {
          date: nextDate.value,
          parentId: null,
          siblingKey: copy.siblingKey,
          title: copy.title,
          description: copy.description,
          timeOfDayMinutes: copy.timeOfDayMinutes,
        },
      })
      const parsed = parsePlannerLines([created], nextDate)[0]
      if (parsed !== undefined) copy.id = parsed.id
    } catch {
      writeLocal(`dirty-lines:${nextDate.value}`, [
        ...readLocal<PlannerLine[]>(`dirty-lines:${nextDate.value}`, []),
        copy,
      ])
    }
  }
  writeLocal(`lines:${nextDate.value}`, [...nextLines, copy])
  selectedDate = nextDate
  await loadPage()
}

async function saveLineAsTemplate(line: PlannerLine): Promise<void> {
  if (line.title.trim() === "") return
  const template: TaskTemplate = {
    id: `local-template-${Date.now()}`,
    title: line.title,
    body: line.description ?? "",
    timeOfDayMinutes: line.timeOfDayMinutes,
  }
  if (isDesktop()) {
    try {
      await call("create_task_template", {
        request: {
          title: template.title,
          body: template.body,
          timeOfDayMinutes: template.timeOfDayMinutes,
        },
      })
      templates = parseTaskTemplates(await call<unknown>("list_task_templates"))
    } catch {
      templates = [...templates, template]
      writeLocal("templates", templates)
    }
  } else {
    templates = [...templates, template]
    writeLocal("templates", templates)
  }
  render()
}

async function insertTemplateIntoLine(templateId: string, line: PlannerLine): Promise<void> {
  if (isDesktop() && !line.id.startsWith("draft-") && !line.id.startsWith("line-")) {
    try {
      await call("insert_task_template_copy", {
        request: {
          templateId,
          date: selectedDate.value,
          parentId: line.parentId,
          siblingKey: `${line.siblingKey}0`,
        },
      })
      await loadPage()
      return
    } catch {
      /* Keep the local fallback below. */
    }
  }
  const template = templates.find((candidate) => candidate.id === templateId)
  if (template === undefined) return
  lines = [
    ...lines,
    {
      ...draftLine(lines.length),
      title: template.title,
      description: template.body,
      timeOfDayMinutes: template.timeOfDayMinutes,
    },
  ]
  writeLocal(`lines:${selectedDate.value}`, lines)
  render()
}

function bindEvents(): void {
  app.querySelectorAll<HTMLButtonElement>("[data-view]").forEach((button) => {
    button.addEventListener("click", () => {
      const view = button.dataset["view"]
      if (view !== "planner" && view !== "notes" && view !== "tasks") return
      activeView = view
      void loadWorkspace()
    })
  })
  app.querySelector<HTMLFormElement>("#note-form")?.addEventListener("submit", (event) => {
    event.preventDefault()
    void createNote(new FormData(event.currentTarget as HTMLFormElement))
  })
  app.querySelector<HTMLFormElement>("#task-form")?.addEventListener("submit", (event) => {
    event.preventDefault()
    void createTemplate(new FormData(event.currentTarget as HTMLFormElement))
  })
  app.querySelector<HTMLButtonElement>("[data-new-note]")?.addEventListener("click", () => {
    noteEditor = "new"
    render()
  })
  app.querySelectorAll<HTMLButtonElement>("[data-note-id]").forEach((button) => {
    button.addEventListener("click", () => {
      noteEditor = notes.find((note) => note.id === button.dataset["noteId"]) ?? null
      render()
    })
  })
  app.querySelector<HTMLButtonElement>("[data-new-template]")?.addEventListener("click", () => {
    templateEditor = "new"
    render()
  })
  app.querySelectorAll<HTMLButtonElement>("[data-template-card]").forEach((button) => {
    button.addEventListener("click", () => {
      templateEditor =
        templates.find((template) => template.id === button.dataset["templateCard"]) ?? null
      if (templateEditor !== null) render()
    })
  })
  app.querySelector<HTMLButtonElement>("[data-cancel-editor]")?.addEventListener("click", () => {
    noteEditor = null
    templateEditor = null
    render()
  })
  app.querySelectorAll<HTMLButtonElement>("[data-day]").forEach((button) => {
    button.addEventListener("click", () => {
      const offset = Number(button.dataset["day"])
      if (offset === 0) selectedDate = today
      else if (offset === -1 || offset === 1) selectedDate = shiftCivilDate(selectedDate, offset)
      void loadPage()
    })
  })
  app.querySelectorAll<HTMLButtonElement>(".cross-line").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.closest<HTMLElement>("[data-line-id]")?.dataset["lineId"]
      if (id === undefined) return
      if (crossedLines.has(id)) crossedLines.delete(id)
      else crossedLines.add(id)
      persistCrossedLines()
      render()
    })
  })
  app.querySelectorAll<HTMLButtonElement>("[data-line-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const row = button.closest<HTMLElement>("[data-line-id]")
      const id = row?.dataset["lineId"]
      const action = button.dataset["lineAction"]
      const line = lines.find((candidate) => candidate.id === id)
      if (line === undefined || action === undefined) return
      if (action === "toggle-description") {
        if (expandedLines.has(id as string)) expandedLines.delete(id as string)
        else expandedLines.add(id as string)
        render()
      } else if (action === "toggle-preview") {
        if (previewLines.has(id as string)) previewLines.delete(id as string)
        else previewLines.add(id as string)
        render()
      } else if (action === "toggle-time") {
        if (timeEditingLines.has(id as string)) timeEditingLines.delete(id as string)
        else timeEditingLines.add(id as string)
        render()
        app
          .querySelector<HTMLInputElement>(
            `[data-line-id="${CSS.escape(id as string)}"] [data-line-field="time"]`,
          )
          ?.focus()
      } else if (action === "next-day") {
        void moveLineToNextDay(line)
      } else if (action === "save-template") {
        void saveLineAsTemplate(line)
      } else if (action === "insert-template") {
        const templateId = button.dataset["templateId"]
        if (templateId !== undefined) void insertTemplateIntoLine(templateId, line)
      }
    })
  })
  app
    .querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-line-field]")
    .forEach((input) => {
      input.addEventListener("change", () => {
        const row = input.closest<HTMLElement>("[data-line-id]")
        const id = row?.dataset["lineId"]
        const field = input.dataset["lineField"]
        const line = lines.find((candidate) => candidate.id === id)
        if (
          line === undefined ||
          (field !== "title" && field !== "time" && field !== "description")
        )
          return
        const wasBlank = line.title === ""
        if (field === "title") line.title = input.value
        else if (field === "description") line.description = input.value === "" ? null : input.value
        else {
          const time = /^(?:[01]\d|2[0-3]):[0-5]\d$/.test(input.value) ? input.value : ""
          line.timeOfDayMinutes =
            time === "" ? null : Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5))
          input.value = time
          timeEditingLines.delete(line.id)
        }
        void persistLine(line)
        if (wasBlank && line.title !== "") render()
        else if (field === "time") render()
      })
    })
  app.querySelectorAll<HTMLElement>(".planner-line").forEach((row) => {
    row.addEventListener("dragstart", (event) => {
      draggedLineId = row.dataset["lineId"]
      row.classList.add("dragging")
      if (event instanceof DragEvent && event.dataTransfer !== null) {
        event.dataTransfer.effectAllowed = "move"
        event.dataTransfer.setData("text/plain", draggedLineId ?? "")
      }
    })
    row.addEventListener("dragend", () => {
      row.classList.remove("dragging")
      draggedLineId = undefined
    })
    row.addEventListener("dragover", (event) => event.preventDefault())
    row.addEventListener("drop", (event) => {
      event.preventDefault()
      const targetId = row.dataset["lineId"]
      if (draggedLineId !== undefined && targetId !== undefined && draggedLineId !== targetId)
        reorderLineBefore(draggedLineId, targetId)
      draggedLineId = undefined
    })
  })
  app.querySelector<HTMLButtonElement>("#add-line")?.addEventListener("click", () => {
    const id = `line-${Date.now()}`
    lines = [...lines, { ...draftLine(lines.length), id }]
    render()
  })
}

async function loadPage(): Promise<void> {
  lines = []
  loadCrossedLines()
  if (templates.length === 0) {
    if (isDesktop()) {
      try {
        templates = parseTaskTemplates(await call<unknown>("list_task_templates"))
      } catch {
        templates = readLocal<TaskTemplate[]>("templates", [])
      }
    } else templates = readLocal<TaskTemplate[]>("templates", [])
  }
  if (isDesktop()) {
    try {
      let rawLines = await call<unknown>("list_planner_lines", {
        request: { date: selectedDate.value },
      })
      const pending = readLocal<PlannerLine[]>(`dirty-lines:${selectedDate.value}`, []).filter(
        (line) => line.title.trim() !== "",
      )
      for (const line of pending) {
        if (line.id.startsWith("draft-") || line.id.startsWith("line-"))
          await call("create_planner_line", {
            request: {
              date: line.date.value,
              parentId: null,
              siblingKey: line.siblingKey,
              title: line.title,
              description: null,
              timeOfDayMinutes: line.timeOfDayMinutes,
            },
          })
        else
          await call("update_planner_line", {
            request: {
              id: line.id,
              title: line.title,
              description: line.description,
              timeOfDayMinutes: line.timeOfDayMinutes,
            },
          })
        clearDirtyLine(line.id)
      }
      if (pending.length > 0)
        rawLines = await call<unknown>("list_planner_lines", {
          request: { date: selectedDate.value },
        })
      lines = fillPage(parsePlannerLines(rawLines, selectedDate))
    } catch {
      lines = fillPage(readLocal<PlannerLine[]>(`lines:${selectedDate.value}`, []))
    }
  } else lines = fillPage(readLocal<PlannerLine[]>(`lines:${selectedDate.value}`, []))
  if (lines.length === 0) lines = fillPage([])
  writeLocal(`lines:${selectedDate.value}`, lines)
  render()
}

async function persistLine(line: PlannerLine): Promise<void> {
  writeLocal(`lines:${selectedDate.value}`, lines)
  if (!isDesktop() || line.title.trim() === "") return
  try {
    if (line.id.startsWith("draft-") || line.id.startsWith("line-")) {
      const created = await call<unknown>("create_planner_line", {
        request: {
          date: line.date.value,
          parentId: null,
          siblingKey: line.siblingKey,
          title: line.title,
          description: null,
          timeOfDayMinutes: line.timeOfDayMinutes,
        },
      })
      const parsed = parsePlannerLines([created], line.date)[0]
      const oldId = line.id
      if (parsed !== undefined) line.id = parsed.id
      clearDirtyLine(oldId)
    } else
      await call("update_planner_line", {
        request: {
          id: line.id,
          title: line.title,
          description: line.description,
          timeOfDayMinutes: line.timeOfDayMinutes,
        },
      })
    clearDirtyLine(line.id)
  } catch {
    /* The local copy remains available for a later retry. */
    markDirtyLine(line)
  }
}

async function loadWorkspace(): Promise<void> {
  if (activeView === "planner") {
    await loadPage()
    return
  }
  if (isDesktop()) {
    try {
      if (activeView === "notes") {
        const pending = readLocal<Note[]>("notes", []).filter((note) =>
          note.id.startsWith("local-"),
        )
        for (const note of pending) {
          await call("create_note", { request: { title: note.title, body: note.body } })
          const remaining = readLocal<Note[]>("notes", []).filter(
            (candidate) => candidate.id !== note.id,
          )
          if (remaining.length === 0) clearLocal("notes")
          else writeLocal("notes", remaining)
        }
        notes = parseNotes(await call<unknown>("list_notes"))
        clearLocal("notes")
      } else {
        const pending = readLocal<TaskTemplate[]>("templates", []).filter((template) =>
          template.id.startsWith("local-"),
        )
        for (const template of pending) {
          await call("create_task_template", {
            request: {
              title: template.title,
              body: template.body,
              timeOfDayMinutes: template.timeOfDayMinutes,
            },
          })
          const remaining = readLocal<TaskTemplate[]>("templates", []).filter(
            (candidate) => candidate.id !== template.id,
          )
          if (remaining.length === 0) clearLocal("templates")
          else writeLocal("templates", remaining)
        }
        templates = parseTaskTemplates(await call<unknown>("list_task_templates"))
        clearLocal("templates")
      }
    } catch {
      if (activeView === "notes") notes = readLocal<Note[]>("notes", [])
      else templates = readLocal<TaskTemplate[]>("templates", [])
    }
  } else if (activeView === "notes") notes = readLocal<Note[]>("notes", [])
  else templates = readLocal<TaskTemplate[]>("templates", [])
  render()
}

async function createNote(form: FormData): Promise<void> {
  const id = String(form.get("id") ?? "")
  const note: Note = {
    id: id || `local-note-${Date.now()}`,
    title: String(form.get("title") ?? ""),
    body: String(form.get("body") ?? ""),
  }
  if (isDesktop()) {
    try {
      if (id && !id.startsWith("local-"))
        await call("update_note", { request: { id, title: note.title, body: note.body } })
      else await call("create_note", { request: { title: note.title, body: note.body } })
      notes = parseNotes(await call<unknown>("list_notes"))
    } catch {
      notes = id
        ? notes.map((candidate) => (candidate.id === id ? note : candidate))
        : [...notes, note]
      writeLocal("notes", notes)
    }
  } else {
    notes = id
      ? notes.map((candidate) => (candidate.id === id ? note : candidate))
      : [...notes, note]
    writeLocal("notes", notes)
  }
  noteEditor = null
  render()
}

async function createTemplate(form: FormData): Promise<void> {
  const id = String(form.get("id") ?? "")
  const time = String(form.get("time") ?? "")
  const timeOfDayMinutes =
    time === "" ? null : Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5))
  const template: TaskTemplate = {
    id: id || `local-template-${Date.now()}`,
    title: String(form.get("title") ?? ""),
    body: String(form.get("body") ?? ""),
    timeOfDayMinutes,
  }
  if (isDesktop()) {
    try {
      if (id && !id.startsWith("local-"))
        await call("update_task_template", {
          request: { id, title: template.title, body: template.body, timeOfDayMinutes },
        })
      else
        await call("create_task_template", {
          request: { title: template.title, body: template.body, timeOfDayMinutes },
        })
      templates = parseTaskTemplates(await call<unknown>("list_task_templates"))
    } catch {
      templates = id
        ? templates.map((candidate) => (candidate.id === id ? template : candidate))
        : [...templates, template]
      writeLocal("templates", templates)
    }
  } else {
    templates = id
      ? templates.map((candidate) => (candidate.id === id ? template : candidate))
      : [...templates, template]
    writeLocal("templates", templates)
  }
  templateEditor = null
  render()
}

render()
void loadPage()
