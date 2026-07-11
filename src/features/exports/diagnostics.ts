import type { SetupDiagnostics } from "../../lib/schemas/dashboard";

export function buildSetupDiagnosticsJson(diagnostics: SetupDiagnostics, generatedAt = new Date()) {
  return JSON.stringify(
    {
      generatedAtUtc: generatedAt.toISOString(),
      app: "TokenStack",
      diagnostics: sanitizeDiagnostics(diagnostics),
    },
    null,
    2,
  );
}

const sensitiveKeys = new Set(["token", "authtoken", "accesstoken", "refreshtoken", "cookie", "cookies", "prompt", "promptbody", "responsebody", "rawresponse", "rawjsonl", "accountlabel"]);

function sanitizeDiagnostics(value: unknown): unknown {
  if (typeof value === "string") {
    const redacted = value
      .replace(/authorization\s*:\s*bearer\s+[A-Za-z0-9._~+/=-]+/gis, "[REDACTED]")
      .replace(/bearer\s+[A-Za-z0-9._~+/=-]+/gis, "[REDACTED]")
      .replace(/\beyJ[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+){1,2}\b/g, "[REDACTED]")
      .replace(/\b(?:sk-|gh[pousr]_|xox[baprs]-)[A-Za-z0-9_-]+\b/gi, "[REDACTED]")
      .replace(/\b(?:access_token|refresh_token|authorization|cookie|token)\s*[:=]?\s*\S+/gi, "[REDACTED]");
    return redacted.split(/(\s+)/).map((part) => {
      const token = part.replace(/[^A-Za-z0-9_.-]/g, "");
      return token.length >= 32 && /[A-Z]/.test(token) && /\d/.test(token) ? "[REDACTED]" : part;
    }).join("");
  }
  if (Array.isArray(value)) return value.map(sanitizeDiagnostics);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [
      key,
      sensitiveKeys.has(key.toLowerCase()) ? "[REDACTED]" : sanitizeDiagnostics(item),
    ]));
  }
  return value;
}

export function buildSetupDiagnosticsFilename(date = new Date()) {
  return `tokenstack-diagnostics-${date.toISOString().slice(0, 10)}.json`;
}
