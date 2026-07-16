---
name: testing-tui
description: Build, run, and drive the options_tracker Cursive terminal UI for end-to-end testing. Use when verifying UI or db changes (trade add/edit/view, reports, enum parsing/validation).
---

# Testing the options_tracker TUI

This is a Rust + Cursive terminal UI backed by SQLite (`rusqlite`, bundled). It is a
TUI, so end-to-end tests must run it in a real terminal (GUI terminal via computer
use), not by piping stdin.

## Build & prerequisites
- Toolchain: needs recent stable Rust. A transitive dep (`time-core`) requires the
  cargo `edition2024` feature (stable >= 1.85). If `cargo build` fails parsing a
  manifest with `feature edition2024 is required`, run `rustup update stable`.
- Build: `cargo build` → binary at `target/debug/options_tracker`.
- Non-interactive smoke test (no TUI): `cargo run --example db_test` — exercises
  add/get/update/delete/report against a temp `test_options.db` and prints `All tests passed!`.
- Lint/format: `cargo clippy --all-targets` and `cargo fmt --check`.

## Running for a UI test
- The app creates/uses `options_tracker.db` in the **current working directory**.
  Run from a clean dir (e.g. copy the binary to an empty folder and delete any
  `options_tracker.db`) so the trade list/report start empty and are deterministic.
- A GUI terminal is available: `konsole` (launch with `DISPLAY=:0 konsole --workdir <dir>`).
  Maximize with `DISPLAY=:0 wmctrl -r :ACTIVE: -b add,maximized_vert,maximized_horz`.
- Menu: arrow keys + Enter. Add New Trade / View/Edit Trades / View Reports / Quit.

## Cursive input quirks (important — these caused friction)
- `EditView` ignores `ctrl+u` / `ctrl+k`. To clear a field: click/focus it, press
  `End`, then `BackSpace` repeatedly.
- Form rows are tightly spaced (~1 line apart), so clicking a specific field row is
  unreliable. Prefer **Tab / Shift+Tab** to move between fields. Tab order is
  Symbol → Type → Action → Price → Quantity → Date → Fees → Comment → Save → Cancel,
  and wraps around (from Cancel, Tab → Symbol).
- Clicking a Cursive button (e.g. `<Save>`) while a text field is focused sometimes
  needs **two clicks** (first moves focus, second activates). Or Tab to the button
  and press Enter.
- Dialogs: dismiss with Enter (the highlighted `<OK>` button).

## What to verify for enum / validation changes
- Add validation: invalid Type → dialog `Type must be 'stock' or 'option'`; invalid
  Action → `Action must be 'buy' or 'sell'`; neither is saved. Type/Action are
  case-insensitive (e.g. `Option` is accepted and stored lowercase).
- Round-trip: a saved trade should appear in View/Edit Trades with the exact stored
  values (proves `ToSql` write + `FromSql` read).
- Reports: P/L per symbol = sum of sells `(price*qty)-fees` minus buys `(price*qty)+fees`.
  E.g. option buy 10x1 (−$10) + stock sell 20x1 (+$20) = `$10.00` over 2 trades.

## Devin Secrets Needed
- None. Fully local; no external services or credentials.
