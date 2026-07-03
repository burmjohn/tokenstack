import type { SetupDiagnostics } from "../../lib/schemas/dashboard";

export function buildSetupDiagnosticsJson(diagnostics: SetupDiagnostics, generatedAt = new Date()) {
  return JSON.stringify(
    {
      generatedAtUtc: generatedAt.toISOString(),
      app: "TokenStack",
      diagnostics,
    },
    null,
    2,
  );
}

export function buildSetupDiagnosticsFilename(date = new Date()) {
  return `tokenstack-diagnostics-${date.toISOString().slice(0, 10)}.json`;
}
