import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/ui/DataTable";

interface Row {
  id: number;
  serial: string;
  status: string;
}

const data: Row[] = [
  { id: 1, serial: "ANT-001", status: "In Service" },
  { id: 2, serial: "ANT-002", status: "Under Repair" },
  { id: 3, serial: "ANT-003", status: "Retired" },
];

const columns: ColumnDef<Row>[] = [
  { accessorKey: "serial", header: "Serial" },
  { accessorKey: "status", header: "Status" },
];

describe("DataTable", () => {
  it("renders every row's columns and headers", () => {
    render(<DataTable columns={columns} data={data} searchPlaceholder="Filter" />);

    expect(screen.getByText("Serial")).toBeInTheDocument();
    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getByText("ANT-001")).toBeInTheDocument();
    expect(screen.getByText("ANT-002")).toBeInTheDocument();
    expect(screen.getByText("ANT-003")).toBeInTheDocument();
  });

  it("filters rows by the global search input", async () => {
    const user = userEvent.setup();
    render(<DataTable columns={columns} data={data} searchPlaceholder="Filter" />);

    const search = screen.getByPlaceholderText("Filter");
    await user.type(search, "002");

    expect(screen.getByText("ANT-002")).toBeInTheDocument();
    expect(screen.queryByText("ANT-001")).not.toBeInTheDocument();
    expect(screen.queryByText("ANT-003")).not.toBeInTheDocument();
  });

  it("invokes onRowClick with the row's data when a row is clicked", async () => {
    const user = userEvent.setup();
    const onRowClick = vi.fn();
    render(
      <DataTable columns={columns} data={data} searchPlaceholder="Filter" onRowClick={onRowClick} />,
    );

    const row = screen.getByText("ANT-002").closest("tr")!;
    await user.click(within(row).getByText("ANT-002"));

    expect(onRowClick).toHaveBeenCalledTimes(1);
    expect(onRowClick).toHaveBeenCalledWith(data[1]);
  });

  it("renders an empty-state cell when no rows are present", () => {
    render(<DataTable columns={columns} data={[]} searchPlaceholder="Filter" />);
    expect(screen.getByText("No records yet.")).toBeInTheDocument();
  });

  it("sorts ascending then descending on repeated header clicks", async () => {
    const user = userEvent.setup();
    render(<DataTable columns={columns} data={data} searchPlaceholder="Filter" />);

    // Initial order is by row insertion (no sort applied).
    const rowsBefore = screen.getAllByRole("row").slice(1);
    expect(rowsBefore[0]).toHaveTextContent("ANT-001");

    // First click → ascending sort. Row order is already ANT-001..ANT-003 in
    // ascending order so the visible top row does not change, but we assert
    // by checking the bottom row to make sure the order is asc.
    await user.click(screen.getByText("Serial"));
    const rowsAsc = screen.getAllByRole("row").slice(1);
    expect(rowsAsc[0]).toHaveTextContent("ANT-001");
    expect(rowsAsc[rowsAsc.length - 1]).toHaveTextContent("ANT-003");

    // Second click → descending sort.
    await user.click(screen.getByText("Serial"));
    const rowsDesc = screen.getAllByRole("row").slice(1);
    expect(rowsDesc[0]).toHaveTextContent("ANT-003");
    expect(rowsDesc[rowsDesc.length - 1]).toHaveTextContent("ANT-001");
  });
});
