import { useCallback, useEffect, useMemo, useState } from "react";

import { library, openPiece, startupFailure } from "./bridge";
import type { OpenedPiece, PieceSummary, Unavailable } from "./bridge";
import type { Marked } from "./components/Score";
import { LibraryScreen } from "./screens/Library";
import { PracticeScreen } from "./screens/Practice";
import { UnavailableScreen } from "./screens/Unavailable";

/**
 * What the window is showing.
 *
 * A discriminated union rather than several booleans, so "loaded but also
 * failed" is not a state that can be reached.
 */
type Screen =
  | { readonly kind: "starting" }
  | { readonly kind: "unavailable"; readonly failure: Unavailable }
  | { readonly kind: "library"; readonly pieces: readonly PieceSummary[] }
  | {
      readonly kind: "practice";
      readonly pieces: readonly PieceSummary[];
      readonly piece: OpenedPiece;
    };

export function App() {
  const [screen, setScreen] = useState<Screen>({ kind: "starting" });

  useEffect(() => {
    let live = true;

    // Whether Hogwash started is asked first: if it did not, nothing else is
    // worth asking.
    void (async () => {
      try {
        const failure = await startupFailure();
        if (!live) return;

        if (failure) {
          setScreen({ kind: "unavailable", failure });
          return;
        }

        const pieces = await library();
        if (live) setScreen({ kind: "library", pieces });
      } catch (error) {
        if (live) setScreen({ kind: "unavailable", failure: defect(error) });
      }
    })();

    return () => {
      live = false;
    };
  }, []);

  const open = useCallback(
    (id: string) => {
      void (async () => {
        try {
          const piece = await openPiece(id);

          setScreen((current) =>
            current.kind === "library" || current.kind === "practice"
              ? { kind: "practice", pieces: current.pieces, piece }
              : current,
          );
        } catch (error) {
          setScreen({ kind: "unavailable", failure: defect(error) });
        }
      })();
    },
    [],
  );

  const leave = useCallback(() => {
    setScreen((current) =>
      current.kind === "practice"
        ? { kind: "library", pieces: current.pieces }
        : current,
    );
  }, []);

  // Until the keyboard is wired up, the cursor rests on the first expected
  // position — which is where `003 §7.16` says a Cursor begins.
  const marked = useMemo<Marked>(
    () => (screen.kind === "practice" ? new Map([[0, "cursor" as const]]) : new Map()),
    [screen.kind],
  );

  switch (screen.kind) {
    case "starting":
      // Deliberately blank. Startup is milliseconds, and a spinner that
      // flashes for one frame is worse than nothing.
      return <main className="min-h-full" />;

    case "unavailable":
      return <UnavailableScreen failure={screen.failure} />;

    case "library":
      return <LibraryScreen pieces={screen.pieces} onOpen={open} />;

    case "practice":
      return (
        <PracticeScreen piece={screen.piece} marked={marked} onLeave={leave} />
      );
  }
}

/**
 * A command that threw.
 *
 * That is a defect in Hogwash rather than something the performer did, and
 * saying so is better than a blank window.
 */
function defect(error: unknown): Unavailable {
  return {
    summary: "Hogwash ran into a problem.",
    detail: String(error),
    lookedIn: [],
  };
}
