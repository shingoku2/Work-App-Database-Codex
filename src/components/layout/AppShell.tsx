import { Boxes, Building2, ClipboardList, Gauge, KeyRound, LayoutDashboard, Settings, Users, Wrench } from "lucide-react";
import type { ReactNode } from "react";
import type { User } from "@/types/db";

export type ViewKey = "dashboard" | "units" | "inventory" | "accounts" | "sites" | "audit" | "tunnelKeys" | "settings";

const baseNav = [
  { key: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { key: "units", label: "Units", icon: Gauge },
  { key: "inventory", label: "Inventory", icon: Boxes },
] satisfies Array<{ key: ViewKey; label: string; icon: typeof Wrench }>;

export function AppShell({
  active,
  onNavigate,
  user,
  serverUrl,
  children,
}: {
  active: ViewKey;
  onNavigate: (view: ViewKey) => void;
  user: User;
  serverUrl: string;
  children: ReactNode;
}) {
  const nav = [
    ...baseNav,
    ...(user.role === "admin"
      ? [
          { key: "accounts" as const, label: "Accounts", icon: Users },
          { key: "sites" as const, label: "Sites", icon: Building2 },
          { key: "audit" as const, label: "Audit Log", icon: ClipboardList },
          { key: "tunnelKeys" as const, label: "Tunnel Keys", icon: KeyRound },
        ]
      : []),
    { key: "settings" as const, label: "Settings", icon: Settings },
  ];
  return (
    <div className="flex min-h-screen bg-[#101821] text-slate-100">
      <aside className="w-64 border-r border-white/10 bg-[#0b1219]">
        <div className="flex h-16 items-center gap-3 border-b border-white/10 px-5">
          <div className="grid h-9 w-9 place-items-center rounded-md bg-primary/20 text-primary">
            <Wrench size={20} />
          </div>
          <div>
            <div className="font-semibold">Antminer Fleet</div>
            <div className="text-xs text-slate-500">Central inventory</div>
          </div>
        </div>
        <nav className="space-y-1 p-3">
          {nav.map((item) => {
            const Icon = item.icon;
            const selected = active === item.key;
            return (
              <button
                key={item.key}
                type="button"
                onClick={() => onNavigate(item.key)}
                className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm transition ${
                  selected ? "bg-primary/15 text-sky-100" : "text-slate-400 hover:bg-white/5 hover:text-slate-100"
                }`}
              >
                <Icon size={18} />
                {item.label}
              </button>
            );
          })}
        </nav>
      </aside>
      <main className="flex-1 overflow-auto">
        <header className="flex h-16 items-center justify-between border-b border-white/10 px-8">
          <div>
            <h1 className="text-lg font-semibold">ASIC Asset Management</h1>
            <p className="text-xs text-slate-500">{serverUrl}</p>
          </div>
          <div className="rounded-md border border-emerald-400/30 bg-emerald-400/10 px-3 py-1 text-xs text-emerald-200">
            {user.display_name} · Connected
          </div>
        </header>
        <div className="p-8">{children}</div>
      </main>
    </div>
  );
}
