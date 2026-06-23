import { describe, expect, it, vi } from "vitest";
import { createPart, deletePart, listParts, updatePart } from "@/features/inventory/partApi";
import { command } from "@/lib/tauri";
import { samplePart } from "./fixtures";

vi.mock("@/lib/tauri", () => ({
  command: vi.fn(),
}));

const mockedCommand = vi.mocked(command);

describe("listParts", () => {
  it("invokes the 'list_parts' command", async () => {
    mockedCommand.mockResolvedValueOnce([samplePart]);
    const result = await listParts();
    expect(mockedCommand).toHaveBeenCalledWith("list_parts");
    expect(result).toEqual([samplePart]);
  });
});

describe("createPart", () => {
  it("wraps the part under 'input'", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);
    await createPart(samplePart);
    expect(mockedCommand).toHaveBeenCalledWith("create_part", { input: samplePart });
  });
});

describe("updatePart", () => {
  it("wraps the part under 'input'", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);
    await updatePart(samplePart);
    expect(mockedCommand).toHaveBeenCalledWith("update_part", { input: samplePart });
  });
});

describe("deletePart", () => {
  it("passes the sku, version, and site_id under their respective keys", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);
    await deletePart("HSB-S21", 4);
    expect(mockedCommand).toHaveBeenCalledWith("delete_part", { sku: "HSB-S21", version: 4, site_id: null });
  });

  it("forwards an explicit site_id when provided", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);
    await deletePart("HSB-S21", 4, 7);
    expect(mockedCommand).toHaveBeenCalledWith("delete_part", { sku: "HSB-S21", version: 4, site_id: 7 });
  });
});
