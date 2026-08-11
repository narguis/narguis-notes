import { invoke } from "@tauri-apps/api/core"
import "./styles.css"
import {
  type CivilDate,
  civilWeekday,
  parseCivilDate,
  shiftCivilDate,
  startOfWeek,
} from "./planner/civil-date"
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
const editingLineFields = new Set<string>()
let activeView: "planner" | "notes" | "tasks" | "unfinished" = "planner"
let plannerMode: "daily" | "weekly" = "daily"
let selectedWeek = startOfWeek(today)
let weeklyContent = ""
let unfinishedLines: PlannerLine[] = []
let draggedLineId: string | undefined
let noteEditor: Note | "new" | null = null
let templateEditor: TaskTemplate | "new" | null = null
let notePreview: Note | null = null
let templatePreview: TaskTemplate | null = null
let templatePickerOpen = false
const DEFAULT_LINE_COUNT = 15

function isDesktop(): boolean {
  return "__TAURI_INTERNALS__" in window
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}

function render(): void {
  const weekly = activeView === "planner" && plannerMode === "weekly"
  const weeklyIsCurrent = selectedWeek.value === startOfWeek(today).value
  app.innerHTML = `
    <div class="planner-shell">
      <header class="planner-topbar">
        <div><p class="eyebrow">Narguis Notes App</p><h1>${weekly ? (weeklyIsCurrent ? "This week, on paper" : `Week of ${formatMonthDay(selectedWeek)}, on paper`) : `${selectedDayLabel()}, on paper`}</h1></div>
        <nav class="day-nav" aria-label="Planner mode">
          <button id="toggle-planner-mode" type="button">${weekly ? "Daily plan" : "Weekly plan"}</button>
        </nav>
      </header>
       <nav class="workspace-tabs" aria-label="Workspace">
         <button class="${activeView === "planner" ? "selected" : ""}" data-view="planner" type="button" aria-current="${activeView === "planner" ? "page" : "false"}">Planner</button><button class="${activeView === "unfinished" ? "selected" : ""}" data-view="unfinished" type="button" aria-current="${activeView === "unfinished" ? "page" : "false"}">Unfinished <span class="tab-count">${unfinishedLines.length || ""}</span></button><button class="${activeView === "notes" ? "selected" : ""}" data-view="notes" type="button" aria-current="${activeView === "notes" ? "page" : "false"}">Notes</button><button class="${activeView === "tasks" ? "selected" : ""}" data-view="tasks" type="button" aria-current="${activeView === "tasks" ? "page" : "false"}">Tasks</button>
       </nav>
       <main>${activeView === "planner" ? renderPlanner() : activeView === "notes" ? renderNotes() : activeView === "tasks" ? renderTasks() : renderUnfinished()}</main>
    </div>`
  bindEvents()
}

function renderPlanner(): string {
  if (plannerMode === "weekly") return renderWeeklyPlanner()
  const visibleLines = flattenVisiblePlannerLines(lines)
  return `
         <div class="page-heading"><div><p class="eyebrow">Selected day</p><h2>${selectedDayLabel()}</h2><span class="day-weekday">${weekdayName(selectedDate)}</span></div><div class="planner-actions"><button data-day="-1" type="button" aria-label="Previous day">‹</button><button data-day="0" type="button">Go to today</button><button data-day="1" type="button" aria-label="Next day">›</button></div></div>
        <section class="paper-panel outline-panel" aria-labelledby="outline-title">
           <div class="section-heading"><div><p class="eyebrow">Appointments and tasks</p><h3 id="outline-title">Plan for ${selectedDayLabel()}</h3><span class="day-weekday">${weekdayName(selectedDate)}</span></div><button id="import-task" type="button">Import from tasks</button></div>
          ${templatePickerOpen ? renderTemplatePicker() : ""}
          <ul class="planner-lines" aria-label="Daily plan">${visibleLines.map(renderLine).join("")}</ul>
          <datalist id="planner-time-options">${Array.from({ length: 96 }, (_, index) => `<option value="${formatMinutes(index * 15)}"></option>`).join("")}</datalist>
        </section>
      `
}

function renderWeeklyPlanner(): string {
  const weekEnd = shiftCivilDate(selectedWeek, 6)
  return `<div class="page-heading"><div><p class="eyebrow">Selected week</p><h2>${formatDate(selectedWeek)} - ${formatDate(weekEnd)}</h2></div><div class="planner-actions"><button data-week="-1" type="button" aria-label="Previous week">‹</button><button data-week="0" type="button">This week</button><button data-week="1" type="button" aria-label="Next week">›</button></div></div><section class="paper-panel weekly-panel"><textarea id="weekly-content" aria-label="Weekly plan" placeholder="Sketch the week, priorities, and loose plans...">${escapeHtml(weeklyContent)}</textarea></section>`
}

function renderNotes(): string {
  if (notePreview !== null) return renderNotePreview()
  if (noteEditor !== null) return renderNoteEditor()
  return `<div class="workspace-heading"><div><p class="eyebrow">Undated writing</p><h2>Notes</h2></div><button type="button" data-new-note>Create new</button></div>
    <section class="paper-panel workspace-panel" aria-labelledby="notes-list-title"><div class="section-heading"><h3 id="notes-list-title">Your notes</h3><span class="muted">${notes.length}</span></div>${notes.length === 0 ? '<p class="empty-line">No notes yet.</p>' : `<div class="note-grid">${notes.map((note) => `<div class="note-card-wrap"><button class="note-card" type="button" data-note-id="${escapeHtml(note.id)}"><h4>${escapeHtml(note.title)}</h4></button><details class="card-actions"><summary aria-label="Note actions">...</summary><div><button type="button" data-note-delete="${escapeHtml(note.id)}">Delete</button></div></details></div>`).join("")}</div>`}</section>`
}

