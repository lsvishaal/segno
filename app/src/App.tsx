import { useEffect, useState } from "react";

import { library, startupFailure } from "./bridge";
import type { PieceSummary, Unavailable } from "./bridge";
import { LibraryScreen } from "./screens/Library";
import { UnavailableScreen } from "./screens/Unavailable";

/**
 * What the window is showing.
 *
 * A discriminated union rather than a handful of booleans, so "loaded but
 * also failed" is not a state that can be reached.
 */
type Screen =
  | { readonly kind: "starting" }
  | { readonly kind: "unavailable"; readonly failure: Unavailable }
  | { readonly kind: "library"; readonly pieces: readonly PieceSummary[] };

export function App() {
  const [screen, setScreen] = useState<Screen>({ kind: "starting" });

  useEffect(() => {
    let live = true;

    // Whether Hogwash started is asked first, because if it did not there is
    // nothing else worth asking.
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
        if (!live) return;

        // A command that threw is a defect rather than something the
        // performer did, and saying so is better than a blank window.
        setScreen({
          kind: "unavailable",
          failure: {
            summary: "Hogwash could not start.",
            detail: String(error),
            lookedIn: [],
          },
        });
      }
    })();

    return () => {
      live = false;
    };
  }, []);

  switch (screen.kind) {
    case "starting":
      // Deliberately blank. Startup is milliseconds, and a spinner that
      // flashes for one frame is worse than nothing.
      return <main className="min-h-full" />;

    case "unavailable":
      return <UnavailableScreen failure={screen.failure} />;

    case "library":
      return (
        <LibraryScreen
          pieces={screen.pieces}
          onOpen={() => {
            // Practice arrives in the next stage.
          }}
        />
      );
  }
}
