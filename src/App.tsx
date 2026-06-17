import { useState } from "react";
import { AppShell, type ViewKey } from "@/components/layout/AppShell";
import { DashboardView } from "@/features/dashboard/DashboardView";
import { InventoryView } from "@/features/inventory/InventoryView";
import { MinersView } from "@/features/miners/MinersView";
import { AccountsView } from "@/features/accounts/AccountsView";
import { ConnectionGate } from "@/features/connection/ConnectionGate";
import { SettingsView } from "@/features/settings/SettingsView";
import { AuditLogView } from "@/features/audit/AuditLogView";
import { SitesView } from "@/features/sites/SitesView";
import { TunnelKeyRequestsPanel } from "@/features/settings/TunnelKeyRequestsView";
import type { User } from "@/types/db";

export function App() {
  return (
    <ConnectionGate>
      {(user, serverUrl) => <AuthenticatedApp user={user} serverUrl={serverUrl} />}
    </ConnectionGate>
  );
}

function AuthenticatedApp({ user, serverUrl }: { user: User; serverUrl: string }) {
  const [view, setView] = useState<ViewKey>("dashboard");

  return (
    <AppShell active={view} onNavigate={setView} user={user} serverUrl={serverUrl}>
      {view === "dashboard" && <DashboardView />}
      {view === "units" && <MinersView canImport={user.role === "admin"} />}
      {view === "inventory" && <InventoryView />}
      {view === "accounts" && user.role === "admin" && <AccountsView />}
      {view === "sites" && user.role === "admin" && <SitesView />}
      {view === "audit" && user.role === "admin" && <AuditLogView />}
      {view === "tunnelKeys" && user.role === "admin" && <TunnelKeyRequestsPanel />}
      {view === "settings" && <SettingsView user={user} serverUrl={serverUrl} />}
    </AppShell>
  );
}
