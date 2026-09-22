// Testing Library's DOM matchers, for every test file.
import "@testing-library/jest-dom/vitest";

// jsdom does no real layout, so it has no `ResizeObserver` at all — every
// component that watches for a resize needs at least a no-op stand-in to
// mount without throwing. Individual tests that care about resize behaviour
// replace this with their own mock instance.
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class ResizeObserver {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  };
}
