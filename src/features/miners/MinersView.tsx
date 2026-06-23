import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ColumnDef } from "@tanstack/react-table";
import { ArrowLeft, Plus, Upload } from "lucide-react";
import { useState } from "react";
import { DataTable } from "@/components/ui/DataTable";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass, textareaClass } from "@/components/ui/Panel";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { invalidateFleetData } from "@/lib/queryInvalidation";
import type { Miner, MinerModel, MinerStatus } from "@/types/db";
import { createMiner, deleteMiner, importMiners, listMiners, updateMiner, type CreateMinerInput } from "./minerApi";
import {
  MAX_IMPORT_BYTES,
  formatImportMessage,
  mapImportRow,
  readSpreadsheetRows,
} from "./import";

const models: MinerModel[] = ["S21", "S21+", "S21 Pro", "S21 XP"];
const statuses: MinerStatus[] = ["In Service", "Under Repair", "RMA", "Retired", "Spare"];

const emptyForm: CreateMinerInput = {
  serial: "",
  model: "S21",
  firmware: null,
  client_name: null,
  miner_type: null,
  ip_address: null,
  mac_address: null,
  pickaxe: null,
  miner_state: null,
  miner_row: null,
  miner_index: null,
  miner_rack: null,
  miner_rack_group: null,
  location: null,
  status: "In Service",
  acquired_date: null,
  notes: null,
};

type MinerViewMode = { type: "list" } | { type: "new" } | { type: "detail"; id: number };

