# TokenStack Exports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add frontend-only TokenStack badge PNG exports and a dashboard usage CSV bundle from the validated `DashboardSummary`.

**Architecture:** Pure helpers live in `src/features/exports/` and accept only the already-loaded summary. `CommandCenterShell` owns export panel visibility and delegates preview/download behavior to `ExportPanel` without adding Tauri commands or backend file access.

**Tech Stack:** React 19, Vite, Vitest/jsdom, Testing Library, Tailwind CSS, browser `canvas` and object URL download APIs.

## Global Constraints

- Implement test-first and watch each new behavior fail before production code.
- Badge layouts are Compact, Usage Focus, and Full Profile Card.
- Badge PNG output is exactly `1200x630`.
- Badge filenames are `tokenstack-badge-LAYOUT-YYYY-MM-DD.png`.
- CSV filename is `tokenstack-usage-YYYY-MM-DD.csv`.
- Badges include the real TokenStack logo/app icon via `src/assets/tokenstack-icon.png`, derived from `src-tauri/icons/icon.png`.
- Badges must not include `Read-only`, `/consume`, or internal safety jargon.
- CSV sections are ordered `metadata`, `metrics`, `daily_usage`, `reset_credits`, `recent_sessions`, `rate_limit_windows`, `source_coverage`.
- CSV escaping quotes fields containing comma, quote, newline, or carriage return, and doubles embedded quotes.
- Exports use only `DashboardSummary`; no new Tauri commands, backend file reads, or auth-adjacent data.
- `.codebase-memory` dirty artifacts stay unstaged unless intentionally updating tooling artifacts.
- `.superpowers/` remains ignored and uncommitted.

---

## File Structure

- Create `src/features/exports/csv.ts`: CSV escaping, deterministic filenames, ordered CSV bundle generation.
- Create `src/features/exports/csv.test.ts`: escaping, section order, representative rows.
- Create `src/features/exports/badges.ts`: badge layout model, logo loading, canvas rendering, filename generation.
- Create `src/features/exports/badges.test.ts`: model output for all three layouts and forbidden-copy assertions.
- Create `src/features/exports/download.ts`: browser download helpers for text and canvas blobs.
- Create `src/components/command-center/ExportPanel.tsx`: inline panel, layout selector, canvas preview, warning states, downloads.
- Modify `src/components/command-center/CommandCenterShell.tsx`: header export controls and panel placement using loaded summary only.
- Modify `src/components/command-center/CommandCenterShell.test.tsx`: component tests for loaded/disabled controls, CSV download, layout selection, PNG download.
- Create `src/assets/tokenstack-icon.png`: copied from `src-tauri/icons/icon.png`.

### Task 1: CSV Bundle Helpers

**Files:**
- Create: `src/features/exports/csv.ts`
- Test: `src/features/exports/csv.test.ts`

**Interfaces:**
- Consumes: `DashboardSummary` from `src/lib/schemas/dashboard.ts`.
- Produces: `escapeCsvField(value: unknown): string`, `buildDashboardUsageCsv(summary: DashboardSummary, generatedAt?: Date): string`, `buildUsageCsvFilename(generatedAt?: Date): string`.

- [ ] **Step 1: Write the failing tests**

