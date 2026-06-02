import { describe, expect, it } from "vitest";
import {
  MAX_IMPORT_BYTES,
  SPREADSHEET_PARSE_ERROR,
  XLSX_MAGIC,
  buildKeyMap,
  buildLocation,
  buildNotes,
  formatImportMessage,
  hasExcelMagicBytes,
  mapImportRow,
  normalizeDate,
  normalizeKey,
  normalizeModel,
  normalizeStatus,
  nullable,
  parseDelimited,
  readSpreadsheetRows,
  rowsToObjects,
  value,
} from "@/features/miners/import";
import type { ImportRow } from "@/features/miners/import";
import { sampleImportResult, sampleImportResultNoSkipped, sampleImportRowRaw } from "./fixtures";

describe("MAX_IMPORT_BYTES", () => {
  it("is exactly 25 MB", () => {
    expect(MAX_IMPORT_BYTES).toBe(25 * 1024 * 1024);
  });

  it("is a positive integer count of bytes", () => {
    expect(Number.isInteger(MAX_IMPORT_BYTES)).toBe(true);
    expect(MAX_IMPORT_BYTES).toBeGreaterThan(0);
  });
});

describe("XLSX_MAGIC", () => {
  it("is the ZIP local-file-header signature (PK)", () => {
    expect(XLSX_MAGIC).toEqual([0x50, 0x4b]);
  });
});

describe("SPREADSHEET_PARSE_ERROR", () => {
  it("is a non-empty user-facing string", () => {
    expect(SPREADSHEET_PARSE_ERROR.length).toBeGreaterThan(0);
    expect(SPREADSHEET_PARSE_ERROR).toContain("Could not read");
  });
});

describe("normalizeKey", () => {
  it("lowercases and strips non-alphanumerics", () => {
    expect(normalizeKey("Client Name")).toBe("clientname");
    expect(normalizeKey("MINER_IP")).toBe("minerip");
  });

  it("collapses mixed separators to a single normalized token", () => {
    expect(normalizeKey("client_name")).toBe("clientname");
    expect(normalizeKey("Client Name")).toBe("clientname");
    expect(normalizeKey("Client-Name")).toBe("clientname");
    expect(normalizeKey("Client  Name")).toBe("clientname");
  });

  it("strips dots, slashes, and whitespace", () => {
    expect(normalizeKey("rack.group / label")).toBe("rackgrouplabel");
  });
});

describe("value / buildKeyMap integration", () => {
  it("returns the trimmed value when a normalized key matches", () => {
    const row: ImportRow = { "Client Name": "  Acme  " };
    const keyMap = buildKeyMap(row);
    expect(value(row, "client_name", keyMap)).toBe("Acme");
  });

  it("returns empty string when the key is not present", () => {
    const row: ImportRow = { foo: "bar" };
    const keyMap = buildKeyMap(row);
    expect(value(row, "client_name", keyMap)).toBe("");
  });

  it("converts a Date value to YYYY-MM-DD", () => {
    const row: ImportRow = { acquired: new Date("2024-06-15T00:00:00Z") };
    const keyMap = buildKeyMap(row);
    expect(value(row, "acquired", keyMap)).toBe("2024-06-15");
  });

  it("returns empty string for null and undefined", () => {
    const row: ImportRow = { a: null, b: undefined };
    const keyMap = buildKeyMap(row);
    expect(value(row, "a", keyMap)).toBe("");
    expect(value(row, "b", keyMap)).toBe("");
  });
});

describe("nullable", () => {
  it("converts empty string to null", () => {
    expect(nullable("")).toBeNull();
  });

  it("preserves non-empty strings", () => {
    expect(nullable("hello")).toBe("hello");
  });
});

