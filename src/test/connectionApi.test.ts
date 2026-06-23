import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  approveTunnelKeyRequest,
  changePassword,
  getTunnelKeyRequestStatus,
  listTunnelKeyRequests,
  login,
  pairServer,
  probeServer,
  rejectTunnelKeyRequest,
  resetUserPassword,
  revokeTunnelKeyRequest,
  submitTunnelKeyRequest,
  updateUser,
} from "@/features/connection/connectionApi";
import { command } from "@/lib/tauri";
import type { PairingInfo, User } from "@/types/db";

vi.mock("@/lib/tauri", () => ({ command: vi.fn() }));

const mockedCommand = vi.mocked(command);
const pairing: PairingInfo = {
  server: { product: "Fleet Server", version: "0.3.0", api_version: "v1" },
  certificate_pem: "CERT",
  fingerprint_sha256: "AA:BB",
};
const user: User = {
  id: 1,
  site_id: null,
  site_name: null,
  username: "admin",
  display_name: "Administrator",
  role: "admin",
  enabled: true,
  version: 2,
};

describe("connection API", () => {
  beforeEach(() => {
    mockedCommand.mockReset();
  });

  it("probes and pairs using Tauri camelCase argument names", async () => {
    mockedCommand.mockResolvedValueOnce(pairing).mockResolvedValueOnce(undefined);
    await probeServer("https://server:8443");
    await pairServer("https://server:8443", pairing);
    expect(mockedCommand).toHaveBeenNthCalledWith(1, "probe_server", { url: "https://server:8443" });
    expect(mockedCommand).toHaveBeenNthCalledWith(2, "pair_server", {
      url: "https://server:8443",
      certificatePem: "CERT",
      fingerprintSha256: "AA:BB",
    });
  });

  it("passes credentials only to login", async () => {
    mockedCommand.mockResolvedValueOnce({
      token: "session-token",
      expires_at: "2026-06-23T18:00:00Z",
      user,
    });
    const response = await login("admin", "long-password");
    expect(mockedCommand).toHaveBeenCalledWith("login", {
      username: "admin",
      password: "long-password",
    });
    expect(response).toEqual({
      token: "session-token",
      expires_at: "2026-06-23T18:00:00Z",
      user,
    });
  });

  it("maps password and user update request shapes", async () => {
    mockedCommand.mockResolvedValue(undefined);
    await changePassword("old-password", "new-password");
    await updateUser(user);
    await resetUserPassword(1, "reset-password");
    expect(mockedCommand).toHaveBeenNthCalledWith(1, "change_password", {
      input: { current_password: "old-password", new_password: "new-password" },
    });
    expect(mockedCommand).toHaveBeenNthCalledWith(2, "update_user", {
      id: 1,
      input: {
        id: 1,
        site_id: null,
        display_name: "Administrator",
        role: "admin",
        enabled: true,
        version: 2,
      },
    });
    expect(mockedCommand).toHaveBeenNthCalledWith(3, "reset_user_password", {
      id: 1,
      input: { password: "reset-password" },
    });
  });

  it("submits tunnel key requests with the pre-pair server URL", async () => {
    mockedCommand.mockResolvedValue({
      id: 42,
      label: "alice-workstation",
      public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
      status: "pending",
      note: null,
      status_token: "token-42",
      fingerprint_sha256: "SHA256:abc",
      created_at: "2026-06-16T10:00:00Z",
    });

    await submitTunnelKeyRequest("https://fleet.example:8443", {
      label: "alice-workstation",
      public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
    });

    expect(mockedCommand).toHaveBeenCalledWith("submit_tunnel_key_request", {
      serverUrl: "https://fleet.example:8443",
      input: {
        label: "alice-workstation",
        public_key: "ssh-ed25519 AAAATEST antminer-fleet-tunnel",
      },
    });
  });

  it("lists and manages tunnel key requests with camelCase args", async () => {
    mockedCommand.mockResolvedValueOnce([
      {
        id: 7,
        label: "bob-laptop",
        public_key: "ssh-ed25519 AAAATEST bob",
        status: "pending",
        note: null,
        fingerprint_sha256: "SHA256:def",
        created_at: "2026-06-16T10:00:00Z",
      },
    ]).mockResolvedValueOnce({
      id: 7,
      label: "bob-laptop",
      public_key: "ssh-ed25519 AAAATEST bob",
      status: "approved",
      note: "verified",
      status_token: "token-7",
      fingerprint_sha256: "SHA256:def",
      created_at: "2026-06-16T10:00:00Z",
    });
    await listTunnelKeyRequests();
    await approveTunnelKeyRequest(7, { note: "verified in person" });
    expect(mockedCommand).toHaveBeenNthCalledWith(1, "list_tunnel_key_requests");
    expect(mockedCommand).toHaveBeenNthCalledWith(2, "approve_tunnel_key_request", {
      id: 7,
      input: { note: "verified in person" },
    });
  });

  it("rejects and revokes tunnel key requests", async () => {
    mockedCommand.mockResolvedValue({
      id: 9,
      label: "old-laptop",
      public_key: "ssh-ed25519 AAAATEST old",
      status: "revoked",
      note: "retired",
      status_token: "token-9",
      fingerprint_sha256: null,
      created_at: "2026-06-16T10:00:00Z",
    });
    await rejectTunnelKeyRequest(9, { note: "unknown device" });
    await revokeTunnelKeyRequest(9, { note: "retired" });
    expect(mockedCommand).toHaveBeenNthCalledWith(1, "reject_tunnel_key_request", {
      id: 9,
      input: { note: "unknown device" },
    });
    expect(mockedCommand).toHaveBeenNthCalledWith(2, "revoke_tunnel_key_request", {
      id: 9,
      input: { note: "retired" },
    });
  });

  it("polls tunnel key request status before pairing", async () => {
    mockedCommand.mockResolvedValue({
      id: 42,
      status: "approved",
      note: null,
      client_config: {
        ssh_destination: "antminer-fleet-client-tunnel@10.0.0.5",
        ssh_port: 22,
        local_port: 8443,
        remote_host: "127.0.0.1",
        remote_port: 8443,
      },
    });
    await getTunnelKeyRequestStatus("https://fleet.example:8443", 42, "token-42");
    expect(mockedCommand).toHaveBeenCalledWith("get_tunnel_key_request_status", {
      serverUrl: "https://fleet.example:8443",
      id: 42,
      token: "token-42",
    });
  });
});
