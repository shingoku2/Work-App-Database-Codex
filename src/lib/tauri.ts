import { invoke } from "@tauri-apps/api/core";

export async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(message || `Tauri command failed: ${name}`);
  }
}
