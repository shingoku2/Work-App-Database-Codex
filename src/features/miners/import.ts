import type { MinerModel, MinerStatus } from "@/types/db";
import type { CreateMinerInput } from "./minerApi";

export type ImportRow = Record<string, unknown>;

export const MAX_IMPORT_BYTES = 25 * 1024 * 1024;
export const XLSX_MAGIC: readonly [number, number] = [0x50, 0x4b];
export const SPREADSHEET_PARSE_ERROR =
  "Could not read the spreadsheet. The file may be corrupt or in an unsupported format.";

export function normalizeKey(key: string): string {
  return key.toLowerCase().replace(/[^a-z0-9]/g, "");
}

export function buildKeyMap(row: ImportRow): Map<string, string> {
  const keyMap = new Map<string, string>();
  for (const originalKey of Object.keys(row)) {
    keyMap.set(normalizeKey(originalKey), originalKey);
  }
  return keyMap;
}

export function value(row: ImportRow, key: string, keyMap: Map<string, string>): string {
  const matchingKey = keyMap.get(normalizeKey(key));
  const raw = matchingKey !== undefined ? row[matchingKey] : "";
  if (raw instanceof Date) {
    return raw.toISOString().slice(0, 10);
  }
  return raw == null ? "" : String(raw).trim();
}

export function nullable(input: string): string | null {
  return input || null;
}

export function normalizeModel(minerType: string): MinerModel {
  const normalized = minerType.toLowerCase();

  if (normalized.includes("s21 xp") || normalized.includes("s21xp")) return "S21 XP";
  if (normalized.includes("s21 pro") || normalized.includes("s21pro")) return "S21 Pro";
  if (normalized.includes("s21+")) return "S21+";
  return "S21";
}

export function normalizeStatus(state: string, status: string): MinerStatus {
  const text = `${state} ${status}`.toLowerCase();

  if (text.includes("rma")) return "RMA";
  if (text.includes("retired")) return "Retired";
  if (text.includes("spare")) return "Spare";
  if (text.includes("fail") || text.includes("repair") || text.includes("offline")) return "Under Repair";
  return "In Service";
}

export function normalizeDate(input: string): string | null {
  const isoMatch = input.match(/\d{4}-\d{2}-\d{2}/);
  if (isoMatch) return isoMatch[0];

  const slashMatch = input.match(/^(\d{1,2})\/(\d{1,2})\/(\d{2,4})$/);
  if (!slashMatch) return null;

  const [, mm, dd, yyyy] = slashMatch;
  const month = Number(mm);
  const day = Number(dd);
  if (month < 1 || month > 12 || day < 1 || day > 31) return null;
  const fullYear = yyyy.length === 2 ? `20${yyyy}` : yyyy;
  return `${fullYear}-${mm.padStart(2, "0")}-${dd.padStart(2, "0")}`;
}

export function buildLocation(row: ImportRow, keyMap: Map<string, string>): string | null {
  const pickaxe = value(row, "pickaxe", keyMap);
  const rackGroup = value(row, "miner_rack_group", keyMap);
  const rack = value(row, "miner_rack", keyMap);
  const minerRow = value(row, "miner_row", keyMap);
  const minerIndex = value(row, "miner_index", keyMap);
  const slot = [minerRow && `Row ${minerRow}`, minerIndex && `Index ${minerIndex}`].filter(Boolean).join(" ");

  return nullable([pickaxe, rackGroup, rack, slot].filter(Boolean).join(" / "));
}

export function buildNotes(row: ImportRow, keyMap: Map<string, string>): string | null {
  const noteParts = [
    ["Miner ID", value(row, "miner_id", keyMap)],
    ["Name", value(row, "miner_name", keyMap)],
    ["Raw status", value(row, "status", keyMap)],
    ["Tags", value(row, "miner_tags", keyMap)],
    ["PSU serial", value(row, "psu_serial", keyMap)],
    ["Control board", value(row, "miner_control_board", keyMap)],
    ["Wattage", value(row, "wattage", keyMap)],
    ["Hash rate", value(row, "hash_rate", keyMap)],
    ["Max temp", value(row, "max_temp", keyMap)],
    ["Last update", value(row, "last_update", keyMap)],
  ]
    .filter(([, part]) => part)
    .map(([label, part]) => `${label}: ${part}`);

  return nullable(noteParts.join("\n"));
}

