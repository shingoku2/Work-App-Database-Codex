import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, Boxes, Gauge, PackageSearch } from "lucide-react";
import { getDashboardSummary } from "./dashboardApi";
import { StatusBadge } from "@/components/ui/StatusBadge";

export function DashboardView() {
  const { data, error, isLoading } = useQuery({ queryKey: ["dashboard"], queryFn: getDashboardSummary });

  if (isLoading) return <div className="text-slate-400">Loading dashboard...</div>;

  if (error) {
    return (
      <div className="rounded-lg border border-amber-400/30 bg-amber-400/10 p-5 text-amber-100">
        The desktop database is not available in this browser session. Run the app with <code>npm run tauri:dev</code> to use local SQLite records.
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="space-y-6">
      <section className="grid grid-cols-3 gap-4">
        <Metric title="Tracked units" value={data.unit_count} icon={<Gauge size={20} />} />
        <Metric title="Low stock parts" value={data.low_stock_count} icon={<AlertTriangle size={20} />} />
        <Metric title="Inventory SKUs" value={data.part_count} icon={<Boxes size={20} />} />
      </section>
      <section className="grid grid-cols-[1fr_1.3fr] gap-4">
        <div className="rounded-lg border border-white/10 bg-card p-5">
          <h2 className="mb-4 font-semibold">Units by Status</h2>
          <div className="space-y-3">
            {data.units_by_status.length === 0 && <div className="text-sm text-slate-500">No units recorded.</div>}
            {data.units_by_status.map((item) => (
              <div key={item.status} className="flex items-center justify-between">
                <StatusBadge value={item.status} />
                <span className="text-2xl font-semibold">{item.count}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="rounded-lg border border-white/10 bg-card p-5">
          <h2 className="mb-4 flex items-center gap-2 font-semibold">
            <PackageSearch size={18} /> Low Stock Parts
          </h2>
          <div className="space-y-3">
            {data.low_stock_parts.length === 0 && <div className="text-sm text-slate-500">No low-stock parts.</div>}
            {data.low_stock_parts.map((part) => (
              <div key={part.sku} className="flex items-center justify-between rounded-md border border-white/10 p-3">
                <div>
                  <div className="font-medium">{part.name}</div>
                  <div className="text-xs text-slate-500">{part.sku}</div>
                </div>
                <div className="text-right">
                  <div className="text-lg font-semibold">{part.qty_on_hand}</div>
                  <div className="text-xs text-slate-500">threshold {part.reorder_threshold}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}

function Metric({ title, value, icon }: { title: string; value: number; icon: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-white/10 bg-card p-5">
      <div className="flex items-center justify-between text-slate-400">
        <span className="text-sm">{title}</span>
        {icon}
      </div>
      <div className="mt-4 text-4xl font-semibold">{value}</div>
    </div>
  );
}
