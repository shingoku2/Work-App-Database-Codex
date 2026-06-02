import type { MinerStatus } from "@/types/db";

const styles: Record<MinerStatus, string> = {
  "In Service": "border-blue-400/40 bg-blue-400/15 text-blue-200",
  "Under Repair": "border-amber-400/40 bg-amber-400/15 text-amber-200",
  RMA: "border-red-400/40 bg-red-400/15 text-red-200",
  Retired: "border-zinc-400/30 bg-zinc-400/10 text-zinc-300",
  Spare: "border-teal-400/40 bg-teal-400/15 text-teal-200",
};

export function StatusBadge({ value }: { value: MinerStatus }) {
  return (
    <span className={`inline-flex rounded-md border px-2 py-1 text-xs font-medium ${styles[value]}`}>
      {value}
    </span>
  );
}
