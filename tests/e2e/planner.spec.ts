import { expect, test } from "@playwright/test"

test("starts each planner day with a clean page of lines", async ({ page }) => {
  await page.goto("/")
  await expect(page.locator(".planner-line")).toHaveCount(15)
  await expect(page.locator(".blank-line .line-title").first()).not.toHaveAttribute("placeholder")
  await expect(page.getByText("A fresh page starts with 15 lines")).toHaveCount(0)
})

test("edits a line, sets and clears a 24-hour time, and crosses it off", async ({ page }) => {
  await page.goto("/")
  const blank = page.locator(".blank-line .line-title").first()
  await blank.fill("Review release")
  await blank.press("Tab")
  const row = page
    .locator(".planner-line")
    .filter({ has: page.locator(".line-title[value='Review release']") })
    .first()
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

test("opens line details and renders the lightweight Markdown preview", async ({ page }) => {
  await page.goto("/")
  const blank = page.locator(".blank-line .line-title").first()
  await blank.fill("Write details")
  await blank.press("Tab")
  const row = page
    .locator(".planner-line")
    .filter({ has: page.locator(".line-title[value='Write details']") })
    .first()
  await row.getByRole("button", { name: "Details", exact: true }).click()
  await row.locator("textarea.description-field").fill("**Important** and `ready`")
  await row.getByRole("button", { name: "Preview" }).click()
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
  await expect(page.getByRole("button", { name: "Save changes" })).toBeVisible()

  await page.getByRole("button", { name: "Tasks" }).click()
  await expect(page.getByRole("button", { name: "Create new" })).toBeVisible()
  await page.getByRole("button", { name: "Create new" }).click()
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
