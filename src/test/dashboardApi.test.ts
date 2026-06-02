import { describe, expect, it, vi } from "vitest";
import { getDashboardSummary } from "@/features/dashboard/dashboardApi";
import { command } from "@/lib/tauri";
import type { DashboardSummary } from "@/types/db";

vi.mock("@/lib/tauri", () => ({
  command: vi.fn(),
}));

const mockedCommand = vi.mocked(command);

describe("getDashboardSummary", () => {
  it("invokes the 'get_dashboard_summary' tauri command with no args", async () => {
    const fixture: DashboardSummary = {
      unit_count: 10,
      part_count: 4,
      low_stock_count: 2,
      units_by_status: [
        { status: "In Service", count: 8 },
        { status: "Under Repair", count: 2 },
      ],
      low_stock_parts: [],
    };
    mockedCommand.mockResolvedValueOnce(fixture);

    const result = await getDashboardSummary();

    expect(mockedCommand).toHaveBeenCalledWith("get_dashboard_summary");
    expect(result).toEqual(fixture);
  });

  it("propagates errors thrown by the command wrapper", async () => {
    mockedCommand.mockRejectedValueOnce(new Error("backend offline"));

    await expect(getDashboardSummary()).rejects.toThrow("backend offline");
  });
});
