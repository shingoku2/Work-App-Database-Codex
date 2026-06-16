import { command } from "@/lib/tauri";
import type { ApproveTunnelKeyRequest, ConnectionState, PairingInfo, SubmitTunnelKeyRequest, TunnelConfigInput, TunnelKeyInfo, TunnelKeyRequest, TunnelStatus, User, UserRole } from "@/types/db";

export function getConnectionState(): Promise<ConnectionState> {
  return command<ConnectionState>("get_connection_state");
}

export function getTunnelStatus(): Promise<TunnelStatus> {
  return command<TunnelStatus>("get_tunnel_status");
}

export function generateTunnelKey(): Promise<TunnelKeyInfo> {
  return command<TunnelKeyInfo>("generate_tunnel_key");
}

export function saveTunnelConfig(input: TunnelConfigInput): Promise<TunnelStatus> {
  return command<TunnelStatus>("save_tunnel_config", { input });
}

export function startTunnelConnection(): Promise<TunnelStatus> {
  return command<TunnelStatus>("start_tunnel_connection");
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
      id: input.id,
      site_id: input.site_id ?? null,
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

// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

export function submitTunnelKeyRequest(input: SubmitTunnelKeyRequest): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("submit_tunnel_key_request", { input });
}

export function listTunnelKeyRequests(): Promise<TunnelKeyRequest[]> {
  return command<TunnelKeyRequest[]>("list_tunnel_key_requests");
}

export function approveTunnelKeyRequest(
  id: number,
  input: ApproveTunnelKeyRequest,
): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("approve_tunnel_key_request", { id, input });
}

export function rejectTunnelKeyRequest(id: number): Promise<void> {
  return command<void>("reject_tunnel_key_request", { id });
}
