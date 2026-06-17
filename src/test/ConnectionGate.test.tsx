import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ConnectionGate } from "@/features/connection/ConnectionGate";
import {
  clearTunnelKeyOnboarding,
  generateTunnelKey,
  getConnectionState,
  getTunnelStatus,
  getTunnelKeyRequestStatus,
  loadTunnelKeyOnboarding,
  login,
  probeServer,
  saveTunnelConfig,
  saveTunnelKeyOnboarding,
  submitTunnelKeyRequest,
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
  submitTunnelKeyRequest: vi.fn(),
  getTunnelKeyRequestStatus: vi.fn(),
  saveTunnelKeyOnboarding: vi.fn(),
  loadTunnelKeyOnboarding: vi.fn(),
  clearTunnelKeyOnboarding: vi.fn(),
  formatOnboardingBundle: vi.fn(),
}));

const mockedState = vi.mocked(getConnectionState);
const mockedTunnel = vi.mocked(getTunnelStatus);
const mockedGenerateTunnelKey = vi.mocked(generateTunnelKey);
const mockedSaveTunnelConfig = vi.mocked(saveTunnelConfig);
const mockedLogin = vi.mocked(login);
const mockedSubmitTunnelKeyRequest = vi.mocked(submitTunnelKeyRequest);
const mockedProbeServer = vi.mocked(probeServer);
const mockedLoadTunnelKeyOnboarding = vi.mocked(loadTunnelKeyOnboarding);
const mockedSaveTunnelKeyOnboarding = vi.mocked(saveTunnelKeyOnboarding);
const mockedGetTunnelKeyRequestStatus = vi.mocked(getTunnelKeyRequestStatus);

beforeEach(() => {
  mockedState.mockReset();
  mockedTunnel.mockReset();
  mockedGenerateTunnelKey.mockReset();
  mockedSaveTunnelConfig.mockReset();
  mockedSubmitTunnelKeyRequest.mockReset();
  mockedLogin.mockReset();
  mockedProbeServer.mockReset();
  mockedLoadTunnelKeyOnboarding.mockReset();
  mockedGetTunnelKeyRequestStatus.mockReset();
  mockedGetTunnelKeyRequestStatus.mockResolvedValue({
    id: 42,
    status: "pending",
    note: null,
    client_config: null,
  });
  vi.mocked(clearTunnelKeyOnboarding).mockReset();
  mockedLoadTunnelKeyOnboarding.mockResolvedValue(null);
  mockedSaveTunnelKeyOnboarding.mockResolvedValue(undefined);
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
    mockedProbeServer.mockResolvedValue({
      server: { product: "Fleet Server", version: "0.3.0", api_version: "v1" },
      certificate_pem: "CERT",
      fingerprint_sha256: "AA:BB",
    });
    mockedSubmitTunnelKeyRequest.mockResolvedValue({
      id: 42,
      label: "alice-workstation",
      public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
      status: "pending",
      note: null,
      status_token: "token-42",
      fingerprint_sha256: "SHA256:abc",
      created_at: "2026-06-16T10:00:00Z",
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

    const serverUrlInput = screen.getByLabelText("Server URL");
    await actor.clear(serverUrlInput);
    await actor.type(serverUrlInput, "https://fleet.example:8443");
    await actor.type(screen.getByPlaceholderText("Your name or machine tag, e.g. alice-workstation"), "alice-workstation");

    await actor.click(screen.getByRole("button", { name: "Generate This Computer's SSH Key" }));
    expect(await screen.findByDisplayValue("ssh-ed25519 AAAATEST antminer-fleet-tunnel")).toBeInTheDocument();
    expect(screen.getByDisplayValue("C:/Users/example/.ssh/antminer_fleet_tunnel")).toBeInTheDocument();
    expect(screen.getByText(/private key stays on this computer/i)).toBeInTheDocument();
    expect(screen.getByText(/restricted tunnel account/i)).toBeInTheDocument();

    await waitFor(() => expect(mockedProbeServer).toHaveBeenCalled());

    await actor.click(screen.getByRole("button", { name: "Submit Key for Admin Approval" }));

    await waitFor(() =>
      expect(mockedSubmitTunnelKeyRequest).toHaveBeenCalledWith("https://fleet.example:8443", {
        label: "alice-workstation",
        public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
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
