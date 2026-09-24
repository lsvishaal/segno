import { drawingSurface, readPage } from "./scoreGeometry";
import type { PageGeometry } from "./scoreGeometry";

const SVG = "http://www.w3.org/2000/svg";

/**
 * Draws what the geometry layer reads from a page, over the page itself:
 * notehead boxes, the five lines of every staff with its clef and line
 * spacing, and every bar box. Inside the page's own drawing surface, so it
 * stays aligned at any zoom.
 */
export function showGeometry(page: Element, visible: boolean): void {
  const surface = drawingSurface(page);
  if (!surface) return;

  surface.querySelector(":scope > g.hw-debug")?.remove();
  if (!visible) return;

  surface.append(debugLayer(readPage(page)));
}

function debugLayer(geometry: PageGeometry): SVGGElement {
  const layer = element("g", { class: "hw-debug" });

  for (const bar of geometry.bars) {
    layer.append(
      element("rect", {
        class: "hw-debug-bar",
        x: bar.x0,
        y: bar.top,
        width: bar.x1 - bar.x0,
        height: bar.bottom - bar.top,
      }),
    );
  }

  for (const system of geometry.systems) {
    for (const staff of system.staves) {
      for (const y of staff.lines) {
        layer.append(element("line", { class: "hw-debug-line", x1: staff.x0, x2: staff.x1, y1: y, y2: y }));
      }

      const clef = geometry.bars.find((bar) => bar.system === system.index)?.clefs[staff.index];
      const label = element("text", {
        class: "hw-debug-label",
        x: staff.x0,
        y: staff.lines[0]! - staff.spacing * 0.6,
        "font-size": staff.spacing * 1.1,
      });
      label.textContent = `${clef ? `${clef.sign} clef` : "no clef"} · ${Math.round(staff.spacing)}`;
      layer.append(label);
    }
  }

  for (const head of geometry.noteheads.values()) {
    layer.append(
      element("rect", {
        class: "hw-debug-head",
        x: head.x,
        y: head.y - head.height / 2,
        width: head.width,
        height: head.height,
      }),
    );
  }

  return layer;
}

function element<K extends keyof SVGElementTagNameMap>(
  name: K,
  attributes: Record<string, string | number>,
): SVGElementTagNameMap[K] {
  const created = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attributes)) {
    created.setAttribute(key, String(value));
  }
  return created;
}