export function MinersView({ canImport }: { canImport: boolean }) {
  const queryClient = useQueryClient();
  const [view, setView] = useState<MinerViewMode>({ type: "list" });
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const [importIsError, setImportIsError] = useState(false);
  const { data = [], error, isLoading } = useQuery({ queryKey: ["miners"], queryFn: listMiners });

  const selectedMiner = view.type === "detail" ? data.find((miner) => miner.id === view.id) ?? null : null;

  const deleteMutation = useMutation({
    mutationFn: ({ id, version }: Pick<Miner, "id" | "version">) => deleteMiner(id, version),
    onSuccess: async () => {
      setView({ type: "list" });
      await invalidateFleetData(queryClient, "miners");
    },
  });

  const importMutation = useMutation({
    mutationFn: importMiners,
    onSuccess: async (result) => {
      setImportIsError(false);
      setImportMessage(formatImportMessage(result));
      await invalidateFleetData(queryClient, "miners");
    },
  });

  const columns: ColumnDef<Miner>[] = [
    { accessorKey: "serial", header: "Serial" },
    { accessorKey: "client_name", header: "Client", cell: ({ row }) => row.original.client_name || "-" },
    { accessorKey: "model", header: "Model" },
    { accessorKey: "firmware", header: "Firmware", cell: ({ row }) => row.original.firmware || "-" },
    { accessorKey: "ip_address", header: "IP", cell: ({ row }) => row.original.ip_address || "-" },
    { accessorKey: "mac_address", header: "MAC", cell: ({ row }) => row.original.mac_address || "-" },
    { accessorKey: "miner_rack", header: "Rack", cell: ({ row }) => row.original.miner_rack || "-" },
    { accessorKey: "location", header: "Location", cell: ({ row }) => row.original.location || "-" },
    { accessorKey: "status", header: "Status", cell: ({ row }) => <StatusBadge value={row.original.status} /> },
    {
      id: "actions",
      header: "",
      cell: ({ row }) => (
        <button
          type="button"
          className={secondaryButtonClass}
          disabled={deleteMutation.isPending}
          onClick={(event) => {
            event.stopPropagation();
            if (window.confirm(`Delete miner "${row.original.serial}"?`)) {
              deleteMutation.mutate({ id: row.original.id, version: row.original.version });
            }
          }}
        >
          Delete
        </button>
      ),
    },
  ];

  async function handleImport(file: File | null) {
    if (!file) return;

    setImportMessage(null);

    if (file.size > MAX_IMPORT_BYTES) {
      throw new Error(`File is too large. Imports are limited to ${MAX_IMPORT_BYTES / (1024 * 1024)} MB.`);
    }

    const rows = await readSpreadsheetRows(file);
    const miners = rows.map(mapImportRow).filter((miner): miner is CreateMinerInput => Boolean(miner));

    if (miners.length === 0) {
      throw new Error("No importable miners were found. Make sure the file includes miner_serial values.");
    }

    importMutation.mutate(miners);
  }

  if (view.type === "new") {
    return (
      <MinerDetailView
        key="new"
        onBack={() => setView({ type: "list" })}
        title="Add Unit"
      />
    );
  }

  if (view.type === "detail") {
    if (!selectedMiner && isLoading) {
      return <div className="text-slate-400">Loading unit...</div>;
    }

    if (!selectedMiner) {
      return (
        <section className="space-y-4">
          <button type="button" className={secondaryButtonClass} onClick={() => setView({ type: "list" })}>
            Back to units
          </button>
          <div className="rounded-lg border border-amber-400/30 bg-amber-400/10 p-5 text-amber-100">
            That miner is no longer available.
          </div>
        </section>
      );
    }

    return (
      <MinerDetailView
        key={selectedMiner.id}
        miner={selectedMiner}
        title={selectedMiner.serial}
        onBack={() => setView({ type: "list" })}
      />
    );
  }

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-2xl font-semibold">Unit Registry</h2>
          <p className="text-sm text-slate-500">Track serials, locations, firmware, and lifecycle status.</p>
        </div>
        <button className={`${primaryButtonClass} inline-flex items-center gap-2`} onClick={() => setView({ type: "new" })}>
          <Plus size={16} />
          Add Unit
        </button>
      </div>

      {canImport && <Panel title="Import Miners">
        <div className="flex flex-wrap items-center gap-3">
          <label className={`${primaryButtonClass} inline-flex cursor-pointer items-center gap-2`}>
            <Upload size={16} />
            Import CSV or Spreadsheet
            <input
              className="hidden"
              type="file"
              accept=".csv,.tsv,.xlsx"
              disabled={importMutation.isPending}
              onChange={(event) => {
                handleImport(event.target.files?.[0] ?? null).catch((importError) => {
                  importMutation.reset();
                  setImportIsError(true);
                  setImportMessage(importError instanceof Error && importError.message ? importError.message : "Import failed.");
                });
                event.currentTarget.value = "";
              }}
            />
          </label>
          <span className="text-sm text-slate-400">
            Bulk import client_name, miner_serial, miner_type, miner_ip, miner_mac, firmware_version, pickaxe, state, row, index, rack, and rack group.
          </span>
          {importMutation.isPending && <span className="text-sm text-slate-300">Importing...</span>}
          {importMessage && (
            <span className={`text-sm ${importIsError ? "text-red-300" : "text-emerald-300"}`}>
              {importMessage}
            </span>
          )}
          {importMutation.error && <span className="text-sm text-red-300">{String(importMutation.error)}</span>}
        </div>
      </Panel>}

      {isLoading ? (
        <div className="text-slate-400">Loading units...</div>
      ) : (
        <DataTable columns={columns} data={data} searchPlaceholder="Filter units" onRowClick={(miner) => setView({ type: "detail", id: miner.id })} />
      )}

      {(deleteMutation.error || error) && (
        <div className="text-sm text-red-300">{String(deleteMutation.error || error)}</div>
      )}
    </section>
  );
}

