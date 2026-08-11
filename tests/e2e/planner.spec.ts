import { expect, test } from "@playwright/test"

test("starts each planner day with a clean page of lines", async ({ page }) => {
  await page.goto("/")
  await expect(page.locator(".planner-line")).toHaveCount(15)
  await expect(page.locator(".blank-line .line-title-display").first()).toBeVisible()
  await expect(page.getByText("A fresh page starts with 15 lines")).toHaveCount(0)
  await expect(page.getByRole("button", { name: "Import from tasks" })).toBeVisible()
  await page.locator(".planner-line").last().locator(".line-title-display").click()
  await page.locator(".planner-line").last().locator(".line-title").press("Enter")
  await expect(page.locator(".planner-line")).toHaveCount(16)
})

test("edits a line, sets and clears a 24-hour time, and crosses it off", async ({ page }) => {
  await page.goto("/")
  await page.locator(".blank-line .line-title-display").first().click()
  const blank = page.locator(".blank-line .line-title").first()
  await blank.fill("Review release")
  await blank.press("Tab")
  const row = page.locator(".planner-line").filter({ hasText: "Review release" }).first()
  await expect(row).toBeVisible()
  await row.getByRole("button", { name: "Set time" }).click()
  await row.locator("input[data-line-field='time']").fill("09:30")
  await row.locator("input[data-line-field='time']").press("Tab")
  await expect(row.locator(".time-toggle")).toHaveText("09:30")
  await row.getByRole("button", { name: "Change time, 09:30" }).click()
  await row.locator("input[data-line-field='time']").fill("")
  await row.locator("input[data-line-field='time']").press("Tab")
  await expect(row.getByRole("button", { name: "Set time" })).toBeVisible()
  await row.getByRole("button", { name: "Cross off Review release" }).click()
  await expect(row).toHaveClass(/crossed/)
})

test("line actions keep template insertion out and expose deletion", async ({ page }) => {
  await page.goto("/")
  const row = page.locator(".planner-line").first()
  await row.locator(".line-title-display").click()
  await row.locator(".line-title").fill("Remove this")
  await row.locator(".line-title").press("Tab")
  await row.getByRole("button", { name: "More actions" }).click()
  await expect(row.getByRole("button", { name: "Delete line" })).toBeVisible()
  await expect(row.getByRole("button", { name: /Insert template/ })).toHaveCount(0)
  await row.getByRole("button", { name: "Delete line" }).click()
  await expect(row).toHaveCount(0)
})

test("opens line details and renders the lightweight Markdown preview", async ({ page }) => {
  await page.goto("/")
  await page.locator(".blank-line .line-title-display").first().click()
  const blank = page.locator(".blank-line .line-title").first()
  await blank.fill("Write details")
  await blank.press("Tab")
  const row = page.locator(".planner-line").filter({ hasText: "Write details" }).first()
  await row.getByRole("button", { name: "Show details", exact: true }).click()
  await row.locator("textarea.description-field").fill("**Important** and `ready`")
  await row.locator("textarea.description-field").press("Tab")
  await expect(row.locator(".markdown-preview strong")).toHaveText("Important")
  await expect(row.locator(".markdown-preview code")).toHaveText("ready")
})

test("notes and tasks show grids first and support create and edit", async ({ page }) => {
  await page.goto("/")
  await page.getByRole("button", { name: "Notes" }).click()
  await expect(page.getByRole("button", { name: "Create new" })).toBeVisible()
  await page.getByRole("button", { name: "Create new" }).click()
  await page.getByLabel("Note title").fill("A public note")
  await page.getByLabel("Note title").press("Tab")
  await page.locator("textarea[name='body']").fill("A note body")
  await page.getByRole("button", { name: "Create note" }).click()
  await expect(page.getByRole("button", { name: /A public note/ })).toBeVisible()
  await page.getByRole("button", { name: /A public note/ }).click()
  await expect(page.getByRole("button", { name: "Edit" })).toBeVisible()
  await page.getByRole("button", { name: "Edit" }).click()
  await expect(page.getByRole("button", { name: "Save changes" })).toBeVisible()

  await page.getByRole("button", { name: "Tasks" }).click()
  await expect(page.getByRole("button", { name: "Create task" })).toBeVisible()
  await page.getByRole("button", { name: "Create task" }).click()
  await page.getByLabel("Task title").fill("A reusable task")
  await page.locator("textarea[name='body']").fill("Task details")
  await page.getByRole("button", { name: "Create task" }).click()
  await expect(page.getByRole("button", { name: /A reusable task/ })).toBeVisible()
})

test("planner remains usable without horizontal overflow", async ({ page }) => {
  await page.goto("/")
  for (const width of [375, 768, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    const layout = await page.evaluate(() => ({
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: document.documentElement.scrollWidth,
    }))
    expect(layout.scrollWidth).toBeLessThanOrEqual(layout.clientWidth)
  }
})
