/**
 * A choice of MIDI device, as the browser's own `<select>`: it opens its
 * list the way the rest of the system does, and brings its own keyboard
 * and screen-reader behaviour.
 *
 * `none` adds a first option that means "no device" (value `null`); leave it
 * out where one has to be chosen.
 */
export function DeviceSelect({
  label,
  value,
  options,
  none,
  onChange,
}: {
  readonly label: string;
  readonly value: string | null;
  readonly options: readonly string[];
  readonly none?: string;
  readonly onChange: (value: string | null) => void;
}) {
  return (
    <select
      aria-label={label}
      value={value ?? ""}
      onChange={(event) => onChange(event.target.value || null)}
      className="min-w-0 flex-1 cursor-pointer rounded-md border border-rule-strong bg-paper-raised px-3 py-1.5 text-sm text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
    >
      {none !== undefined && <option value="">{none}</option>}
      {options.map((name) => (
        <option key={name} value={name}>
          {name}
        </option>
      ))}
    </select>
  );
}
