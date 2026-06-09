import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InventoryView } from "@/features/inventory/InventoryView";
import { listParts, updatePart } from "@/features/inventory/partApi";
import { samplePart } from "./fixtures";

vi.mock("@/features/inventory/partApi", () => ({
  createPart: vi.fn(),
  deletePart: vi.fn(),
  listParts: vi.fn(),
  updatePart: vi.fn(),
}));

const mockedListParts = vi.mocked(listParts);
const mockedUpdatePart = vi.mocked(updatePart);

beforeEach(() => {
  mockedListParts.mockReset();
  mockedUpdatePart.mockReset();
  mockedListParts.mockResolvedValue([samplePart]);
  mockedUpdatePart.mockResolvedValue(undefined);
});

function renderInventory() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <InventoryView />
    </QueryClientProvider>,
  );
}

describe("InventoryView integer currency", () => {
  it("renders integer cents as an exact dollar amount", async () => {
    renderInventory();

    expect(await screen.findByText("$199.99")).toBeInTheDocument();
  });

  it("converts edited dollars to integer cents before updating a part", async () => {
    const actor = userEvent.setup();
    renderInventory();
    const row = (await screen.findByText(samplePart.sku)).closest("tr");
    expect(row).not.toBeNull();

    await actor.click(within(row!).getByRole("button", { name: "Edit" }));
    const costInput = screen.getByPlaceholderText("Unit cost");
    expect(costInput).toHaveValue(199.99);
    await actor.clear(costInput);
    await actor.type(costInput, "0.10");
    await actor.click(screen.getByRole("button", { name: "Save Part" }));

    await waitFor(() =>
      expect(mockedUpdatePart).toHaveBeenCalledWith({
        ...samplePart,
        unit_cost_cents: 10,
      }),
    );
  });
});
