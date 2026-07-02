# TokenStack export design

This design adds two user-facing export features to the TokenStack Command
Center: shareable image badges and a complete usage CSV bundle. The feature
uses the existing validated dashboard summary and stays in the frontend so it
doesn't add new backend permissions, auth access, or network behavior.

## Goals

The export feature gives users a simple way to share or analyze their usage
without exposing session details they didn't ask to export. The app must offer
three badge layouts and one complete CSV export.

- Users can export three badge image styles: compact, usage focus, and full
  profile card.
- Badges include the TokenStack logo or app icon.
- Badges use public-facing copy only. They don't include internal safety terms
  such as `Read-only` or `/consume`.
- Users can export one CSV file with metrics, daily usage, reset credits,
  recent sessions, rate-limit windows, and source coverage.
- Exports use only fields already present in the `DashboardSummary` DTO.

## Non-goals

The first version avoids broader sharing and reporting features. These choices
keep the implementation small and aligned with the current dashboard.

- No cloud sharing or hosted badge URL.
- No direct social posting integration.
- No custom badge editor.
- No backend file-write command.
- No export of raw local history files, raw auth files, or auth-adjacent data.

## User experience

The dashboard header gains export controls near the existing refresh and theme
actions. The controls must stay compact so the first-screen dashboard layout
continues to work on desktop and mobile.

- **Export badge** opens an inline export panel.
- **Export CSV** downloads the dashboard usage bundle directly.
- The badge panel shows a segmented layout selector for **Compact**, **Usage**,
  and **Profile**.
- The selected badge shows a preview and a **Download PNG** button.
- The CSV button downloads `tokenstack-usage-YYYY-MM-DD.csv`.

The badge panel closes after download or when the user selects the export button
again. If dashboard data is still loading, the export controls stay disabled and
use tooltips to explain that exports require loaded dashboard data.

## Badge layouts

All badge layouts use the same source data and logo asset. The visual language
must be branded, dense, and polished enough for a README, blog post, or social
share.

### Compact badge

The compact badge emphasizes lifetime usage. It is the smallest and most
shareable format.

- Logo and `TokenStack` lockup.
- `Usage badge` label.
- Lifetime tokens as the hero value.
- Today, reset credits, and timezone as supporting stats.
- Snapshot year label.

### Usage focus badge

The usage badge emphasizes recent momentum. It is the best option for users who
want to show monthly output.

- Logo and `TokenStack` lockup.
- `Monthly output` label.
- This-month tokens as the hero value.
- Peak session, month delta, and coverage as supporting stats.
- Small decorative sparkline based on recent heatmap intensity.

### Full profile card

The profile card is the richest share image. It works best for project pages or
posts with more space.

- Logo and `TokenStack` lockup.
- `Usage profile` label.
- Lifetime, this-month, today, and reset-credit summary.
- Source coverage, peak session, and reset timezone as supporting stats.
- Small decorative sparkline based on recent heatmap intensity.

## Logo handling

Implementation must use the real TokenStack app icon, not a placeholder. The
frontend must keep a web-safe copy at `src/assets/tokenstack-icon.png`, derived
from `src-tauri/icons/icon.png`, so Vite preview and the packaged WebView use
the same badge logo path.

The badge renderer must load the logo before drawing the canvas. If loading the
logo fails, the download must still work with a simple `TS` monogram fallback
and a non-blocking warning in the export panel.

## CSV format

The CSV export is one file with named sections. Each section has its own header,
with blank lines separating sections so spreadsheet tools remain readable.

The file includes these sections:

- `metadata`: generated timestamp, data mode, refresh status, timezone, and
  last refresh label.
- `metrics`: metric key, label, value, delta, status, coverage percent,
  confidence, and source kind.
- `daily_usage`: date, weekday, tokens, and intensity.
- `reset_credits`: credit count, UTC expiration, New York expiration, days
  remaining, and confidence.
- `recent_sessions`: start time, duration, tokens, peak tokens, mode, and
  semicolon-separated sources.
- `rate_limit_windows`: window, limit, used, remaining, reset countdown, and
  progress percent.
- `source_coverage`: metric key, source kind, coverage percent, confidence,
  last evidence timestamp, formula version, missing facets, and explanation.

CSV generation must quote fields according to RFC 4180-style escaping: wrap a
field in quotes when it contains a comma, quote, newline, or carriage return;
double quotes inside quoted fields.

## Architecture

The feature stays in the React frontend and uses the current dashboard query
data. It doesn't need new Tauri commands.

Proposed modules:

- `src/features/exports/csv.ts`: pure functions that convert a
  `DashboardSummary` into CSV text and file names.
- `src/features/exports/badges.ts`: pure badge layout definitions and canvas
  draw helpers.
- `src/features/exports/download.ts`: browser download helpers for text and
  canvas blobs.
- `src/components/command-center/ExportPanel.tsx`: the inline panel that
  previews badge choices and triggers downloads.

`CommandCenterShell` owns the export panel visibility and passes the loaded
`DashboardSummary` into export controls. The export modules must not import
query hooks, Tauri APIs, or auth-related code.

## Data flow

The data flow starts with the already-loaded dashboard summary and ends with a
local browser download.

1. `useDashboardSummary(dataMode)` returns a validated `DashboardSummary`.
2. The user selects **Export CSV** or opens **Export badge**.
3. CSV export calls `buildDashboardUsageCsv(summary)`.
4. Badge export calls `renderUsageBadge(summary, layout, logo)`.
5. Browser download helpers create a temporary object URL, click a temporary
   anchor, and revoke the URL after the download starts.

No export path reads from SQLite, local files, auth locations, or remote
connectors directly.

## Error handling

Exports are local and deterministic, but the UI still needs clear failure
states.

- Disable export controls until summary data exists.
- Show a panel warning if logo loading fails and use the `TS` fallback.
- Show a panel warning if canvas export returns `null`.
- Keep the existing dashboard visible if export fails.
- Never include raw exception objects in downloaded files or visible messages.

## Accessibility

Export controls must be keyboard usable and screen-reader labeled.

- Header buttons use icon plus accessible labels.
- The badge layout selector exposes the selected layout.
- Badge previews use descriptive `aria-label` text.
- Download buttons state the file type and selected badge layout.
- The CSV button states that it exports a dashboard usage bundle.

## Testing

Implementation must follow test-driven development. Tests come before
production code.

- Unit-test CSV escaping, section order, and representative rows.
- Unit-test badge layout model output for all three layouts.
- Component-test that export controls render only when summary data is loaded.
- Component-test CSV download by mocking `URL.createObjectURL`, anchor clicks,
  and `URL.revokeObjectURL`.
- Component-test badge layout selection and PNG download with a mocked canvas.
- Run existing dashboard rendering tests after adding the panel.
- Run `pnpm test`, `pnpm typecheck`, and targeted browser/screenshot checks if
  layout changes affect the first viewport.

## Implementation decisions

These decisions remove ambiguity for the implementation plan.

- Badge PNG output uses `1200x630` dimensions.
- The exported image file name is
  `tokenstack-badge-LAYOUT-YYYY-MM-DD.png`.
- The CSV file name is `tokenstack-usage-YYYY-MM-DD.csv`.
- Badge copy must not include `Read-only`, `/consume`, or other internal
  safety jargon.
