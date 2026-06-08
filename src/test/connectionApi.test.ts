import { describe, expect, it, vi } from "vitest";
import {
  changePassword,
  login,
  pairServer,
  probeServer,
  resetUserPassword,
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
  username: "admin",
  display_name: "Administrator",
  role: "admin",
  enabled: true,
  version: 2,
};

describe("connection API", () => {
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
    mockedCommand.mockResolvedValueOnce({ user });
    await login("admin", "long-password");
    expect(mockedCommand).toHaveBeenCalledWith("login", {
      username: "admin",
      password: "long-password",
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
});
