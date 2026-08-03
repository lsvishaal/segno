import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";

import { AnimatePresence, motion, MotionConfig } from "motion/react";

import { library, openPiece, startupFailure } from "./bridge";
import type { OpenedPiece, PieceSummary, Unavailable } from "./bridge";
import { LibraryScreen } from "./screens/Library";
import { PracticeScreen } from "./screens/Practice";
import { UnavailableScreen } from "./screens/Unavailable";

/**
 * A cross-fade between screens, never a hard cut.
 *
 * The `key` that makes `AnimatePresence` treat one screen as leaving and the
 * next as entering belongs on this component itself at each call site, not
 * on the `motion.div` inside it — `AnimatePresence` only tracks the identity
 * of its direct children.
 */
function Transitioned({ children }: { readonly children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      className="h-full"
    >
      {children}
    </motion.div>
  );
}

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

  // `reducedMotion="user"` makes every `motion.*` component below honour
  // `prefers-reduced-motion` automatically, in one place, rather than each
  // component checking it independently.
  return (
    <MotionConfig reducedMotion="user">
      <AnimatePresence mode="wait">
        {(() => {
          switch (screen.kind) {
            case "starting":
              // Deliberately blank. Startup is milliseconds, and a spinner
              // that flashes for one frame is worse than nothing.
              return <main key="starting" className="min-h-full" />;

            case "unavailable":
              return (
                <Transitioned key="unavailable">
                  <UnavailableScreen failure={screen.failure} />
                </Transitioned>
              );

            case "library":
              return (
                <Transitioned key="library">
                  <LibraryScreen pieces={screen.pieces} onOpen={open} />
                </Transitioned>
              );

            case "practice":
              return (
                <Transitioned key={`practice-${screen.piece.id}`}>
                  <PracticeScreen piece={screen.piece} onLeave={leave} />
                </Transitioned>
              );
          }
        })()}
      </AnimatePresence>
    </MotionConfig>
  );
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
