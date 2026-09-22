import type { Box } from "./Score";

/**
 * The letter name of every drawn note, labelled in place.
 *
 * A pure rendering component — `Score.tsx` does the measuring (the same
 * `boxOf` it already uses for the cursor and dots) and hands over finished
 * boxes, so this component never touches the DOM itself.
 */
export function NoteNames({
  labels,
}: {
  readonly labels: readonly { elementId: string; step: string; box: Box }[];
}) {
  return (
    <>
      {labels.map((label) => (
        <span
          key={label.elementId}
          className="hw-note-name"
          style={{
            left: label.box.left + label.box.width / 2,
            top: label.box.top,
          }}
        >
          {label.step}
        </span>
      ))}
    </>
  );
}
