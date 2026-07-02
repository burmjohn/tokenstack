import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { CommandCenterShell } from "./CommandCenterShell";

function renderShell() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <CommandCenterShell />
    </QueryClientProvider>,
  );
}

describe("CommandCenterShell", () => {
  it("renders required Command Center concepts", async () => {
    renderShell();

    expect(await screen.findAllByText("TokenStack")).not.toHaveLength(0);
    expect(screen.getByRole("heading", { name: "Dashboard" })).toBeInTheDocument();
    expect(screen.getByText("Read-only")).toBeInTheDocument();
    expect(screen.getByText("Never /consume")).toBeInTheDocument();
    expect(screen.getAllByText("Local + Remote")[0]).toBeInTheDocument();
    expect(await screen.findByText("Daily token usage")).toBeInTheDocument();
    expect(screen.getByText("Reset credit timeline")).toBeInTheDocument();
    expect(screen.getByText("Source coverage")).toBeInTheDocument();
    expect(screen.getByText("Active connectors")).toBeInTheDocument();
    expect(screen.getAllByText("Undocumented (RO)")[0]).toBeInTheDocument();
    expect(screen.getAllByText(/America\/New_York/)[0]).toBeInTheDocument();
    expect(screen.getByText("All data is read-only")).toBeInTheDocument();
  });

  it("toggles theme and runs refresh without duplicate controls", async () => {
    const user = userEvent.setup();
    renderShell();

    const themeButton = await screen.findByRole("button", { name: /Switch to light theme/i });
    await user.click(themeButton);
    expect(document.documentElement.dataset.theme).toBe("light");

    const refreshButton = screen.getByRole("button", { name: "Refresh now" });
    await user.click(refreshButton);
    expect(refreshButton).toBeDisabled();
  });
});
