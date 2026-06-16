import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionGate } from "@/features/connection/ConnectionGate";
import {
  generateTunnelKey,
  getConnectionState,
  getTunnelStatus,
  login,
  saveTunnelConfig,
} from "@/features/connection/connectionApi";

vi.mock("@/features/connection/connectionApi", () => ({
  getConnectionState: vi.fn(),
  getTunnelStatus: vi.fn(),
  generateTunnelKey: vi.fn(),
  login: vi.fn(),
  pairServer: vi.fn(),
  probeServer: vi.fn(),
  saveTunnelConfig: vi.fn(),
  startTunnelConnection: vi.fn(),
  unpairServer: vi.fn(),
}));

const mockedState = vi.mocked(getConnectionState);
const mockedTunnel = vi.mocked(getTunnelStatus);
const mockedGenerateTunnelKey = vi.mocked(generateTunnelKey);
const mockedSaveTunnelConfig = vi.mocked(saveTunnelConfig);
const mockedLogin = vi.mocked(login);

beforeEach(() => {
  mockedState.mockReset();
  mockedTunnel.mockReset();
  mockedGenerateTunnelKey.mockReset();
  mockedSaveTunnelConfig.mockReset();
  mockedTunnel.mockResolvedValue({
    supported: true,
    configured: true,
    running: true,
    local_port_open: true,
    local_url: "https://localhost:8443",
    remote_target: "127.0.0.1:8443",
    process_id: 1234,
    config_path: "C:/Users/example/AppData/Local/AntminerFleetManager/fleet-tunnel.local.json",
    error: null,
  });
  mockedLogin.mockReset();
});

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
  it("starts new users with SSH tunnel setup before server pairing", async () => {
    mockedState.mockResolvedValue({
      paired: false,
      status: "unpaired",
      url: null,
      fingerprint_sha256: null,
      user: null,
      error: null,
    });
    mockedTunnel.mockResolvedValue({
      supported: true,
      configured: false,
      running: false,
      local_port_open: false,
      local_url: "https://localhost:8443",
      remote_target: "127.0.0.1:8443",
      process_id: null,
      config_path: "C:/Users/example/AppData/Local/AntminerFleetManager/fleet-tunnel.local.json",
      error: null,
    });
    mockedGenerateTunnelKey.mockResolvedValue({
      identity_file: "C:/Users/example/.ssh/antminer_fleet_tunnel",
      public_key_file: "C:/Users/example/.ssh/antminer_fleet_tunnel.pub",
      public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
    });
    mockedSaveTunnelConfig.mockResolvedValue({
      supported: true,
      configured: true,
      running: true,
      local_port_open: true,
      local_url: "https://localhost:8443",
      remote_target: "127.0.0.1:8443",
      process_id: 1234,
      config_path: "C:/Users/example/AppData/Local/AntminerFleetManager/fleet-tunnel.local.json",
      error: null,
    });
    const actor = userEvent.setup();

    renderGate();

    expect(await screen.findByRole("heading", { name: "Set up SSH tunnel" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Connect to Fleet Server" })).not.toBeInTheDocument();

    await actor.click(screen.getByRole("button", { name: "Generate This Computer's SSH Key" }));
    expect(await screen.findByDisplayValue("ssh-ed25519 AAAATEST antminer-fleet-tunnel")).toBeInTheDocument();
    expect(screen.getByDisplayValue("C:/Users/example/.ssh/antminer_fleet_tunnel")).toBeInTheDocument();

    await actor.type(screen.getByPlaceholderText("SSH destination, e.g. fleet-user@ssh-host.example"), "alice@jump.example");
    await actor.click(screen.getByRole("button", { name: "Save and Start Tunnel" }));

    await waitFor(() =>
      expect(mockedSaveTunnelConfig).toHaveBeenCalledWith({
        ssh_destination: "alice@jump.example",
        ssh_port: 22,
        identity_file: "C:/Users/example/.ssh/antminer_fleet_tunnel",
        local_port: 8443,
        remote_host: "127.0.0.1",
        remote_port: 8443,
      }),
    );
  });

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

  it("signs in again from an unauthenticated saved-server state", async () => {
    const user = {
      id: 1,
      site_id: null,
      site_name: null,
      username: "admin",
      display_name: "Administrator",
      role: "admin" as const,
      enabled: true,
      version: 1,
    };
    mockedState
      .mockResolvedValueOnce({
        paired: true,
        status: "unauthenticated",
        url: "https://fleet.example:8443",
        fingerprint_sha256: "AA:BB",
        user: null,
        error: null,
      })
      .mockResolvedValue({
        paired: true,
        status: "authenticated",
        url: "https://fleet.example:8443",
        fingerprint_sha256: "AA:BB",
        user,
        error: null,
      });
    mockedLogin.mockResolvedValue({ user });
    const actor = userEvent.setup();

    renderGate();
    await actor.type(await screen.findByPlaceholderText("Username"), "admin");
    await actor.type(screen.getByPlaceholderText("Password"), "long-enough-password");
    await actor.click(screen.getByRole("button", { name: "Sign In" }));

    await waitFor(() =>
      expect(mockedLogin).toHaveBeenCalledWith("admin", "long-enough-password"),
    );
    expect(await screen.findByText("authenticated")).toBeInTheDocument();
    expect(mockedState).toHaveBeenCalledTimes(2);
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