describe("normalizeModel", () => {
  it("detects S21 XP by name and joined spelling", () => {
    expect(normalizeModel("Antminer S21 XP")).toBe("S21 XP");
    expect(normalizeModel("S21XP")).toBe("S21 XP");
  });

  it("detects S21 Pro variants", () => {
    expect(normalizeModel("Antminer S21 Pro")).toBe("S21 Pro");
    expect(normalizeModel("s21pro")).toBe("S21 Pro");
  });

  it("detects S21+", () => {
    expect(normalizeModel("Antminer S21+")).toBe("S21+");
  });

  it("falls back to plain S21 for unrecognized values", () => {
    expect(normalizeModel("S19 Pro")).toBe("S21");
    expect(normalizeModel("")).toBe("S21");
  });

  it("is case-insensitive", () => {
    expect(normalizeModel("ANTMINER S21 XP")).toBe("S21 XP");
  });
});

describe("normalizeStatus", () => {
  it("routes RMA via either state or status", () => {
    expect(normalizeStatus("RMA", "")).toBe("RMA");
    expect(normalizeStatus("", "RMA")).toBe("RMA");
  });

  it("routes Retired when state contains 'retired'", () => {
    expect(normalizeStatus("retired", "")).toBe("Retired");
  });

  it("routes Spare when state contains 'spare'", () => {
    expect(normalizeStatus("Spare Unit", "")).toBe("Spare");
  });

  it("routes Under Repair on fail/repair/offline text", () => {
    expect(normalizeStatus("hashboard failure", "")).toBe("Under Repair");
    expect(normalizeStatus("", "Under Repair")).toBe("Under Repair");
    expect(normalizeStatus("offline", "")).toBe("Under Repair");
  });

  it("defaults to In Service", () => {
    expect(normalizeStatus("running", "active")).toBe("In Service");
    expect(normalizeStatus("", "")).toBe("In Service");
  });
});

describe("normalizeDate", () => {
  it("returns ISO-format dates unchanged", () => {
    expect(normalizeDate("2024-01-02")).toBe("2024-01-02");
  });

  it("extracts an ISO date embedded in a longer string", () => {
    expect(normalizeDate("Acquired on 2023-12-31 by Acme")).toBe("2023-12-31");
  });

  it("converts m/d/yyyy (US order, destructive parsing as documented)", () => {
    // The audit L-5 verified the implementation interprets "1/2/2024" as
    // month=1, day=2 (US format) and outputs YYYY-MM-DD. This test pins the
    // exact behavior so a "fix" that swaps month and day is intentional.
    expect(normalizeDate("1/2/2024")).toBe("2024-01-02");
  });

  it("zero-pads single-digit month and day", () => {
    expect(normalizeDate("3/4/2024")).toBe("2024-03-04");
  });

  it("rejects out-of-range month and day", () => {
    expect(normalizeDate("13/45/2024")).toBeNull();
  });

  it("rejects nonsense input", () => {
    expect(normalizeDate("not a date")).toBeNull();
    expect(normalizeDate("")).toBeNull();
    expect(normalizeDate("2024/01/02")).toBeNull();
  });
});

describe("buildLocation", () => {
  it("joins pickaxe, rack group, rack, and slot with ' / '", () => {
    const row: ImportRow = {
      pickaxe: "PX-1",
      "miner rack group": "A",
      "miner rack": "R7",
      "miner row": "3",
      "miner index": "12",
    };
    const keyMap = buildKeyMap(row);
    expect(buildLocation(row, keyMap)).toBe("PX-1 / A / R7 / Row 3 Index 12");
  });

  it("omits the slot when row and index are both missing", () => {
    const row: ImportRow = { pickaxe: "PX-1", "miner rack": "R7" };
    const keyMap = buildKeyMap(row);
    expect(buildLocation(row, keyMap)).toBe("PX-1 / R7");
  });

  it("returns null when no location fields are set", () => {
    const row: ImportRow = { pickaxe: "" };
    const keyMap = buildKeyMap(row);
    expect(buildLocation(row, keyMap)).toBeNull();
  });
});

