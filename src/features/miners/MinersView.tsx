import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ColumnDef } from "@tanstack/react-table";
import { ArrowLeft, Plus, Upload } from "lucide-react";
import { useState } from "react";
import { DataTable } from "@/components/ui/DataTable";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass, textareaClass } from "@/components/ui/Panel";
import { StatusBadge } from "@/components/ui/StatusBadge";
import type { Miner, MinerModel, MinerStatus } from "@/types/db";
import { createMiner, deleteMiner, importMiners, listMiners, updateMiner, type CreateMinerInput } from "./minerApi";

const models: MinerModel[] = ["S21", "S21+", "S21 Pro", "S21 XP"];
const statuses: MinerStatus[] = ["In Service", "Under Repair", "RMA", "Retired", "Spare"];

const emptyForm: CreateMinerInput = {
  serial: "",
  model: "S21",
  firmware: "",
  client_name: "",
  miner_type: "",
  ip_address: "",
  mac_address: "",
  pickaxe: "",
  miner_state: "",
  miner_row: "",
  miner_index: "",
  miner_rack: "",
  miner_rack_group: "",
  location: "",
  status: "In Service",
  acquired_date: "",
  notes: "",
};

type ImportRow = Record<string, unknown>;
type MinerViewMode = { type: "list" } | { type: "new" } | { type: "detail"; id: number };

export function MinersView() {
  const queryClient = useQueryClient();
  const [view, setView] = useState<MinerViewMode>({ type: "list" });
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const { data = [], error, isLoading } = useQuery({ queryKey: ["miners"], queryFn: listMiners });

  const selectedMiner = view.type === "detail" ? data.find((miner) => miner.id === view.id) ?? null : null;

  const deleteMutation = useMutation({
    mutationFn: deleteMiner,
    onSuccess: async () => {
      setView({ type: "list" });
      await queryClient.invalidateQueries({ queryKey: ["miners"] });
      await queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });

  const importMutation = useMutation({
    mutationFn: importMiners,
    onSuccess: async (result) => {
      setImportMessage(`Imported ${result.imported} miner${result.imported === 1 ? "" : "s"}.`);
      await queryClient.invalidateQueries({ queryKey: ["miners"] });
      await queryClient.invalidateQueries({ queryKey: ["dashboard"] });
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
            deleteMutation.mutate(row.original.id);
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

      <Panel title="Import Miners">
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
                  setImportMessage(null);
                  importMutation.reset();
                  console.error(importError);
                  alert(String(importError));
                });
                event.currentTarget.value = "";
              }}
            />
          </label>
          <span className="text-sm text-slate-400">
            Bulk import client_name, miner_serial, miner_type, miner_ip, miner_mac, firmware_version, pickaxe, state, row, index, rack, and rack group.
          </span>
          {importMutation.isPending && <span className="text-sm text-slate-300">Importing...</span>}
          {importMessage && <span className="text-sm text-emerald-300">{importMessage}</span>}
          {importMutation.error && <span className="text-sm text-red-300">{String(importMutation.error)}</span>}
        </div>
      </Panel>

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
        await updateMiner({ ...form, id: miner.id });
      } else {
        await createMiner(form);
      }
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["miners"] });
      await queryClient.invalidateQueries({ queryKey: ["dashboard"] });
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
            <input className={fieldClass} placeholder="Client" value={form.client_name ?? ""} onChange={(event) => setForm({ ...form, client_name: event.target.value })} />
            <select className={fieldClass} value={form.model} onChange={(event) => setForm({ ...form, model: event.target.value as MinerModel })}>
              {models.map((model) => <option key={model}>{model}</option>)}
            </select>
            <input className={fieldClass} placeholder="Miner type" value={form.miner_type ?? ""} onChange={(event) => setForm({ ...form, miner_type: event.target.value })} />
            <input className={fieldClass} placeholder="Firmware" value={form.firmware ?? ""} onChange={(event) => setForm({ ...form, firmware: event.target.value })} />
            <input className={fieldClass} placeholder="IP address" value={form.ip_address ?? ""} onChange={(event) => setForm({ ...form, ip_address: event.target.value })} />
            <input className={fieldClass} placeholder="MAC address" value={form.mac_address ?? ""} onChange={(event) => setForm({ ...form, mac_address: event.target.value })} />
            <select className={fieldClass} value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as MinerStatus })}>
              {statuses.map((status) => <option key={status}>{status}</option>)}
            </select>
          </div>

          <div className="grid grid-cols-4 gap-3">
            <input className={fieldClass} placeholder="Pickaxe / facility" value={form.pickaxe ?? ""} onChange={(event) => setForm({ ...form, pickaxe: event.target.value })} />
            <input className={fieldClass} placeholder="Miner state" value={form.miner_state ?? ""} onChange={(event) => setForm({ ...form, miner_state: event.target.value })} />
            <input className={fieldClass} placeholder="Rack group" value={form.miner_rack_group ?? ""} onChange={(event) => setForm({ ...form, miner_rack_group: event.target.value })} />
            <input className={fieldClass} placeholder="Rack" value={form.miner_rack ?? ""} onChange={(event) => setForm({ ...form, miner_rack: event.target.value })} />
            <input className={fieldClass} placeholder="Row" value={form.miner_row ?? ""} onChange={(event) => setForm({ ...form, miner_row: event.target.value })} />
            <input className={fieldClass} placeholder="Index" value={form.miner_index ?? ""} onChange={(event) => setForm({ ...form, miner_index: event.target.value })} />
            <input className={fieldClass} placeholder="Location / slot" value={form.location ?? ""} onChange={(event) => setForm({ ...form, location: event.target.value })} />
            <input className={fieldClass} type="date" value={form.acquired_date ?? ""} onChange={(event) => setForm({ ...form, acquired_date: event.target.value })} />
          </div>

          <textarea className={`${textareaClass} w-full`} placeholder="Notes" value={form.notes ?? ""} onChange={(event) => setForm({ ...form, notes: event.target.value })} />

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
    firmware: miner.firmware ?? "",
    client_name: miner.client_name ?? "",
    miner_type: miner.miner_type ?? "",
    ip_address: miner.ip_address ?? "",
    mac_address: miner.mac_address ?? "",
    pickaxe: miner.pickaxe ?? "",
    miner_state: miner.miner_state ?? "",
    miner_row: miner.miner_row ?? "",
    miner_index: miner.miner_index ?? "",
    miner_rack: miner.miner_rack ?? "",
    miner_rack_group: miner.miner_rack_group ?? "",
    location: miner.location ?? "",
    status: miner.status,
    acquired_date: miner.acquired_date ?? "",
    notes: miner.notes ?? "",
  };
}