function renderTasks(): string {
  if (templatePreview !== null) return renderTaskPreview()
  if (templateEditor !== null) return renderTemplateEditor()
  const weekdayTasks = templates.filter((task) =>
    (task.repeatDays ?? []).includes(civilWeekday(today)),
  )
  const otherTasks = templates.filter(
    (task) => !(task.repeatDays ?? []).includes(civilWeekday(today)),
  )
  return `<div class="workspace-heading"><div><p class="eyebrow">Actual reusable tasks</p><h2>Tasks</h2></div><button type="button" data-new-template>Create task</button></div>${templates.length === 0 ? '<section class="paper-panel workspace-panel"><p class="empty-line">No tasks yet.</p></section>' : `${renderTaskSection(`${weekdayName(today)} tasks`, weekdayTasks)}${renderTaskSection("Other tasks", otherTasks)}`}`
}

function renderTaskSection(title: string, tasks: TaskTemplate[]): string {
  if (tasks.length === 0) return ""
  return `<section class="paper-panel workspace-panel task-section"><div class="section-heading"><h3>${escapeHtml(title)}</h3><span class="muted">${tasks.length}</span></div><div class="note-grid">${tasks.map((task) => `<div class="note-card-wrap"><button class="note-card" type="button" data-template-card="${escapeHtml(task.id)}"><h4>${escapeHtml(task.title)}</h4></button><details class="card-actions"><summary aria-label="Task actions">...</summary><div><button type="button" data-template-delete="${escapeHtml(task.id)}">Delete</button><button type="button" data-template-today="${escapeHtml(task.id)}">Add to today</button></div></details></div>`).join("")}</div></section>`
}

function renderUnfinished(): string {
  const groups = new Map<string, PlannerLine[]>()
  for (const line of unfinishedLines)
    groups.set(line.date.value, [...(groups.get(line.date.value) ?? []), line])
  return `<div class="workspace-heading"><div><p class="eyebrow">Open work and backlog</p><h2>Unfinished</h2></div><span class="muted">${unfinishedLines.length}</span></div>${
    unfinishedLines.length === 0
      ? '<section class="paper-panel workspace-panel"><p class="empty-line">Nothing unfinished in the current planning window.</p></section>'
      : [...groups.entries()]
          .map(([, dayLines]) => {
            const date = dayLines[0]?.date
            if (date === undefined) return ""
            return `<section class="paper-panel workspace-panel unfinished-day"><div class="unfinished-day-heading"><div><h3>${formatMonthDay(date)}</h3><span class="day-weekday">${weekdayName(date)}</span></div><span class="muted">${dayLines.length}</span></div><ul class="unfinished-list">${dayLines.map((line) => `<li class="unfinished-line" data-backlog-id="${escapeHtml(line.id)}"><button class="cross-line" type="button" data-backlog-action="cross" aria-label="Cross off ${escapeHtml(line.title)}">□</button><button class="unfinished-title" type="button" data-backlog-action="open">${escapeHtml(line.title || "Untitled line")}</button><span class="deadline-badge">${deadlineText(line)}</span><details class="card-actions"><summary aria-label="Backlog actions">...</summary><div><button type="button" data-backlog-action="open">Open</button><button type="button" data-backlog-action="today">Add to today</button><button type="button" data-backlog-action="tomorrow">Add to tomorrow</button><button type="button" data-backlog-action="delete">Delete line</button></div></details></li>`).join("")}</ul></section>`
          })
          .join("")
  }`
}

function renderNotePreview(): string {
  if (notePreview === null) return ""
  return `<div class="workspace-heading"><div><p class="eyebrow">Note preview</p><h2>${escapeHtml(notePreview.title)}</h2></div><div class="planner-actions"><button type="button" data-back-to-notes>Back to notes</button><button type="button" data-edit-note>Edit</button></div></div><section class="paper-panel workspace-panel preview-panel"><div class="markdown-preview">${renderMarkdown(notePreview.body)}</div></section>`
}

function renderTaskPreview(): string {
  if (templatePreview === null) return ""
  return `<div class="workspace-heading"><div><p class="eyebrow">Task preview</p><h2>${escapeHtml(templatePreview.title)}</h2></div><div class="planner-actions"><button type="button" data-back-to-tasks>Back to tasks</button><button type="button" data-edit-template>Edit</button></div></div><section class="paper-panel workspace-panel preview-panel"><div class="task-preview-line"><strong>${escapeHtml(templatePreview.title)}</strong>${templatePreview.timeOfDayMinutes === null ? "" : `<time>${formatMinutes(templatePreview.timeOfDayMinutes)}</time>`}</div>${templatePreview.body ? `<div class="markdown-preview">${renderMarkdown(templatePreview.body)}</div>` : ""}</section>`
}

function renderNoteEditor(): string {
  const note = noteEditor === "new" ? null : noteEditor
  return `<div class="workspace-heading"><div><p class="eyebrow">${note ? "Edit note" : "New note"}</p><h2>Notes</h2></div><button type="button" data-cancel-editor>Back to notes</button></div><section class="paper-panel workspace-panel"><form id="note-form" class="editor-form"><input type="hidden" name="id" value="${note ? escapeHtml(note.id) : ""}" /><input name="title" required maxlength="200" value="${note ? escapeHtml(note.title) : ""}" placeholder="Title" aria-label="Note title" /><textarea name="body" maxlength="10000" placeholder="Write a note...">${note ? escapeHtml(note.body) : ""}</textarea><div class="editor-actions"><button type="submit">${note ? "Save changes" : "Create note"}</button>${note ? `<button type="button" data-delete-editor-note>Delete note</button>` : ""}</div></form></section>`
}