function MinerDetailView({
  miner,
  title,
  onBack,
}: {
  miner?: Miner;
  title: string;
  onBack: () => void;
}) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<CreateMinerInput>(() => miner ? minerToForm(miner) : emptyForm);

  const saveMutation = useMutation({
    mutationFn: async () => {
      if (miner) {
        await updateMiner({ ...form, id: miner.id, version: miner.version, site_id: miner.site_id, site_name: miner.site_name });
      } else {
        await createMiner(form);
      }
    },
    onSuccess: async () => {
      await invalidateFleetData(queryClient, "miners");
      onBack();
    },
  });

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <button type="button" className={`${secondaryButtonClass} mb-4 inline-flex items-center gap-2`} onClick={onBack}>
            <ArrowLeft size={16} />
            Back to units
          </button>
          <h2 className="text-2xl font-semibold">{title}</h2>
          <p className="text-sm text-slate-500">Review and edit every tracked data point for this miner.</p>
        </div>
        {miner && <StatusBadge value={form.status} />}
      </div>

      <Panel title="Core Information">
        <form
          className="space-y-5"
          onSubmit={(event) => {
            event.preventDefault();
            saveMutation.mutate();
          }}
        >
          <div className="grid grid-cols-4 gap-3">
            <input className={fieldClass} required placeholder="Serial number" value={form.serial} onChange={(event) => setForm({ ...form, serial: event.target.value })} />
            <input className={fieldClass} placeholder="Client" value={form.client_name ?? ""} onChange={(event) => setForm({ ...form, client_name: event.target.value || null })} />
            <select className={fieldClass} value={form.model} onChange={(event) => setForm({ ...form, model: event.target.value as MinerModel })}>
              {models.map((model) => <option key={model}>{model}</option>)}
            </select>
            <input className={fieldClass} placeholder="Miner type" value={form.miner_type ?? ""} onChange={(event) => setForm({ ...form, miner_type: event.target.value || null })} />
            <input className={fieldClass} placeholder="Firmware" value={form.firmware ?? ""} onChange={(event) => setForm({ ...form, firmware: event.target.value || null })} />
            <input className={fieldClass} placeholder="IP address" value={form.ip_address ?? ""} onChange={(event) => setForm({ ...form, ip_address: event.target.value || null })} />
            <input className={fieldClass} placeholder="MAC address" value={form.mac_address ?? ""} onChange={(event) => setForm({ ...form, mac_address: event.target.value || null })} />
            <select className={fieldClass} value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as MinerStatus })}>
              {statuses.map((status) => <option key={status}>{status}</option>)}
            </select>
          </div>

          <div className="grid grid-cols-4 gap-3">
            <input className={fieldClass} placeholder="Pickaxe / facility" value={form.pickaxe ?? ""} onChange={(event) => setForm({ ...form, pickaxe: event.target.value || null })} />
            <input className={fieldClass} placeholder="Miner state" value={form.miner_state ?? ""} onChange={(event) => setForm({ ...form, miner_state: event.target.value || null })} />
            <input className={fieldClass} placeholder="Rack group" value={form.miner_rack_group ?? ""} onChange={(event) => setForm({ ...form, miner_rack_group: event.target.value || null })} />
            <input className={fieldClass} placeholder="Rack" value={form.miner_rack ?? ""} onChange={(event) => setForm({ ...form, miner_rack: event.target.value || null })} />
            <input className={fieldClass} placeholder="Row" value={form.miner_row ?? ""} onChange={(event) => setForm({ ...form, miner_row: event.target.value || null })} />
            <input className={fieldClass} placeholder="Index" value={form.miner_index ?? ""} onChange={(event) => setForm({ ...form, miner_index: event.target.value || null })} />
            <input className={fieldClass} placeholder="Location / slot" value={form.location ?? ""} onChange={(event) => setForm({ ...form, location: event.target.value || null })} />
            <input className={fieldClass} type="date" value={form.acquired_date ?? ""} onChange={(event) => setForm({ ...form, acquired_date: event.target.value || null })} />
          </div>

          <textarea className={`${textareaClass} w-full`} placeholder="Notes" value={form.notes ?? ""} onChange={(event) => setForm({ ...form, notes: event.target.value || null })} />

          <div className="flex items-center gap-2">
            <button className={primaryButtonClass} disabled={saveMutation.isPending}>
              {miner ? "Save Unit" : "Create Unit"}
            </button>
            <button type="button" className={secondaryButtonClass} onClick={onBack}>
              Cancel
            </button>
            {saveMutation.error && <span className="text-sm text-red-300">{String(saveMutation.error)}</span>}
          </div>
        </form>
      </Panel>
    </section>
  );
}

function minerToForm(miner: Miner): CreateMinerInput {
  return {
    serial: miner.serial,
    model: miner.model,
    firmware: miner.firmware,
    client_name: miner.client_name,
    miner_type: miner.miner_type,
    ip_address: miner.ip_address,
    mac_address: miner.mac_address,
    pickaxe: miner.pickaxe,
    miner_state: miner.miner_state,
    miner_row: miner.miner_row,
    miner_index: miner.miner_index,
    miner_rack: miner.miner_rack,
    miner_rack_group: miner.miner_rack_group,
    location: miner.location,
    status: miner.status,
    acquired_date: miner.acquired_date,
    notes: miner.notes,
  };
}
