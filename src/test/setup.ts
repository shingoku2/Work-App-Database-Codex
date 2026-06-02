import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, vi } from "vitest";

// JSDOM 25's Blob implementation is missing `arrayBuffer()` and `text()` on
// both `Blob.prototype` and the objects returned by `Blob.prototype.slice()`.
// The WebView in production supports them natively; in tests we polyfill via
// `FileReader` so the import path's magic-byte sniff and CSV/TSV reader work.
if (typeof Blob !== "undefined") {
  if (typeof Blob.prototype.arrayBuffer !== "function") {
    Blob.prototype.arrayBuffer = function arrayBuffer(): Promise<ArrayBuffer> {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as ArrayBuffer);
        reader.onerror = () => reject(reader.error ?? new Error("FileReader failed"));
        reader.readAsArrayBuffer(this);
      });
    };
  }
  if (typeof Blob.prototype.text !== "function") {
    Blob.prototype.text = function text(): Promise<string> {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = () => reject(reader.error ?? new Error("FileReader failed"));
        reader.readAsText(this);
      });
    };
  }
}

// Tauri invokes go through `window.__TAURI_INTERNALS__` in production but
// during tests no Tauri host exists. Suppress the resulting console errors so
// the test output is not dominated by `window.alert` warnings.
const originalAlert = window.alert;
beforeEach(() => {
  window.alert = vi.fn();
});
afterEach(() => {
  window.alert = originalAlert;
  vi.restoreAllMocks();
});
