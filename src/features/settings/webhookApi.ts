import { command } from "@/lib/tauri";
import type { CreateWebhook, UpdateWebhook, Webhook, WebhookDelivery } from "@/types/db";

export async function listWebhooks(): Promise<Webhook[]> {
  return command<Webhook[]>("list_webhooks");
}

export async function createWebhook(input: CreateWebhook): Promise<Webhook> {
  return command<Webhook>("create_webhook", { input });
}

export async function updateWebhook(input: UpdateWebhook): Promise<Webhook> {
  return command<Webhook>("update_webhook", { input });
}

export async function deleteWebhook(id: number, version: number): Promise<void> {
  return command<void>("delete_webhook", { id, version });
}

export async function listWebhookDeliveries(id: number): Promise<WebhookDelivery[]> {
  return command<WebhookDelivery[]>("list_webhook_deliveries", { id });
}
