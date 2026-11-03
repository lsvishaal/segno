import { describe, expect, it } from "vitest";

import { barLabel, barName, barsInline, barsLabel } from "./bars";

describe("bar names", () => {
  it("call the pickup the pickup, never bar 0", () => {
    expect(barLabel(0)).toBe("Pickup");
    expect(barName(0)).toBe("the pickup");
    expect(barsLabel(0, 2)).toBe("Pickup–bar 2");
    expect(barsInline(0, 2)).toBe("pickup–bar 2");
    expect(barsLabel(0, 0)).toBe("Pickup");
    expect(barsInline(0, 0)).toBe("the pickup");
  });

  it("number every other bar as printed", () => {
    expect(barLabel(7)).toBe("Bar 7");
    expect(barName(7)).toBe("bar 7");
    expect(barsLabel(6, 7)).toBe("Bars 6–7");
    expect(barsInline(6, 7)).toBe("bars 6–7");
  });
});
