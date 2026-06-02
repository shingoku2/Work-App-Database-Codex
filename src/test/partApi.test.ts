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
  it("passes the sku under 'sku'", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);
    await deletePart("HSB-S21");
    expect(mockedCommand).toHaveBeenCalledWith("delete_part", { sku: "HSB-S21" });
  });
});