```ts
import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import { buildDashboardUsageCsv, buildUsageCsvFilename, escapeCsvField } from "./csv";

describe("export CSV helpers", () => {
  it("escapes commas, quotes, newlines, and carriage returns", () => {
    expect(escapeCsvField("plain")).toBe("plain");
    expect(escapeCsvField("alpha,beta")).toBe('"alpha,beta"');
    expect(escapeCsvField('alpha "beta"')).toBe('"alpha ""beta"""');
    expect(escapeCsvField("alpha\nbeta")).toBe('"alpha\nbeta"');
    expect(escapeCsvField("alpha\rbeta")).toBe('"alpha\rbeta"');
  });

  it("writes sections in the committed design order", () => {
    const csv = buildDashboardUsageCsv(createMockDashboardSummary(), new Date("2026-07-02T19:30:00Z"));
    const sections = csv.split(/\n\n/).map((section) => section.split("\n")[0]);

    expect(sections).toEqual([
      "metadata",
      "metrics",
      "daily_usage",
      "reset_credits",
      "recent_sessions",
      "rate_limit_windows",
      "source_coverage",
    ]);
  });

  it("includes representative dashboard rows", () => {
    const csv = buildDashboardUsageCsv(createMockDashboardSummary("combined"), new Date("2026-07-02T19:30:00Z"));

    expect(csv).toContain("generated_at_utc,data_mode,refresh_status,timezone,last_refresh_label");
    expect(csv).toContain("2026-07-02T19:30:00.000Z,combined,idle,America/New_York,2m ago");
    expect(csv).toContain("metric_key,label,value,delta,status,coverage_percent,confidence,source_kind");
    expect(csv).toContain('lifetime,Lifetime tokens,38.1B,12.4% vs last 30 days,positive,72,medium,Local history');
    expect(csv).toContain("date,weekday,tokens,intensity");
    expect(csv).toContain("credit_count,expires_at_utc,expires_at_new_york,days_remaining,confidence");
    expect(csv).toContain("start_time,duration,tokens,peak_tokens,mode,sources");
    expect(csv).toContain("Jun 14, 1:12 PM,47m 23s,512.3M,1.72B,deep-research,CLI; Cloud");
    expect(csv).toContain("window,limit,used,remaining,reset_countdown,progress_percent");
    expect(csv).toContain('"1m",20.0B,11.2B,8.8B (44%),00:14,56');
    expect(csv).toContain("metric_key,source_kind,coverage_percent,confidence,last_evidence_at_utc,formula_version,missing_facets,explanation");
  });

  it("builds the required dated filename", () => {
    expect(buildUsageCsvFilename(new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-usage-2026-07-02.csv");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/features/exports/csv.test.ts`
Expected: FAIL because `src/features/exports/csv.ts` does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement `escapeCsvField`, section writer helpers, and filename formatting from `generatedAt.toISOString().slice(0, 10)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/features/exports/csv.test.ts`
Expected: PASS.

### Task 2: Badge Layout Model and Canvas Renderer

**Files:**
- Create: `src/features/exports/badges.ts`
- Test: `src/features/exports/badges.test.ts`

**Interfaces:**
- Consumes: `DashboardSummary`.
- Produces: `BADGE_WIDTH`, `BADGE_HEIGHT`, `BADGE_LAYOUTS`, `buildBadgeLayoutModel(summary, layout)`, `buildBadgeFilename(layout, generatedAt?)`, `loadBadgeLogo(src)`, `renderUsageBadge(canvas, summary, layout, logo)`.

- [ ] **Step 1: Write the failing tests**

```ts
import { describe, expect, it } from "vitest";
import { createMockDashboardSummary } from "../../lib/api/mockData";
import { BADGE_HEIGHT, BADGE_LAYOUTS, BADGE_WIDTH, buildBadgeFilename, buildBadgeLayoutModel } from "./badges";

describe("badge export model", () => {
  it("defines required output dimensions and layouts", () => {
    expect(BADGE_WIDTH).toBe(1200);
    expect(BADGE_HEIGHT).toBe(630);
    expect(BADGE_LAYOUTS.map((layout) => layout.id)).toEqual(["compact", "usage", "profile"]);
  });

  it.each([
    ["compact", "Usage badge", "38.1B"],
    ["usage", "Monthly output", "3.62B"],
    ["profile", "Usage profile", "38.1B"],
  ] as const)("builds %s layout with public copy", (layout, label, heroValue) => {
    const model = buildBadgeLayoutModel(createMockDashboardSummary(), layout);
    const copy = JSON.stringify(model);

    expect(model.label).toBe(label);
    expect(model.heroValue).toBe(heroValue);
    expect(model.brand).toBe("TokenStack");
    expect(model.stats.length).toBeGreaterThanOrEqual(3);
    expect(copy).not.toContain("Read-only");
    expect(copy).not.toContain("/consume");
    expect(copy).not.toContain("safety");
  });

  it("builds required badge filenames", () => {
    expect(buildBadgeFilename("compact", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-compact-2026-07-02.png");
    expect(buildBadgeFilename("usage", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-usage-2026-07-02.png");
    expect(buildBadgeFilename("profile", new Date("2026-07-02T19:30:00Z"))).toBe("tokenstack-badge-profile-2026-07-02.png");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/features/exports/badges.test.ts`