describe("buildNotes", () => {
  it("joins extra fields with 'Label: value' lines", () => {
    const row: ImportRow = {
      "miner id": "42",
      "miner name": "North Rack 1",
      wattage: "3500",
    };
    const keyMap = buildKeyMap(row);
    const notes = buildNotes(row, keyMap);
    expect(notes).toContain("Miner ID: 42");
    expect(notes).toContain("Name: North Rack 1");
    expect(notes).toContain("Wattage: 3500");
  });

  it("returns null when no extra fields are present", () => {
    const row: ImportRow = { serial: "X" };
    const keyMap = buildKeyMap(row);
    expect(buildNotes(row, keyMap)).toBeNull();
  });
});

describe("mapImportRow", () => {
  it("maps a fully-populated import row to a CreateMinerInput", () => {
    const result = mapImportRow(sampleImportRowRaw);
    expect(result).not.toBeNull();
    expect(result!.serial).toBe("ANT-9001");
    expect(result!.model).toBe("S21 Pro");
    expect(result!.client_name).toBe("Acme Mining");
    expect(result!.firmware).toBe("2.0.1");
    expect(result!.status).toBe("In Service");
    expect(result!.acquired_date).toBe("2024-01-02");
    expect(result!.location).toBe("PX-1 / B / R3 / Row 5 Index 9");
    expect(result!.notes).toContain("Miner ID: 42");
  });

  it("accepts alternate header spellings (case + space)", () => {
    const result = mapImportRow({ "Client Name": "Acme", "Miner Serial": "X-1" });
    expect(result).not.toBeNull();
    expect(result!.client_name).toBe("Acme");
    expect(result!.serial).toBe("X-1");
  });

  it("returns null when no serial is present", () => {
    expect(mapImportRow({ "Client Name": "Acme" })).toBeNull();
    expect(mapImportRow({})).toBeNull();
  });

  it("falls back to a plain 'serial' column when 'miner_serial' is absent", () => {
    const result = mapImportRow({ serial: "X-2", "miner type": "S21 XP" });
    expect(result).not.toBeNull();
    expect(result!.serial).toBe("X-2");
    expect(result!.model).toBe("S21 XP");
  });

  it("writes null for missing optional fields, never empty strings", () => {
    const result = mapImportRow({ "miner serial": "X-3" });
    expect(result).not.toBeNull();
    expect(result!.firmware).toBeNull();
    expect(result!.client_name).toBeNull();
    expect(result!.notes).toBeNull();
  });

  it("routes state text 'failure' to 'Under Repair'", () => {
    const result = mapImportRow({ "miner serial": "X-4", "miner state": "hashboard failure" });
    expect(result!.status).toBe("Under Repair");
  });
});

describe("rowsToObjects", () => {
  it("uses the first row as headers and produces key/value objects", () => {
    const rows = [
      ["serial", "model"],
      ["ANT-1", "S21"],
      ["ANT-2", "S21 Pro"],
    ];
    const out = rowsToObjects(rows);
    expect(out).toEqual([
      { serial: "ANT-1", model: "S21" },
      { serial: "ANT-2", model: "S21 Pro" },
    ]);
  });

  it("returns an empty array when only headers are present", () => {
    expect(rowsToObjects([["a", "b"]])).toEqual([]);
  });

  it("returns an empty array when no header row is present", () => {
    expect(rowsToObjects([])).toEqual([]);
  });

  it("skips blank header cells", () => {
    const rows = [
      ["serial", "", "model"],
      ["ANT-1", "ignored", "S21"],
    ];
    const out = rowsToObjects(rows);
    expect(out).toEqual([{ serial: "ANT-1", model: "S21" }]);
  });
});

