# Hogwash

**A desktop piano practice companion, built around standard sheet music.**

You open a score, play it on a MIDI keyboard, and see immediately whether each
note is right — with the score waiting for you, and the session recorded.

---

## Status

Early development. There is no application to run yet.

The specification is written and frozen first; the code follows it. The
authoritative documents live in `docs/`, which is a separate repository:

| Document | Owns |
|---|---|
| `001_system_charter.md` | Purpose, principles, scope, governance |
| `002_Requirements_Specification.md` | Observable behaviour (FR / NFR / C) |
| `003_Domain_Model.md` | The vocabulary and what it means |
| `004_System_Architecture.md` | Components, boundaries, state ownership |
| `product/MVP_Definition.md` | What Version 1 is, and what it is not |
| `product/Product_Discovery.md` | Research, rationale, the capability ledger |

Implementation may not invent a domain concept (`004 §3`) or a component
boundary (`004 §14`). Where code and documents disagree, the documents win.

## What makes it different

Practice **waits for you**. The cursor marks the note expected next and does
not advance until you play it.

That is not merely the simplest practice mode — it removes a whole class of
failure. A tool that keeps scoring after it has lost your place produces
confident, plausible nonsense on exactly the passages you cannot yet play,
which is precisely where you needed the truth. Waiting makes that
structurally impossible rather than something to defend against.

It also has a cost worth stating: if the system waits for you, the tempo *is*
your speed, so **timing cannot be assessed in Version 1**. Wait-for-me and
timing assessment are mutually exclusive. That is a derivation, not a
preference (`MVP §5`, `003 §7.4`).

## What it is not

Not a falling-notes game. Standard notation is the centre of the product, and
falling-notes presentation is a permanent exclusion (`MVP §7`), not a
deferral.

Not a reader of MIDI files as scores. A MIDI file records a performance, not
written music — no clefs, no key signature, no spelling, no articulation as
written. A score reconstructed from one is a guess you would have to correct
by hand (`001 §11`). MIDI is used for live input from your instrument, which
is a completely different role.

Not a cloud service. No accounts, no telemetry, no network calls. Everything
is local (`001 §7`).

## Architecture

```
Library ◀── Acquisition & Validation ──▶ Notation Representation
   │                                            │
   ▼                                            ▼
Practice Coordinator ◀── played Notes ── Performance Capture
   │                                            │
   ▼                                            ▼
Presentation ◀── Feedback ◀── Feedback ◀── Performance Comparison
```

One crate per component. The engine crates — `notation`, `practice`,
`analysis`, `feedback` — are pure: no I/O, no MIDI, no GUI, no async, and no
wall-clock time anywhere. `hogwash-domain` has no dependencies at all.

That is enforced rather than intended: `scripts/check-arch.sh` fails the build
if any of them acquires a dependency on the outside world.

## Building

Requires the toolchain pinned in `rust-toolchain.toml` (rustup fetches it
automatically) and `cargo-deny`:

```sh
cargo install cargo-deny --locked
just ci
```

`just ci` is the gate: formatting, lints as errors, tests, licence and
advisory checks, and the architecture guard.

## Licence

**Not yet chosen.** `Product_Discovery.md §3` commits to open source without
naming a licence, and the choice is constrained by the deferred engraving
decision — Verovio is LGPL-3.0, and linking against it carries obligations.
Both are settled together, deliberately, rather than one being decided by
accident of the other.
