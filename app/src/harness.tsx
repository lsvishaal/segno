import ReactDOM from "react-dom/client";

import type { Mark, Step } from "./bridge";
import minuet from "./components/__fixtures__/minuet-in-g.page1.svg?raw";
import minuetNotes from "./components/__fixtures__/minuet-in-g.page1.notes.json";
import prelude from "./components/__fixtures__/prelude-in-c.page1.svg?raw";
import preludeNotes from "./components/__fixtures__/prelude-in-c.page1.notes.json";
import { Score } from "./components/Score";
import { readPage } from "./components/scoreGeometry";
import "./styles/index.css";

/**
 * Dev-only page: the real score component on a real piece's first page,
 * outside Tauri, so a plain browser (or a headless one) can show exactly
 * what the app draws. `just harness-shot` screenshots it.
 *
 *   /harness.html?piece=minuet-in-g&step=3&names=1&debug=1
 *
 * Steps are the page's note columns, left to right, system by system: close
 * enough to the performed order to put the cursor anywhere on the page.
 */
const PIECES: Record<string, { svg: string; notes: Record<string, { step: string; octave: number }> }> = {
  "minuet-in-g": { svg: minuet, notes: minuetNotes },
  "prelude-in-c": { svg: prelude, notes: preludeNotes },
};

const query = new URLSearchParams(location.search);
const piece = PIECES[query.get("piece") ?? "minuet-in-g"]!;
const step = Number(query.get("step") ?? 0);

const geometry = readPage(new DOMParser().parseFromString(piece.svg, "image/svg+xml").documentElement);
const heads = [...geometry.noteheads.values()].sort((a, b) => a.system - b.system || a.x - b.x);
const marks: Mark[] = [];
let columnX = -Infinity;
let columnSystem = -1;

for (const head of heads) {
  const spacing = geometry.systems[head.system]!.staves[0]!.spacing;
  if (head.system !== columnSystem || head.x - columnX > spacing) {
    marks.push({ measure: 1, position: marks.length, page: 0, notes: [] });
    columnX = head.x;
    columnSystem = head.system;
  }
  const spelling = piece.notes[head.id]!;
  (marks[marks.length - 1]!.notes as Mark["notes"][number][]).push({
    elementId: head.id,
    step: spelling.step as Step,
    octave: spelling.octave,
  });
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <div className="hw-score" style={{ padding: 24 }}>
    <Score
      pages={[piece.svg]}
      marks={marks}
      cursor={{ position: step }}
      dot={null}
      followCursor={false}
      showNoteNames={query.has("names")}
      showGeometry={query.has("debug")}
    />
  </div>,
);