describe("parseDelimited", () => {
  it("splits a single CSV row by commas", () => {
    expect(parseDelimited("a,b,c", ",")).toEqual([["a", "b", "c"]]);
  });

  it("splits a single TSV row by tabs", () => {
    expect(parseDelimited("a\tb\tc", "\t")).toEqual([["a", "b", "c"]]);
  });

  it("supports quoted fields containing the delimiter", () => {
    expect(parseDelimited('"a,b",c', ",")).toEqual([["a,b", "c"]]);
  });

  it("supports escaped double quotes inside quoted fields", () => {
    expect(parseDelimited('"He said ""hi""",ok', ",")).toEqual([['He said "hi"', "ok"]]);
  });

  it("handles CRLF line endings", () => {
    expect(parseDelimited("a,b\r\nc,d", ",")).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
  });

  it("handles LF line endings", () => {
    expect(parseDelimited("a,b\nc,d", ",")).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
  });

  it("ignores a trailing empty row", () => {
    expect(parseDelimited("a,b\n", ",")).toEqual([["a", "b"]]);
  });
});

describe("hasExcelMagicBytes", () => {
  it("returns true for a buffer that starts with the ZIP magic", async () => {
    const file = new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], "x.xlsx");
    await expect(hasExcelMagicBytes(file)).resolves.toBe(true);
  });

  it("returns false for a buffer with the wrong first byte", async () => {
    const file = new File([new Uint8Array([0x00, 0x4b])], "x.xlsx");
    await expect(hasExcelMagicBytes(file)).resolves.toBe(false);
  });

  it("returns false for a buffer with the wrong second byte", async () => {
    const file = new File([new Uint8Array([0x50, 0x00])], "x.xlsx");
    await expect(hasExcelMagicBytes(file)).resolves.toBe(false);
  });

  it("returns false for an empty file", async () => {
    const file = new File([new Uint8Array([])], "x.xlsx");
    await expect(hasExcelMagicBytes(file)).resolves.toBe(false);
  });

  it("returns false for a CSV that starts with 'A,' (no magic bytes)", async () => {
    const file = new File([new Uint8Array([0x41, 0x2c])], "x.csv");
    await expect(hasExcelMagicBytes(file)).resolves.toBe(false);
  });
});

describe("formatImportMessage", () => {
  it("renders the standard 3-bucket message when skipped is zero", () => {
    expect(formatImportMessage(sampleImportResultNoSkipped)).toBe(
      "Imported miners: 5 added, 2 updated.",
    );
  });

  it("includes the skipped count when it is non-zero", () => {
    expect(formatImportMessage(sampleImportResult)).toBe(
      "Imported miners: 5 added, 2 updated, 1 skipped.",
    );
  });

  it("renders a zero/zero/zero case as '0 added, 0 updated.'", () => {
    expect(formatImportMessage({ imported: 0, updated: 0, skipped: 0 })).toBe(
      "Imported miners: 0 added, 0 updated.",
    );
  });
});

describe("readSpreadsheetRows", () => {
  it("parses a CSV file with a quoted header row", async () => {
    const csv = "client_name,miner_serial\nAcme,ANT-1\nFoo,ANT-2\n";
    const file = new File([csv], "miners.csv", { type: "text/csv" });
    const rows = await readSpreadsheetRows(file);
    expect(rows).toEqual([
      { client_name: "Acme", miner_serial: "ANT-1" },
      { client_name: "Foo", miner_serial: "ANT-2" },
    ]);
  });

  it("parses a TSV file with tabs as delimiter", async () => {
    const tsv = "client_name\tminer_serial\nAcme\tANT-1\n";
    const file = new File([tsv], "miners.tsv", { type: "text/tab-separated-values" });
    const rows = await readSpreadsheetRows(file);
    expect(rows).toEqual([{ client_name: "Acme", miner_serial: "ANT-1" }]);
  });

  it("rejects an .xlsx file that fails the ZIP-magic sniff", async () => {
    const file = new File([new Uint8Array([0x41, 0x42])], "fake.xlsx");
    await expect(readSpreadsheetRows(file)).rejects.toThrow(SPREADSHEET_PARSE_ERROR);
  });
});