function renderTemplateEditor(): string {
  const template = templateEditor === "new" ? null : templateEditor
  return `<div class="workspace-heading"><div><p class="eyebrow">${template ? "Edit task" : "New task"}</p><h2>Tasks</h2></div><button type="button" data-cancel-editor>Back to tasks</button></div><section class="paper-panel workspace-panel"><form id="task-form" class="editor-form task-editor-form"><input type="hidden" name="id" value="${template ? escapeHtml(template.id) : ""}" /><div class="task-editor-line"><input name="title" required maxlength="200" value="${template ? escapeHtml(template.title) : ""}" placeholder="Task title" aria-label="Task title" /><label>Time <input name="time" type="text" inputmode="numeric" maxlength="5" pattern="^([01]\\d|2[0-3]):[0-5]\\d$" value="${template?.timeOfDayMinutes === null || template === null ? "" : formatMinutes(template.timeOfDayMinutes)}" placeholder="HH:MM" /></label></div><details class="task-editor-details"><summary>Details</summary><textarea name="body" maxlength="10000" placeholder="Optional task details">${template ? escapeHtml(template.body) : ""}</textarea><label>Deadline in days <input name="deadlineDays" type="number" min="0" max="365" value="${template?.deadlineDays ?? ""}" placeholder="Optional" /></label><div class="weekday-picker"><span>Repeat weekly</span><div>${["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"].map((day, index) => `<button type="button" class="weekday-button ${(template?.repeatDays ?? []).includes(index) ? "selected" : ""}" data-weekday="${index}">${day.slice(0, 3)}</button>`).join("")}</div><input type="hidden" name="repeatDays" value="${repeatDaysValue(template?.repeatDays ?? [])}" /></div></details><div class="editor-actions"><button type="submit">${template ? "Save changes" : "Create task"}</button>${template ? `<button type="button" data-delete-editor-task>Delete task</button>` : ""}</div></form></section>`
}

function formatDate(date: CivilDate): string {
  return `${date.value.slice(8, 10)}/${date.value.slice(5, 7)}/${date.value.slice(0, 4)}`
}

function formatMonthDay(date: CivilDate): string {
  const month = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ][Number(date.value.slice(5, 7)) - 1]
  return `${month} ${date.value.slice(8, 10)}`
}

function weekdayName(date: CivilDate): string {
  return (
    ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"][
      civilWeekday(date)
    ] ?? ""
  )
}

function selectedDayLabel(): string {
  if (selectedDate.value === today.value) return "Today"
  if (selectedDate.value === shiftCivilDate(today, -1).value) return "Yesterday"
  if (selectedDate.value === shiftCivilDate(today, 1).value) return "Tomorrow"
  return formatMonthDay(selectedDate)
}