async function readSpreadsheetRows(file: File): Promise<ImportRow[]> {
  const extension = file.name.split(".").pop()?.toLowerCase();

  if (extension === "csv" || extension === "tsv") {
    const delimiter = extension === "tsv" ? "\t" : ",";
    return rowsToObjects(parseDelimited(await file.text(), delimiter));
  }

  const { readSheet } = await import("read-excel-file/browser");
  return rowsToObjects(await readSheet(file));
}

function rowsToObjects(rows: unknown[][]): ImportRow[] {
  const [headers, ...records] = rows;

  if (!headers) {
    return [];
  }

  return records.map((record) =>
    headers.reduce<ImportRow>((mapped, header, index) => {
      const key = header == null ? "" : String(header).trim();
      if (key) {
        mapped[key] = record[index] ?? "";
      }
      return mapped;
    }, {}),
  );
}

function parseDelimited(text: string, delimiter: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];

    if (char === "\"") {
      if (inQuotes && next === "\"") {
        field += "\"";
        index += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }

    if (!inQuotes && char === delimiter) {
      row.push(field);
      field = "";
      continue;
    }

    if (!inQuotes && (char === "\n" || char === "\r")) {
      if (char === "\r" && next === "\n") {
        index += 1;
      }
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
      continue;
    }

    field += char;
  }

  row.push(field);
  if (row.some((value) => value.trim())) {
    rows.push(row);
  }

  return rows;
}

