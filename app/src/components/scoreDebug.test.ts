import { describe, expect, it } from "vitest";

import minuet from "./__fixtures__/minuet-in-g.page1.svg?raw";
import { showGeometry } from "./scoreDebug";

function page(): Element {
  const holder = document.createElement("div");
  holder.innerHTML = minuet;
  return holder;
}

describe("Show geometry", () => {
  it("draws notehead, staff-line, label and bar layers inside the page's shifted music group", () => {
    const holder = page();
    showGeometry(holder, true);

    const layer = holder.querySelector("svg.definition-scale > g.page-margin > g.hw-debug")!;
    expect(layer).not.toBeNull();
    expect(layer.querySelectorAll(".hw-debug-head")).toHaveLength(holder.querySelectorAll("g.note").length);
    expect(layer.querySelectorAll(".hw-debug-bar")).toHaveLength(holder.querySelectorAll("g.measure").length);
    expect(layer.querySelectorAll(".hw-debug-line").length % 5).toBe(0);
    expect(layer.querySelector(".hw-debug-label")?.textContent).toBe("G clef · 180");
  });

  it("removes itself when turned off, and never draws twice", () => {
    const holder = page();
    showGeometry(holder, true);
    showGeometry(holder, true);
    expect(holder.querySelectorAll("g.hw-debug")).toHaveLength(1);

    showGeometry(holder, false);
    expect(holder.querySelector("g.hw-debug")).toBeNull();
  });
});
