import type { PlannerLine } from "./ipc"

export function flattenVisiblePlannerLines(lines: PlannerLine[]): PlannerLine[] {
  const children = new Map<string | null, PlannerLine[]>()
  for (const line of lines)
    children.set(line.parentId, [...(children.get(line.parentId) ?? []), line])
  for (const entries of children.values())
    entries.sort((a, b) => a.siblingKey.localeCompare(b.siblingKey))
  const visible: PlannerLine[] = []
  function visit(parentId: string | null): void {
    for (const line of children.get(parentId) ?? []) {
      visible.push(line)
      if (!line.isCollapsed) visit(line.id)
    }
  }
  visit(null)
  return visible
}