function mapImportRow(row: ImportRow): CreateMinerInput | null {
  const serial = value(row, "miner_serial") || value(row, "serial");

  if (!serial) {
    return null;
  }

  const rawStatus = value(row, "miner_state") || value(row, "status");
  const location = buildLocation(row);
  const notes = buildNotes(row);

  return {
    serial,
    model: normalizeModel(value(row, "miner_type")),
    firmware: nullable(value(row, "firmware_version")),
    client_name: nullable(value(row, "client_name")),
    miner_type: nullable(value(row, "miner_type")),
    ip_address: nullable(value(row, "miner_ip")),
    mac_address: nullable(value(row, "miner_mac")),
    pickaxe: nullable(value(row, "pickaxe")),
    miner_state: nullable(value(row, "miner_state")),
    miner_row: nullable(value(row, "miner_row")),
    miner_index: nullable(value(row, "miner_index")),
    miner_rack: nullable(value(row, "miner_rack")),
    miner_rack_group: nullable(value(row, "miner_rack_group")),
    location,
    status: normalizeStatus(rawStatus, value(row, "status")),
    acquired_date: normalizeDate(value(row, "miner_created_date")),
    notes,
  };
}

function value(row: ImportRow, key: string): string {
  const target = normalizeKey(key);
  const matchingKey = Object.keys(row).find((candidate) => normalizeKey(candidate) === target);
  const raw = matchingKey ? row[matchingKey] : "";
  if (raw instanceof Date) {
    return raw.toISOString().slice(0, 10);
  }
  return raw == null ? "" : String(raw).trim();
}

function nullable(input: string): string | null {
  return input || null;
}

function normalizeKey(key: string): string {
  return key.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function normalizeModel(minerType: string): MinerModel {
  const normalized = minerType.toLowerCase();

  if (normalized.includes("s21 xp") || normalized.includes("s21xp")) return "S21 XP";
  if (normalized.includes("s21 pro") || normalized.includes("s21pro")) return "S21 Pro";
  if (normalized.includes("s21+")) return "S21+";
  return "S21";
}

function normalizeStatus(state: string, status: string): MinerStatus {
  const text = `${state} ${status}`.toLowerCase();

  if (text.includes("rma")) return "RMA";
  if (text.includes("retired")) return "Retired";
  if (text.includes("spare")) return "Spare";
  if (text.includes("fail") || text.includes("repair") || text.includes("offline")) return "Under Repair";
  return "In Service";
}

function normalizeDate(input: string): string | null {
  const isoMatch = input.match(/\d{4}-\d{2}-\d{2}/);
  if (isoMatch) return isoMatch[0];

  const slashMatch = input.match(/^(\d{1,2})\/(\d{1,2})\/(\d{2,4})$/);
  if (!slashMatch) return null;

  const [, month, day, year] = slashMatch;
  const fullYear = year.length === 2 ? `20${year}` : year;
  return `${fullYear}-${month.padStart(2, "0")}-${day.padStart(2, "0")}`;
}

function buildLocation(row: ImportRow): string | null {
  const pickaxe = value(row, "pickaxe");
  const rackGroup = value(row, "miner_rack_group");
  const rack = value(row, "miner_rack");
  const minerRow = value(row, "miner_row");
  const minerIndex = value(row, "miner_index");
  const slot = [minerRow && `Row ${minerRow}`, minerIndex && `Index ${minerIndex}`].filter(Boolean).join(" ");

  return nullable([pickaxe, rackGroup, rack, slot].filter(Boolean).join(" / "));
}

function buildNotes(row: ImportRow): string | null {
  const noteParts = [
    ["Miner ID", value(row, "miner_id")],
    ["Name", value(row, "miner_name")],
    ["Raw status", value(row, "status")],
    ["Tags", value(row, "miner_tags")],
    ["PSU serial", value(row, "psu_serial")],
    ["Control board", value(row, "miner_control_board")],
    ["Wattage", value(row, "wattage")],
    ["Hash rate", value(row, "hash_rate")],
    ["Max temp", value(row, "max_temp")],
    ["Last update", value(row, "last_update")],
  ]
    .filter(([, part]) => part)
    .map(([label, part]) => `${label}: ${part}`);

  return nullable(noteParts.join("\n"));
}