export function mapImportRow(row: ImportRow): CreateMinerInput | null {
  const keyMap = buildKeyMap(row);

  const serial = value(row, "miner_serial", keyMap) || value(row, "serial", keyMap);

  if (!serial) {
    return null;
  }

  const statusValue = value(row, "status", keyMap);
  const rawStatus = value(row, "miner_state", keyMap) || statusValue;
  const location = buildLocation(row, keyMap);
  const notes = buildNotes(row, keyMap);

  return {
    serial,
    model: normalizeModel(value(row, "miner_type", keyMap)),
    firmware: nullable(value(row, "firmware_version", keyMap)),
    client_name: nullable(value(row, "client_name", keyMap)),
    miner_type: nullable(value(row, "miner_type", keyMap)),
    ip_address: nullable(value(row, "miner_ip", keyMap)),
    mac_address: nullable(value(row, "miner_mac", keyMap)),
    pickaxe: nullable(value(row, "pickaxe", keyMap)),
    miner_state: nullable(value(row, "miner_state", keyMap)),
    miner_row: nullable(value(row, "miner_row", keyMap)),
    miner_index: nullable(value(row, "miner_index", keyMap)),
    miner_rack: nullable(value(row, "miner_rack", keyMap)),
    miner_rack_group: nullable(value(row, "miner_rack_group", keyMap)),
    location,
    status: normalizeStatus(rawStatus, statusValue),
    acquired_date: normalizeDate(value(row, "miner_created_date", keyMap)),
    notes,
  };
}

export function rowsToObjects(rows: unknown[][]): ImportRow[] {
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

export function parseDelimited(text: string, delimiter: string): string[][] {
  // Expects UTF-8 input. `file.text()` decodes as UTF-8 in the Tauri WebView;
  // files exported as UTF-16-LE or Windows-1252 will be mis-parsed (mojibake).
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

export async function hasExcelMagicBytes(blob: Blob): Promise<boolean> {
  try {
    const head = await blob.slice(0, 2).arrayBuffer();
    const bytes = new Uint8Array(head);
    return bytes.length >= 2 && bytes[0] === XLSX_MAGIC[0] && bytes[1] === XLSX_MAGIC[1];
  } catch {
    return false;
  }
}

export async function readSpreadsheetRows(file: File): Promise<ImportRow[]> {
  const extension = file.name.split(".").pop()?.toLowerCase();

  if (extension === "csv" || extension === "tsv") {
    const delimiter = extension === "tsv" ? "\t" : ",";
    try {
      return rowsToObjects(parseDelimited(await file.text(), delimiter));
    } catch {
      throw new Error(SPREADSHEET_PARSE_ERROR);
    }
  }

  if (!(await hasExcelMagicBytes(file))) {
    throw new Error(SPREADSHEET_PARSE_ERROR);
  }

  try {
    const { readSheet } = await import("read-excel-file/browser");
    return rowsToObjects(await readSheet(file));
  } catch {
    throw new Error(SPREADSHEET_PARSE_ERROR);
  }
}

export function formatImportMessage(result: {
  imported: number;
  updated: number;
  skipped: number;
  conflicts: string[];
}): string {
  const parts = [`${result.imported} added`];
  if (result.skipped > 0) {
    parts.push(`${result.skipped} skipped to preserve existing records`);
  }
  if (result.conflicts.length > 0) {
    parts.push(`conflicts: ${result.conflicts.join(", ")}`);
  }
  return `Imported miners: ${parts.join(", ")}.`;
}
