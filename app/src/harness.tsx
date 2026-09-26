import ReactDOM from "react-dom/client";

import { marksOf, PIECES } from "./components/__fixtures__/pages";
import type { Piece } from "./components/__fixtures__/pages";
import { Score } from "./components/Score";
import "./styles/index.css";

/**
 * Dev-only page: the real score component on a real piece's first page,
 * outside Tauri, so a plain or headless browser shows exactly what the app
 * draws. `just harness`, then:
 *
 *   /harness.html?piece=minuet-in-g&step=3&wrong=76&names=1&debug=1
 */
const query = new URLSearchParams(location.search);
const piece = (query.get("piece") ?? "minuet-in-g") as Piece;
const step = Number(query.get("step") ?? 0);
const wrong = query.get("wrong");

ReactDOM.createRoot(document.getElementById("root")!).render(
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
