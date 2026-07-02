import { expect, test } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";

test("dashboard renders dark and light command center screenshots without console errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await expect(page.getByText("Never /consume")).toBeVisible();
  await expect(page.getByText("Daily token usage")).toBeVisible();
  await expect(page.getByText("America/New_York").first()).toBeVisible();

  const screenshotDir = path.join(process.cwd(), "docs", "screenshots");
  await fs.mkdir(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "tokenstack-dashboard-dark.png"), fullPage: true });

  await page.getByRole("button", { name: /Switch to light theme/i }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.screenshot({ path: path.join(screenshotDir, "tokenstack-dashboard-light.png"), fullPage: true });

  expect(errors).toEqual([]);
});
