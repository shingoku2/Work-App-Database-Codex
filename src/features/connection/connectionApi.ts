import { command } from "@/lib/tauri";
import type { ConnectionState, PairingInfo, User, UserRole } from "@/types/db";

export function getConnectionState(): Promise<ConnectionState> {
  return command<ConnectionState>("get_connection_state");
}

export function probeServer(url: string): Promise<PairingInfo> {
  return command<PairingInfo>("probe_server", { url });
}

export function pairServer(url: string, pairing: PairingInfo): Promise<void> {
  return command<void>("pair_server", {
    url,
    certificatePem: pairing.certificate_pem,
    fingerprintSha256: pairing.fingerprint_sha256,
  });
}

export function unpairServer(): Promise<void> {
  return command<void>("unpair_server");
}

export function login(username: string, password: string): Promise<{ user: User }> {
  return command<{ user: User }>("login", { username, password });
}

export function logout(): Promise<void> {
  return command<void>("logout");
}

export function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  return command<void>("change_password", {
    input: { current_password: currentPassword, new_password: newPassword },
  });
}

export function listUsers(): Promise<User[]> {
  return command<User[]>("list_users");
}

export function createUser(input: {
  username: string;
  display_name: string;
  password: string;
  role: UserRole;
}): Promise<User> {
  return command<User>("create_user", { input });
}

export function updateUser(input: User): Promise<User> {
  return command<User>("update_user", {
    id: input.id,
    input: {
      display_name: input.display_name,
      role: input.role,
      enabled: input.enabled,
      version: input.version,
    },
  });
}

export function resetUserPassword(id: number, password: string): Promise<void> {
  return command<void>("reset_user_password", { id, input: { password } });
}
