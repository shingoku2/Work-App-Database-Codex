import { useState } from "react";
import { AppShell, type ViewKey } from "@/components/layout/AppShell";
import { DashboardView } from "@/features/dashboard/DashboardView";
import { InventoryView } from "@/features/inventory/InventoryView";
import { MinersView } from "@/features/miners/MinersView";

export function App() {
  const [view, setView] = useState<ViewKey>("dashboard");

  return (
    <AppShell active={view} onNavigate={setView}>
      {view === "dashboard" && <DashboardView />}
      {view === "units" && <MinersView />}
      {view === "inventory" && <InventoryView />}
    </AppShell>
  );
}
