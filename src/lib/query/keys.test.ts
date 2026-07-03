import { describe, expect, it } from "vitest";
import { queryKeys } from "./keys";

describe("queryKeys", () => {
  it("returns stable dashboard and refresh query keys", () => {
    expect(queryKeys.dashboard.summary("combined")).toEqual(["dashboard", "summary", "combined"]);
    expect(queryKeys.usage.daily("90d", "local")).toEqual(["usage", "daily", "90d", "local"]);
    expect(queryKeys.diagnostics.setup()).toEqual(["diagnostics", "setup"]);
    expect(queryKeys.refresh.status()).toEqual(["refresh", "status"]);
  });
});
