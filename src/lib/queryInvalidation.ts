import type { QueryClient } from "@tanstack/react-query";

export async function invalidateFleetData(
  queryClient: QueryClient,
  queryKey: "miners" | "parts",
): Promise<void> {
  await queryClient.invalidateQueries({ queryKey: [queryKey] });
  await queryClient.invalidateQueries({ queryKey: ["dashboard"] });
}
