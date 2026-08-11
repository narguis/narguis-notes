import { expect, test } from "@playwright/test"

test("renders the planner shell", async ({ page }) => {
  await page.goto("/")
  await expect(page.getByRole("heading", { name: "Today, on paper" })).toBeVisible()
  await expect(page.getByRole("button", { name: "Planner" })).toHaveClass(/selected/)
})
