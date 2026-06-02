import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
} from "@tanstack/react-table";
import { useState } from "react";

export function DataTable<TData>({
  columns,
  data,
  searchPlaceholder = "Filter rows",
  onRowClick,
}: {
  columns: ColumnDef<TData>[];
  data: TData[];
  searchPlaceholder?: string;
  onRowClick?: (row: TData) => void;
}) {
  const [globalFilter, setGlobalFilter] = useState("");
  const table = useReactTable({
    data,
    columns,
    state: { globalFilter },
    onGlobalFilterChange: setGlobalFilter,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: {
      pagination: {
        pageSize: 50,
      },
    },
  });
  const filteredRows = table.getFilteredRowModel().rows.length;

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <input
          value={globalFilter}
          onChange={(event) => setGlobalFilter(event.target.value)}
          placeholder={searchPlaceholder}
          className="h-10 w-80 rounded-md border border-white/10 bg-white/5 px-3 text-sm outline-none ring-primary/30 placeholder:text-slate-500 focus:ring-2"
        />
        <div className="flex items-center gap-2 text-sm text-slate-400">
          <span>Show</span>
          <select
            className="h-10 rounded-md border border-white/10 bg-[#101821] px-2 text-slate-100 outline-none ring-primary/30 focus:ring-2"
            value={table.getState().pagination.pageSize >= filteredRows && filteredRows > 0 ? "all" : table.getState().pagination.pageSize}
            onChange={(event) => {
              table.setPageSize(event.target.value === "all" ? Math.max(filteredRows, 1) : Number(event.target.value));
            }}
          >
            {[25, 50, 100, 250].map((size) => (
              <option key={size} value={size}>{size}</option>
            ))}
            <option value="all">All</option>
          </select>
          <span>rows</span>
        </div>
      </div>
      <div className="overflow-auto rounded-lg border border-white/10">
        <table className="min-w-full border-collapse text-sm">
          <thead className="bg-white/[0.04] text-left text-xs uppercase text-slate-400">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <th key={header.id} className="px-4 py-3 font-semibold">
                    {header.isPlaceholder ? null : (
                      <button
                        type="button"
                        onClick={header.column.getToggleSortingHandler()}
                        className="text-left"
                      >
                        {flexRender(header.column.columnDef.header, header.getContext())}
                      </button>
                    )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.length === 0 && (
              <tr>
                <td className="px-4 py-8 text-center text-slate-500" colSpan={columns.length}>
                  No records yet.
                </td>
              </tr>
            )}
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className={`border-t border-white/10 hover:bg-white/[0.03] ${onRowClick ? "cursor-pointer" : ""}`}
                onClick={() => onRowClick?.(row.original)}
              >
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-4 py-3 text-slate-200">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 text-sm text-slate-400">
        <span>
          {filteredRows} rows
          {filteredRows > 0 && (
            <> · page {table.getState().pagination.pageIndex + 1} of {table.getPageCount()}</>
          )}
        </span>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => table.setPageIndex(0)}
            disabled={!table.getCanPreviousPage()}
            className="rounded-md border border-white/10 px-3 py-1 disabled:opacity-40"
          >
            First
          </button>
          <button
            type="button"
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
            className="rounded-md border border-white/10 px-3 py-1 disabled:opacity-40"
          >
            Previous
          </button>
          <input
            type="number"
            min="1"
            max={Math.max(table.getPageCount(), 1)}
            value={table.getState().pagination.pageIndex + 1}
            onChange={(event) => {
              const page = event.target.value ? Number(event.target.value) - 1 : 0;
              table.setPageIndex(Math.min(Math.max(page, 0), Math.max(table.getPageCount() - 1, 0)));
            }}
            className="h-8 w-16 rounded-md border border-white/10 bg-white/5 px-2 text-center text-slate-100 outline-none ring-primary/30 focus:ring-2"
          />
          <button
            type="button"
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
            className="rounded-md border border-white/10 px-3 py-1 disabled:opacity-40"
          >
            Next
          </button>
          <button
            type="button"
            onClick={() => table.setPageIndex(Math.max(table.getPageCount() - 1, 0))}
            disabled={!table.getCanNextPage()}
            className="rounded-md border border-white/10 px-3 py-1 disabled:opacity-40"
          >
            Last
          </button>
        </div>
      </div>
    </div>
  );
}