function formatMinutes(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`
}

function deadlineText(line: PlannerLine, fromDate: CivilDate = today): string {
  if (line.deadlineDate == null) return ""
  let remaining = 0
  let cursor = fromDate
  const direction = line.deadlineDate.value < cursor.value ? -1 : 1
  while (cursor.value !== line.deadlineDate.value && Math.abs(remaining) <= 366) {
    cursor = shiftCivilDate(cursor, direction)
    remaining += direction
  }
  if (remaining < 0) return `${Math.abs(remaining)}d overdue`
  if (remaining === 0) return "Due today"
  return `${remaining}d left`
}

function lineIdentity(line: PlannerLine): string {
  const suffix = `-${line.date.value}`
  return line.id.startsWith("repeat-") && line.id.endsWith(suffix)
    ? line.id.slice(0, -suffix.length)
    : line.id
}

function repeatDaysValue(days: number[]): string {
  return [...new Set(days ?? [])].sort((a, b) => a - b).join(",")
}

function scheduledLines(date: CivilDate, existing: PlannerLine[] = []): PlannerLine[] {
  return templates
    .filter((template) => (template.repeatDays ?? []).includes(civilWeekday(date)))
    .filter(
      (template) =>
        !existing.some(
          (line) => line.title === template.title && (line.repeatDays ?? []).length > 0,
        ),
    )
    .map((template, index) => ({
      id: `repeat-${template.id}-${date.value}`,
      date,
      parentId: null,
      siblingKey: `r${String(index + 1).padStart(4, "0")}`,
      title: template.title,
      description: template.body || null,
      timeOfDayMinutes: template.timeOfDayMinutes,
      isCollapsed: false,
      deadlineDays: template.deadlineDays ?? null,
      deadlineDate:
        template.deadlineDays == null ? null : shiftCivilDate(date, template.deadlineDays),
      repeatDays: template.repeatDays ?? [],
      sourceTaskId: template.id,
    }))
}

function renderLine(line: PlannerLine): string {
  const depth = Math.min(lineDepth(line), 4)
  const time = line.timeOfDayMinutes === null ? "" : formatMinutes(line.timeOfDayMinutes)
  const crossed = crossedLines.has(lineIdentity(line))
  const expanded = expandedLines.has(line.id)
  const editingTitle = !crossed && editingLineFields.has(`${line.id}:title`)
  const editingDescription = !crossed && editingLineFields.has(`${line.id}:description`)
  const editingTime = !crossed && editingLineFields.has(`${line.id}:time`)
  const deadline = deadlineText(line, selectedDate)
  const timeControl = crossed
    ? `<span class="time-locked">${time || ""}</span>`
    : editingTime
      ? `<input class="line-time" type="text" inputmode="numeric" list="planner-time-options" maxlength="5" pattern="^([01][0-9]|2[0-3]):[0-5][0-9]$" value="${time}" placeholder="HH:MM" aria-label="Time in 24-hour HH:MM format" data-line-field="time" />`
      : `<button class="time-toggle" type="button" data-line-action="toggle-time" aria-label="${time ? `Change time, ${time}` : "Set time"}">${time || "◷"}</button>`
  const titleControl = editingTitle
    ? `<input class="line-title" value="${escapeHtml(line.title)}" aria-label="Plan item" data-line-field="title" />`
    : crossed
      ? `<span class="line-title-display">${escapeHtml(line.title)}</span>`
      : `<button class="line-title-display" type="button" data-line-action="edit-title" aria-label="Edit ${escapeHtml(line.title || "blank plan item")}">${escapeHtml(line.title)}</button>`
  const detailControl = editingDescription
    ? `<textarea class="description-field" data-line-field="description" placeholder="Add details..." aria-label="Details for ${escapeHtml(line.title || "blank line")}">${escapeHtml(line.description ?? "")}</textarea>`
    : crossed
      ? `<div class="markdown-preview description-display locked-description">${line.description ? renderMarkdown(line.description) : ""}</div>`
      : `<button class="markdown-preview description-display" type="button" data-line-action="edit-description" aria-label="Edit details for ${escapeHtml(line.title || "blank line")}">${line.description ? renderMarkdown(line.description) : '<span class="empty-detail">Add details...</span>'}</button>`
  return `<li class="planner-line ${line.title === "" ? "blank-line " : ""}${crossed ? "crossed" : ""}" style="--depth:${depth}" data-line-id="${escapeHtml(line.id)}"><button class="collapse" type="button" data-line-action="toggle-description" aria-label="${expanded ? "Hide details" : "Show details"}">${expanded ? "−" : "+"}</button><span class="drag-handle" draggable="true" title="Drag to reorder" aria-hidden="true">⋮⋮</span>${timeControl}${titleControl}<span class="deadline-badge">${deadline}</span><button class="cross-line" type="button" aria-label="${crossed ? "Uncross" : "Cross off"} ${escapeHtml(line.title || "blank line")}">${crossed ? "✓" : "□"}</button><details class="line-actions"><summary aria-label="More actions">...</summary><div class="line-menu"><button type="button" data-line-action="next-day">Move to next day</button><button type="button" data-line-action="save-template">Save as task</button><button type="button" data-line-action="delete-line">Delete line</button></div></details>${expanded ? `<div class="line-detail">${detailControl}</div>` : ""}</li>`
}

function renderTemplatePicker(): string {
  if (templates.length === 0)
    return `<div class="template-picker-backdrop"><section class="template-picker"><div class="template-picker-heading"><strong>Import a task</strong><button type="button" data-close-template-picker>Close</button></div><p class="empty-line">No tasks yet. Create one in Tasks.</p></section></div>`
  return `<div class="template-picker-backdrop"><section class="template-picker" aria-label="Choose a task"><div class="template-picker-heading"><strong>Import from tasks</strong><button type="button" data-close-template-picker>Close</button></div><div class="template-picker-list">${templates.map((template) => `<button type="button" data-bring-template="${escapeHtml(template.id)}"><strong>${escapeHtml(template.title)}</strong>${template.timeOfDayMinutes === null ? "" : `<time>${formatMinutes(template.timeOfDayMinutes)}</time>`}</button>`).join("")}</div></section></div>`
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
function draftLine(index: number, date: CivilDate = selectedDate): PlannerLine {
  return {
    id: `draft-${date.value}-${index}`,
    date,
    parentId: null,
    siblingKey: String(index + 1).padStart(4, "0"),
    title: "",
    description: null,
    timeOfDayMinutes: null,
    isCollapsed: false,
    deadlineDays: null,
    deadlineDate: null,
    repeatDays: [],
    sourceTaskId: null,
  }
}
function fillPage(existing: PlannerLine[], date: CivilDate = selectedDate): PlannerLine[] {
  const result = [...existing, ...scheduledLines(date, existing)]
  for (let index = result.length; index < DEFAULT_LINE_COUNT; index += 1)
    result.push(draftLine(index, date))
  return result
}
function loadCrossedLines(): void {
  crossedLines = new Set([
    ...readLocal<string[]>("crossed-entities", []),
    ...readLocal<string[]>(`crossed:${selectedDate.value}`, []),
  ])
}
function persistCrossedLines(): void {
  writeLocal("crossed-entities", [...crossedLines])
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
  const moved: PlannerLine = {
    ...line,
    date: nextDate,
    parentId: null,
    siblingKey: String(nextLines.length + 1).padStart(4, "0"),
  }
  if (isDesktop() && !line.id.startsWith("draft-") && !line.id.startsWith("line-")) {
    try {
      await call("update_planner_line", {
        request: {
          id: line.id,
          date: nextDate.value,
          title: line.title,
          description: line.description,
          timeOfDayMinutes: line.timeOfDayMinutes,
          deadlineDays: line.deadlineDays,
          deadlineDate: line.deadlineDate?.value ?? null,
          repeatDays: repeatDaysValue(line.repeatDays),
          sourceTaskId: line.sourceTaskId,
        },
      })
    } catch {
      writeLocal(`dirty-lines:${nextDate.value}`, [
        ...readLocal<PlannerLine[]>(`dirty-lines:${nextDate.value}`, []),
        moved,
      ])
    }
  }
  lines = lines.filter((candidate) => candidate.id !== line.id)
  writeLocal(`lines:${selectedDate.value}`, lines)
  writeLocal(`lines:${nextDate.value}`, [...nextLines, moved])
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
    deadlineDays: line.deadlineDays,
    repeatDays: line.repeatDays,
  }
  if (isDesktop()) {
    try {
      await call("create_task_template", {
        request: {
          title: template.title,
          body: template.body,
          timeOfDayMinutes: template.timeOfDayMinutes,
          deadlineDays: template.deadlineDays,
          repeatDays: repeatDaysValue(template.repeatDays),
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
      deadlineDays: template.deadlineDays ?? null,
      deadlineDate:
        template.deadlineDays == null ? null : shiftCivilDate(selectedDate, template.deadlineDays),
      repeatDays: [],
      sourceTaskId: template.id,
    },
  ]
  writeLocal(`lines:${selectedDate.value}`, lines)
  render()
}

async function importTaskIntoCurrentDay(templateId: string): Promise<void> {
  const task = templates.find((candidate) => candidate.id === templateId)
  if (task === undefined) return
  const freeLine = flattenVisiblePlannerLines(lines).find((candidate) => candidate.title === "")
  if (freeLine !== undefined) {
    freeLine.title = task.title
    freeLine.description = task.body || null
    freeLine.timeOfDayMinutes = task.timeOfDayMinutes
    freeLine.deadlineDays = task.deadlineDays
    freeLine.deadlineDate =
      task.deadlineDays == null ? null : shiftCivilDate(selectedDate, task.deadlineDays)
    freeLine.repeatDays = []
    freeLine.sourceTaskId = task.id
    void persistLine(freeLine)
    render()
    return
  }
  const target = [...flattenVisiblePlannerLines(lines)]
    .reverse()
    .find((candidate) => !candidate.id.startsWith("draft-") && !candidate.id.startsWith("line-"))
  if (target !== undefined) {
    await insertTemplateIntoLine(templateId, target)
    return
  }
  const inserted: PlannerLine = {
    ...draftLine(lines.length),
    id: `line-${Date.now()}`,
    title: task.title,
    description: task.body,
    timeOfDayMinutes: task.timeOfDayMinutes,
    deadlineDays: task.deadlineDays,
    deadlineDate:
      task.deadlineDays == null ? null : shiftCivilDate(selectedDate, task.deadlineDays),
    repeatDays: [],
    sourceTaskId: task.id,
  }
  lines = [...lines, inserted]
  writeLocal(`lines:${selectedDate.value}`, lines)
  void persistLine(inserted)
  render()
}

function addPlannerLine(): void {
  const id = `line-${Date.now()}`
  editingLineFields.add(`${id}:title`)
  lines = [...lines, { ...draftLine(lines.length), id }]
  writeLocal(`lines:${selectedDate.value}`, lines)
  render()
  app
    .querySelector<HTMLInputElement>(
      `.planner-line[data-line-id="${CSS.escape(lines.at(-1)?.id ?? "")}"] .line-title`,
    )
    ?.focus()
}

async function deletePlannerLine(line: PlannerLine): Promise<void> {
  const removed = new Set<string>([line.id])
  let changed = true
  while (changed) {
    changed = false
    for (const candidate of lines) {
      if (
        candidate.parentId !== null &&
        removed.has(candidate.parentId) &&
        !removed.has(candidate.id)
      ) {
        removed.add(candidate.id)
        changed = true
      }
    }
  }
  lines = lines.filter((candidate) => !removed.has(candidate.id))
  writeLocal(`lines:${selectedDate.value}`, lines)
  if (isDesktop() && !line.id.startsWith("draft-") && !line.id.startsWith("line-")) {
    try {
      await call("delete_planner_line", { request: { id: line.id } })
    } catch {
      // Keep the local deletion; the next native refresh can restore a failed delete.
    }
  }
  render()
}

async function deleteNoteById(id: string): Promise<void> {
  try {
    if (isDesktop() && !id.startsWith("local-")) await call("delete_note", { request: { id } })
  } catch {
    // Keep the local deletion visible if the native database is unavailable.
  }
  notes = notes.filter((note) => note.id !== id)
  writeLocal("notes", notes)
  render()
}

async function deleteTemplateById(id: string): Promise<void> {
  try {
    if (isDesktop() && !id.startsWith("local-"))
      await call("delete_task_template", { request: { id } })
  } catch {
    // Keep the local deletion visible if the native database is unavailable.
  }
  templates = templates.filter((template) => template.id !== id)
  writeLocal("templates", templates)
  render()
}

async function deleteBacklogLine(line: PlannerLine): Promise<void> {
  if (line.id.startsWith("repeat-")) {
    const crossed = new Set(readLocal<string[]>("crossed-entities", []))
    crossed.add(lineIdentity(line))
    writeLocal("crossed-entities", [...crossed])
  } else {
    try {
      if (isDesktop() && !line.id.startsWith("draft-") && !line.id.startsWith("line-"))
        await call("delete_planner_line", { request: { id: line.id } })
    } catch {
      // The local removal remains visible until native storage is available.
    }
    writeLocal(
      `lines:${line.date.value}`,
      readLocal<PlannerLine[]>(`lines:${line.date.value}`, []).filter(
        (candidate) => candidate.id !== line.id,
      ),
    )
  }
  await loadBacklog()
}

function commitLineField(input: HTMLInputElement | HTMLTextAreaElement): void {
  const row = input.closest<HTMLElement>("[data-line-id]")
  const id = row?.dataset["lineId"]
  const field = input.dataset["lineField"]
  const line = lines.find((candidate) => candidate.id === id)
  if (
    id === undefined ||
    line === undefined ||
    (field !== "title" && field !== "time" && field !== "description")
  )
    return
  const key = `${id}:${field}`
  if (!editingLineFields.has(key)) return
  if (field === "title") line.title = input.value
  else if (field === "description") line.description = input.value === "" ? null : input.value
  else {
    const time = /^(?:[01]\d|2[0-3]):[0-5]\d$/.test(input.value) ? input.value : ""
    line.timeOfDayMinutes =
      time === "" ? null : Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5))
  }
  editingLineFields.delete(key)
  void persistLine(line)
  render()
}

function bindEvents(): void {
  app.querySelectorAll<HTMLButtonElement>("[data-view]").forEach((button) => {
    button.addEventListener("click", () => {
      const view = button.dataset["view"]
      if (view !== "planner" && view !== "notes" && view !== "tasks" && view !== "unfinished")
        return
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
  app
    .querySelector<HTMLButtonElement>("[data-delete-editor-note]")
    ?.addEventListener("click", () => {
      if (noteEditor !== null && noteEditor !== "new") void deleteNoteById(noteEditor.id)
    })
  app
    .querySelector<HTMLButtonElement>("[data-delete-editor-task]")
    ?.addEventListener("click", () => {
      if (templateEditor !== null && templateEditor !== "new")
        void deleteTemplateById(templateEditor.id)
    })
  app.querySelectorAll<HTMLButtonElement>("[data-weekday]").forEach((button) => {
    button.addEventListener("click", () => {
      const form = button.closest("form")
      const input = form?.querySelector<HTMLInputElement>("[name='repeatDays']")
      const day = Number(button.dataset["weekday"])
      if (!input || !Number.isInteger(day)) return
      const selected = input.value === "" ? [] : input.value.split(",").map(Number)
      const next = selected.includes(day)
        ? selected.filter((value) => value !== day)
        : [...selected, day]
      input.value = repeatDaysValue(next)
      button.classList.toggle("selected", next.includes(day))
    })
  })
  app.querySelector<HTMLButtonElement>("[data-new-note]")?.addEventListener("click", () => {
    notePreview = null
    noteEditor = "new"
    render()
  })
  app.querySelectorAll<HTMLButtonElement>("[data-note-id]").forEach((button) => {
    button.addEventListener("click", () => {
      notePreview = notes.find((note) => note.id === button.dataset["noteId"]) ?? null
      noteEditor = null
      render()
    })
  })
  app.querySelectorAll<HTMLButtonElement>("[data-note-delete]").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.dataset["noteDelete"]
      if (id !== undefined) void deleteNoteById(id)
    })
  })
  app.querySelector<HTMLButtonElement>("[data-new-template]")?.addEventListener("click", () => {
    templatePreview = null
    templateEditor = "new"
    render()
  })
  app.querySelector<HTMLButtonElement>("#import-task")?.addEventListener("click", () => {
    templatePickerOpen = true
    render()
  })
  app
    .querySelector<HTMLButtonElement>("[data-close-template-picker]")
    ?.addEventListener("click", () => {
      templatePickerOpen = false
      render()
    })
  app.querySelectorAll<HTMLButtonElement>("[data-bring-template]").forEach((button) => {
    button.addEventListener("click", () => {
      const templateId = button.dataset["bringTemplate"]
      if (templateId === undefined) return
      templatePickerOpen = false
      void importTaskIntoCurrentDay(templateId)
    })
  })
  app.querySelectorAll<HTMLButtonElement>("[data-template-card]").forEach((button) => {
    button.addEventListener("click", () => {
      templatePreview =
        templates.find((template) => template.id === button.dataset["templateCard"]) ?? null
      templateEditor = null
      if (templatePreview !== null) render()
    })
  })
  app.querySelectorAll<HTMLButtonElement>("[data-template-today]").forEach((button) => {
    button.addEventListener("click", () => {
      const templateId = button.dataset["templateToday"]
      if (templateId === undefined) return
      selectedDate = today
      activeView = "planner"
      plannerMode = "daily"
      void loadPage().then(() => importTaskIntoCurrentDay(templateId))
    })
  })
  app.querySelector<HTMLButtonElement>("[data-back-to-notes]")?.addEventListener("click", () => {
    notePreview = null
    render()
  })
  app.querySelector<HTMLButtonElement>("[data-edit-note]")?.addEventListener("click", () => {
    if (notePreview === null) return
    noteEditor = notePreview
    notePreview = null
    render()
  })
  app.querySelector<HTMLButtonElement>("[data-back-to-tasks]")?.addEventListener("click", () => {
    templatePreview = null
    render()
  })
  app.querySelector<HTMLButtonElement>("[data-edit-template]")?.addEventListener("click", () => {
    if (templatePreview === null) return
    templateEditor = templatePreview
    templatePreview = null
    render()
  })
  app.querySelectorAll<HTMLButtonElement>("[data-template-delete]").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.dataset["templateDelete"]
      if (id !== undefined) void deleteTemplateById(id)
    })
  })
  app.querySelector<HTMLButtonElement>("[data-cancel-editor]")?.addEventListener("click", () => {
    noteEditor = null
    notePreview = null
    templateEditor = null
    templatePreview = null
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
  app.querySelector<HTMLButtonElement>("#toggle-planner-mode")?.addEventListener("click", () => {
    if (plannerMode === "daily") {
      plannerMode = "weekly"
      selectedWeek = startOfWeek(selectedDate)
      void loadWeeklyPage()
    } else {
      plannerMode = "daily"
      void loadPage()
    }
  })
  app.querySelectorAll<HTMLButtonElement>("[data-week]").forEach((button) => {
    button.addEventListener("click", () => {
      const offset = Number(button.dataset["week"])
      selectedWeek = offset === 0 ? startOfWeek(today) : shiftCivilDate(selectedWeek, offset * 7)
      loadWeeklyPage()
    })
  })
  app.querySelector<HTMLTextAreaElement>("#weekly-content")?.addEventListener("input", (event) => {
    weeklyContent = (event.currentTarget as HTMLTextAreaElement).value
    writeLocal(`weekly:${selectedWeek.value}`, weeklyContent)
  })
  app.querySelectorAll<HTMLButtonElement>(".cross-line").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.closest<HTMLElement>("[data-line-id]")?.dataset["lineId"]
      if (id === undefined) return
      const line = lines.find((candidate) => candidate.id === id)
      if (line === undefined) return
      const identity = lineIdentity(line)
      if (crossedLines.has(identity)) crossedLines.delete(identity)
      else crossedLines.add(identity)
      persistCrossedLines()
      render()
    })
  })
  app.querySelectorAll<HTMLButtonElement>("[data-backlog-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const item = button.closest<HTMLElement>("[data-backlog-id]")
      const line = unfinishedLines.find((candidate) => candidate.id === item?.dataset["backlogId"])
      const action = button.dataset["backlogAction"]
      if (line === undefined || action === undefined) return
      if (action === "open") {
        selectedDate = line.date
        plannerMode = "daily"
        activeView = "planner"
        void loadPage()
      } else if (action === "cross") {
        const crossed = new Set(readLocal<string[]>("crossed-entities", []))
        crossed.add(lineIdentity(line))
        writeLocal("crossed-entities", [...crossed])
        void loadBacklog()
      } else if (action === "today") void copyBacklogLine(line, today)
      else if (action === "tomorrow") void copyBacklogLine(line, shiftCivilDate(today, 1))
      else if (action === "delete") void deleteBacklogLine(line)
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
      } else if (action === "edit-title" || action === "edit-description") {
        const field = action === "edit-title" ? "title" : "description"
        editingLineFields.add(`${id}:${field}`)
        if (field === "description") expandedLines.add(id as string)
        render()
        app
          .querySelector<HTMLInputElement | HTMLTextAreaElement>(
            `[data-line-id="${CSS.escape(id as string)}"] [data-line-field="${field}"]`,
          )
          ?.focus()
      } else if (action === "toggle-time") {
        const key = `${id}:time`
        if (editingLineFields.has(key)) editingLineFields.delete(key)
        else editingLineFields.add(key)
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
      } else if (action === "delete-line") {
        void deletePlannerLine(line)
      }
    })
  })
  app
    .querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-line-field]")
    .forEach((input) => {
      input.addEventListener("keydown", (event) => {
        if (!(event instanceof KeyboardEvent)) return
        if (event.key !== "Enter" || input.dataset["lineField"] !== "title") return
        const visible = flattenVisiblePlannerLines(lines)
        const row = input.closest<HTMLElement>("[data-line-id]")
        const id = row?.dataset["lineId"]
        const line = lines.find((candidate) => candidate.id === id)
        const index = visible.findIndex((candidate) => candidate.id === id)
        if (index < 0 || line === undefined) return
        event.preventDefault()
        if (index < visible.length - 1) editingLineFields.add(`${visible[index + 1]?.id}:title`)
        commitLineField(input as HTMLInputElement)
        if (index === visible.length - 1) addPlannerLine()
        else
          app
            .querySelector<HTMLInputElement>(
              `[data-line-id="${CSS.escape(visible[index + 1]?.id ?? "")}"] .line-title`,
            )
            ?.focus()
      })
      input.addEventListener("change", () => commitLineField(input as HTMLInputElement))
      input.addEventListener("blur", () => commitLineField(input as HTMLInputElement))
      input.addEventListener("keydown", (event) => {
        if (!(event instanceof KeyboardEvent) || event.key !== "Enter") return
        if (input.dataset["lineField"] === "description") {
          event.preventDefault()
          commitLineField(input as HTMLTextAreaElement)
        }
      })
    })
  app.querySelectorAll<HTMLElement>(".planner-line").forEach((row) => {
    const handle = row.querySelector<HTMLElement>(".drag-handle")
    handle?.addEventListener("dragstart", (event) => {
      draggedLineId = row.dataset["lineId"]
      row.classList.add("dragging")
      if (event instanceof DragEvent && event.dataTransfer !== null) {
        event.dataTransfer.effectAllowed = "move"
        event.dataTransfer.setData("text/plain", draggedLineId ?? "")
      }
    })
    handle?.addEventListener("dragend", () => {
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
              description: line.description,
              timeOfDayMinutes: line.timeOfDayMinutes,
              deadlineDays: line.deadlineDays,
              deadlineDate: line.deadlineDate?.value ?? null,
              repeatDays: repeatDaysValue(line.repeatDays),
              sourceTaskId: line.sourceTaskId,
            },
          })
        else
          await call("update_planner_line", {
            request: {
              id: line.id,
              date: line.date.value,
              title: line.title,
              description: line.description,
              timeOfDayMinutes: line.timeOfDayMinutes,
              deadlineDays: line.deadlineDays,
              deadlineDate: line.deadlineDate?.value ?? null,
              repeatDays: repeatDaysValue(line.repeatDays),
              sourceTaskId: line.sourceTaskId,
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
  if (!isDesktop() || line.title.trim() === "" || line.id.startsWith("repeat-")) return
  try {
    if (line.id.startsWith("draft-") || line.id.startsWith("line-")) {
      const created = await call<unknown>("create_planner_line", {
        request: {
          date: line.date.value,
          parentId: null,
          siblingKey: line.siblingKey,
          title: line.title,
          description: line.description,
          timeOfDayMinutes: line.timeOfDayMinutes,
          deadlineDays: line.deadlineDays,
          deadlineDate: line.deadlineDate?.value ?? null,
          repeatDays: repeatDaysValue(line.repeatDays),
          sourceTaskId: line.sourceTaskId,
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
          deadlineDays: line.deadlineDays,
          deadlineDate: line.deadlineDate?.value ?? null,
          repeatDays: repeatDaysValue(line.repeatDays),
          sourceTaskId: line.sourceTaskId,
        },
      })
    clearDirtyLine(line.id)
  } catch {
    /* The local copy remains available for a later retry. */
    markDirtyLine(line)
  }
}

async function loadWeeklyPage(): Promise<void> {
  const key = `weekly:${selectedWeek.value}`
  weeklyContent = readLocal<string>(key, "")
  render()
}

async function loadBacklog(): Promise<void> {
  if (templates.length === 0) {
    if (isDesktop()) {
      try {
        templates = parseTaskTemplates(await call<unknown>("list_task_templates"))
      } catch {
        templates = readLocal<TaskTemplate[]>("templates", [])
      }
    } else templates = readLocal<TaskTemplate[]>("templates", [])
  }
  const anchor = startOfWeek(today)
  const currentWeekEnd = shiftCivilDate(anchor, 6)
  const collected: PlannerLine[] = []
  for (const date of Array.from({ length: 28 }, (_, index) => shiftCivilDate(anchor, index))) {
    let dayLines: PlannerLine[] = []
    if (isDesktop()) {
      try {
        dayLines = parsePlannerLines(
          await call<unknown>("list_planner_lines", { request: { date: date.value } }),
          date,
        )
      } catch {
        dayLines = readLocal<PlannerLine[]>(`lines:${date.value}`, [])
      }
    } else dayLines = readLocal<PlannerLine[]>(`lines:${date.value}`, [])
    const scheduled = date.value <= currentWeekEnd.value ? scheduledLines(date, dayLines) : []
    const all = [...dayLines.filter((line) => line.title.trim() !== ""), ...scheduled]
    const crossed = new Set([
      ...readLocal<string[]>("crossed-entities", []),
      ...readLocal<string[]>(`crossed:${date.value}`, []),
    ])
    collected.push(...all.filter((line) => !crossed.has(lineIdentity(line))))
  }
  unfinishedLines = collected
  render()
}

async function copyBacklogLine(line: PlannerLine, date: CivilDate): Promise<void> {
  const moved: PlannerLine = {
    ...line,
    id: line.id.startsWith("repeat-") ? `line-${Date.now()}` : line.id,
    date,
    parentId: null,
    siblingKey: String(readLocal<PlannerLine[]>(`lines:${date.value}`, []).length + 1).padStart(
      4,
      "0",
    ),
    deadlineDate: line.deadlineDays == null ? null : shiftCivilDate(date, line.deadlineDays),
    repeatDays: [],
  }
  const source = readLocal<PlannerLine[]>(`lines:${line.date.value}`, []).filter(
    (candidate) => candidate.id !== line.id,
  )
  const next = [...readLocal<PlannerLine[]>(`lines:${date.value}`, []), moved]
  writeLocal(`lines:${line.date.value}`, source)
  writeLocal(`lines:${date.value}`, next)
  if (isDesktop() && !line.id.startsWith("repeat-")) {
    try {
      await call("update_planner_line", {
        request: {
          id: line.id,
          date: date.value,
          title: moved.title,
          description: moved.description,
          timeOfDayMinutes: moved.timeOfDayMinutes,
          deadlineDays: moved.deadlineDays,
          deadlineDate: moved.deadlineDate?.value ?? null,
          repeatDays: repeatDaysValue(moved.repeatDays),
          sourceTaskId: moved.sourceTaskId,
        },
      })
    } catch {
      writeLocal(`dirty-lines:${date.value}`, [
        ...readLocal<PlannerLine[]>(`dirty-lines:${date.value}`, []),
        moved,
      ])
    }
  } else if (isDesktop()) {
    void persistLine(moved)
  }
  await loadBacklog()
}

async function loadWorkspace(): Promise<void> {
  if (activeView === "planner") {
    if (plannerMode === "weekly") await loadWeeklyPage()
    else await loadPage()
    return
  }
  if (activeView === "unfinished") {
    await loadBacklog()
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
              deadlineDays: template.deadlineDays,
              repeatDays: repeatDaysValue(template.repeatDays),
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
  const timeOfDayMinutes = /^(?:[01]\d|2[0-3]):[0-5]\d$/.test(time)
    ? Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5))
    : null
  const deadlineValue = String(form.get("deadlineDays") ?? "")
  const deadlineDays = deadlineValue === "" ? null : Number(deadlineValue)
  const repeatDays =
    String(form.get("repeatDays") ?? "") === ""
      ? []
      : String(form.get("repeatDays")).split(",").map(Number)
  const template: TaskTemplate = {
    id: id || `local-template-${Date.now()}`,
    title: String(form.get("title") ?? ""),
    body: String(form.get("body") ?? ""),
    timeOfDayMinutes,
    deadlineDays,
    repeatDays,
  }
  if (isDesktop()) {
    try {
      if (id && !id.startsWith("local-"))
        await call("update_task_template", {
          request: {
            id,
            title: template.title,
            body: template.body,
            timeOfDayMinutes,
            deadlineDays,
            repeatDays: repeatDaysValue(repeatDays),
          },
        })
      else
        await call("create_task_template", {
          request: {
            title: template.title,
            body: template.body,
            timeOfDayMinutes,
            deadlineDays,
            repeatDays: repeatDaysValue(repeatDays),
          },
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

document.addEventListener("click", (event) => {
  const target = event.target
  if (target instanceof Element) {
    const details = target.closest("details")
    if (details !== null) {
      if (target.closest("summary"))
        document.querySelectorAll<HTMLDetailsElement>("details[open]").forEach((openDetails) => {
          if (openDetails !== details) openDetails.open = false
        })
      return
    }
  }
  document.querySelectorAll<HTMLDetailsElement>("details[open]").forEach((details) => {
    details.open = false
  })
})

render()
void loadPage()
void loadBacklog()
