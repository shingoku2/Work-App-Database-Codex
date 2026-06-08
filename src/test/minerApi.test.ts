import { describe, expect, it, vi } from "vitest";
import {
  createMiner,
  deleteMiner,
  importMiners,
  listMiners,
  updateMiner,
  type CreateMinerInput,
} from "@/features/miners/minerApi";
import { command } from "@/lib/tauri";
import type { Miner } from "@/types/db";
import { sampleMiner, sampleMinerInput } from "./fixtures";

vi.mock("@/lib/tauri", () => ({
  command: vi.fn(),
}));

const mockedCommand = vi.mocked(command);

describe("listMiners", () => {
  it("invokes the 'list_miners' command and returns the array", async () => {
    mockedCommand.mockResolvedValueOnce([sampleMiner]);

    const result = await listMiners();

    expect(mockedCommand).toHaveBeenCalledWith("list_miners");
    expect(result).toEqual([sampleMiner]);
  });
});

describe("createMiner", () => {
  it("wraps the input under 'input' and returns the inserted id", async () => {
    mockedCommand.mockResolvedValueOnce(7);

    const id = await createMiner(sampleMinerInput);

    expect(mockedCommand).toHaveBeenCalledWith("create_miner", { input: sampleMinerInput });
    expect(id).toBe(7);
  });
});

describe("updateMiner", () => {
  it("wraps the full miner (including id) under 'input'", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);

    await updateMiner({ ...sampleMinerInput, id: sampleMiner.id } as Miner);

    expect(mockedCommand).toHaveBeenCalledWith("update_miner", {
      input: { ...sampleMinerInput, id: sampleMiner.id },
    });
  });
});

describe("importMiners", () => {
  it("wraps a single object as a one-element array under 'miners'", async () => {
    mockedCommand.mockResolvedValueOnce({ imported: 1, updated: 0, skipped: 0, conflicts: [] });

    await importMiners(sampleMinerInput as CreateMinerInput);

    expect(mockedCommand).toHaveBeenCalledWith("import_miners", { miners: [sampleMinerInput] });
  });

  it("passes an array of miners through under 'miners'", async () => {
    mockedCommand.mockResolvedValueOnce({ imported: 2, updated: 0, skipped: 0, conflicts: [] });

    const batch: CreateMinerInput[] = [sampleMinerInput, { ...sampleMinerInput, serial: "X-2" }];
    await importMiners(batch);

    expect(mockedCommand).toHaveBeenCalledWith("import_miners", { miners: batch });
  });
});

describe("deleteMiner", () => {
  it("passes the id under 'id'", async () => {
    mockedCommand.mockResolvedValueOnce(undefined);

    await deleteMiner(42, 3);

    expect(mockedCommand).toHaveBeenCalledWith("delete_miner", { id: 42, version: 3 });
  });
});
