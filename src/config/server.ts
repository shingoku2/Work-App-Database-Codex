const MANUAL_SERVER_URL = "https://";

export function initialServerUrl(configuredUrl = import.meta.env.VITE_FLEET_SERVER_URL): string {
  const value = configuredUrl?.trim();
  if (!value) {
    return MANUAL_SERVER_URL;
  }

  try {
    const url = new URL(value);
    if (url.protocol !== "https:" || !url.hostname || url.username || url.password) {
      return MANUAL_SERVER_URL;
    }
    url.pathname = "";
    url.search = "";
    url.hash = "";
    return url.toString().replace(/\/$/, "");
  } catch {
    return MANUAL_SERVER_URL;
  }
}
