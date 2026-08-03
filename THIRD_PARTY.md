# Third-party components that are not Rust dependencies

Hogwash is licensed under **Apache-2.0** (see `LICENSE`). It also uses
software that `cargo deny` cannot see, because it does not arrive through
Cargo. That software is listed here, and `scripts/check-native-licences.sh`
fails the build if this file stops matching what is actually used.

The gap this closes is worth stating plainly, because it is easy to be
reassured by the wrong thing: `deny.toml` polices every crate in the
dependency graph and reports "licenses ok", while the single most
licence-significant component in the product — an LGPL-3.0 engraving engine —
is invisible to it. A gate that cannot see the thing it exists to check is
worse than no gate, because it is trusted.

---

## Verovio

| | |
|---|---|
| What it does | Engraves the notation the performer reads (`ADR-0005`) |
| Upstream | https://github.com/rism-digital/verovio |
| Licence | **LGPL-3.0-or-later**, dual-licensed GPL-3.0 |
| How it is used | A separate shared library (`libverovio.so` / `.dll` / `.dylib`) plus its `data/` directory, **opened at run time** with `dlopen` (`libloading`) |
| Linked into Hogwash | **No.** Not statically, not at link time at all. |

### What that arrangement is for

LGPL-3.0 §4 permits combining the library with a work under other terms,
provided the user can replace the library with their own version. Hogwash
does not merely satisfy that clause, it is built on it: the library is never
linked, only located and opened by name at run time, so replacing it is
copying a file over another file. `ADR-0007` records the reasoning; `ADR-0005`
records why the same choice is also the right engineering one.

### Obligations, and where each is met

| Obligation | Met by |
|---|---|
| Give prominent notice that Verovio is used, and that it is LGPL-3.0 | This file, and the application's about screen once one exists (M9) |
| Ship a copy of the GPL-3.0 and LGPL-3.0 texts | `COPYING` and `COPYING.LESSER`, distributed beside the library |
| Let the user replace the library | Run-time loading, per above — no relinking materials are needed because nothing is linked |
| Do not restrict reverse engineering of the combined work | Nothing in `LICENSE` does |

**Not yet done, and not yet due:** the notice files and the about screen are
distribution artefacts, and Hogwash is not distributed. They are release
gates, tracked alongside C-200's provided-library verification in `MVP §9.2`.

### The version used

Pinned by commit rather than by version string, because the two disagree:
the repository tag reads `version-2.7.1-…` while the toolkit itself reports
`6.3.0-dev-fb5c4db` (`S0`). The toolkit's own answer is the true one.

```
commit fb5c4db7e
reports 6.3.0-dev-fb5c4db
```

Verovio is **never vendored into this repository** and never committed. It is
built from an upstream clone and shipped as a separate file.
