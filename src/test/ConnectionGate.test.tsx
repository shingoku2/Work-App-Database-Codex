import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ConnectionGate } from "@/features/connection/ConnectionGate";
import { getConnectionState } from "@/features/connection/connectionApi";

vi.mock("@/features/connection/connectionApi", () => ({
  getConnectionState: vi.fn(),
  login: vi.fn(),
  pairServer: vi.fn(),
  probeServer: vi.fn(),
  unpairServer: vi.fn(),
}));

const mockedState = vi.mocked(getConnectionState);

function renderGate() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ConnectionGate>{() => <div>authenticated</div>}</ConnectionGate>
    </QueryClientProvider>,
  );
}

describe("ConnectionGate", () => {
  it("keeps sign in enabled when the local credential is absent", async () => {
    mockedState.mockResolvedValue({
      paired: true,
      status: "unauthenticated",
      url: "https://fleet.example:8443",
      fingerprint_sha256: "AA:BB",
      user: null,
      error: null,
    });

    renderGate();

    expect(await screen.findByRole("button", { name: "Sign In" })).toBeEnabled();
  });

  it("offers re-pairing when the pinned server is unavailable", async () => {
    mockedState.mockResolvedValue({
      paired: true,
      status: "unavailable",
      url: "https://fleet.example:8443",
      fingerprint_sha256: "AA:BB",
      user: null,
      error: "Pinned certificate verification failed",
    });

    renderGate();

    expect(await screen.findByRole("button", { name: "Sign In" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Forget Server and Re-pair" })).toBeEnabled();
  });
});