Expected: FAIL because `src/features/exports/badges.ts` does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement layout constants, metric lookup, source coverage average, heatmap sparkline values, filename formatting, `loadBadgeLogo`, and a canvas renderer that uses the logo image when present and a `TS` monogram fallback otherwise.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/features/exports/badges.test.ts`
Expected: PASS.

### Task 3: Download Helpers

**Files:**
- Create: `src/features/exports/download.ts`

**Interfaces:**
- Consumes: browser `Blob`, `URL`, temporary anchors, and canvas `toBlob`.
- Produces: `downloadTextFile(filename, text, type)`, `downloadCanvasPng(filename, canvas)`.

- [ ] **Step 1: Write component tests before implementation**

The shell component tests in Task 4 are the red tests for this helper because the helper is only a browser integration boundary.

- [ ] **Step 2: Implement minimal helper**

Create object URLs, click a temporary anchor, remove it, and revoke URLs. Return `Promise<boolean>` from canvas PNG download so the panel can warn on `null`.

### Task 4: Export Panel and Command Center Controls

**Files:**
- Create: `src/components/command-center/ExportPanel.tsx`
- Modify: `src/components/command-center/CommandCenterShell.tsx`
- Test: `src/components/command-center/CommandCenterShell.test.tsx`

**Interfaces:**
- Consumes: `summary?: DashboardSummary`, `isDataLoaded: boolean`.
- Produces: header buttons `Export badge` and `Export CSV`, inline `ExportPanel`, CSV/PNG browser downloads.

- [ ] **Step 1: Write failing component tests**

Add tests that render controls disabled during loading, click `Export CSV` and assert downloaded filename/object URL behavior, open `Export badge`, select `Usage`, and click `Download PNG` with a mocked canvas `toBlob`.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/command-center/CommandCenterShell.test.tsx`
Expected: FAIL because the export controls do not exist.

- [ ] **Step 3: Implement UI and download behavior**

Add icon-labeled buttons near refresh/theme controls. Keep them compact, tooltipped, keyboard-accessible, and disabled until summary exists. Render the panel below the header so it does not crowd the first viewport.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/components/command-center/CommandCenterShell.test.tsx`
Expected: PASS.

### Task 5: Logo Asset and Verification

**Files:**
- Create: `src/assets/tokenstack-icon.png`

**Interfaces:**
- Consumes: `src-tauri/icons/icon.png`.
- Produces: web-safe Vite import for badge/logo preview and canvas rendering.

- [ ] **Step 1: Copy real icon asset**

Run: `cp src-tauri/icons/icon.png src/assets/tokenstack-icon.png`.

- [ ] **Step 2: Run full verification**

Run:

```bash
pnpm test
pnpm typecheck
pnpm lint
pnpm screenshots
```

Expected: all commands pass. Screenshot command is required because the dashboard first viewport changes.

## Self-Review

- Spec coverage: badge layouts, real logo, public badge copy, CSV section order, CSV escaping, frontend-only data flow, filenames, disabled/loading controls, logo fallback warning, canvas null warning, accessibility labels, and viewport screenshot checks are assigned to tasks.
- Placeholder scan: no `TBD`, `TODO`, or unspecified tests remain.
- Type consistency: helper names in tasks match the interfaces used by component tests and implementation.
