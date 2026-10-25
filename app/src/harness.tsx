import ReactDOM from "react-dom/client";

import { marksOf, PIECES } from "./components/__fixtures__/pages";
import type { Piece } from "./components/__fixtures__/pages";
import type { PieceSummary } from "./bridge";
import { Score } from "./components/Score";
import { LibraryScreen } from "./screens/Library";
import "./styles/index.css";

/**
 * Dev-only page: the real score component on a real piece's first page,
 * outside Tauri, so a plain or headless browser shows exactly what the app
 * draws. `just harness`, then:
 *
 *   /harness.html?piece=minuet-in-g&step=3&wrong=76&names=1&debug=1
 *   /harness.html?screen=library
 */
const query = new URLSearchParams(location.search);
const piece = (query.get("piece") ?? "minuet-in-g") as Piece;
const step = Number(query.get("step") ?? 0);
const wrong = query.get("wrong");

/** The shelf as the performer's real pieces summarise (titles as Hogwash reads them). */
const SHELF: PieceSummary[] = [
  { id: "minuet", title: "Minuet in G Major", composer: "Christian Petzold", bars: 32, notes: 209, fileName: null, imported: false, level: "intermediate", lastPractisedMs: null, weakest: [] },
  { id: "prelude", title: "Prelude in C Major", composer: "Johann Sebastian Bach", bars: 35, notes: 533, fileName: null, imported: false, level: "intermediate", lastPractisedMs: null, weakest: [] },
  { id: "canon", title: "Canon in D", composer: "Johann Pachelbel", bars: 57, notes: 1604, fileName: "Canon_in_D.mxl", imported: true, level: null, lastPractisedMs: null, weakest: [] },
  { id: "full", title: "Für Elise", composer: "arr. Verona", bars: 105, notes: 1118, fileName: "Fur_Elise.mxl", imported: true, level: null, lastPractisedMs: null, weakest: [] },
  { id: "easy", title: "Fur Elise", composer: null, bars: 33, notes: 141, fileName: "Fur_Elise_Easy.mxl", imported: true, level: null, lastPractisedMs: null, weakest: [] },
  { id: "moon", title: "Sonate No. 14", composer: "Ludwig van Beethoven", bars: 69, notes: 900, fileName: "Moonlight.mxl", imported: true, level: null, lastPractisedMs: null, weakest: [] },
];

const root = ReactDOM.createRoot(document.getElementById("root")!);

if (query.get("screen") === "library") {
  root.render(
    <LibraryScreen
      pieces={SHELF}
      onOpen={() => {}}
      onRename={async () => null}
      onCountHistory={async () => 3}
      onDelete={async () => null}
      onImport={() => {}}
      importing={false}
      importError={null}
      onDismissError={() => {}}
    />,
  );
} else root.render(
  <div className="hw-score" style={{ padding: 24 }}>
    <Score
      pages={[PIECES[piece].svg]}
      marks={marksOf(piece)}
      cursor={{ position: step }}
      dot={wrong ? { kind: "wrong", position: step, pitch: Number(wrong) } : null}
      followCursor={false}
      showNoteNames={query.has("names")}
      showGeometry={query.has("debug")}
    />
  </div>,
);
