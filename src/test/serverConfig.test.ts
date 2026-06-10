import { describe, expect, it } from "vitest";
import { initialServerUrl } from "@/config/server";

describe("hosted server configuration", () => {
  it("normalizes an HTTPS server URL for first-run pairing", () => {
    expect(initialServerUrl(" https://fleet.example:8443/api?ignored=yes#ignored ")).toBe(
      "https://fleet.example:8443",
    );
  });

  it("falls back to manual entry when the build setting is missing or unsafe", () => {
    expect(initialServerUrl("")).toBe("https://");
    expect(initialServerUrl("http://fleet.example:8443")).toBe("https://");
    expect(initialServerUrl("https://user:password@fleet.example:8443")).toBe("https://");
    expect(initialServerUrl("not a URL")).toBe("https://");
  });
});
