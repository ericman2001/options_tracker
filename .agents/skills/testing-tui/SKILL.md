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
- The trade list is ~90 cols wide; long option rows are clipped at the right edge but
  the list is horizontally scrollable (since 37614ad, `scroll_x(true)`) — focus the
  list and press Right to reveal trailing `exp / status / DTE`. To confirm an option's
  status without scrolling, open its actions dialog: an `open` option shows
  Assign/Exercise/Expire; a resolved one (assigned/expired) shows only Edit/Delete.

## Mutex re-lock deadlock pattern (was a bug; fixed in 37614ad — regression-watch)
- History: at ea24536, clicking **Expire** or **Delete** froze the whole app because the
  handler held the `Database` `MutexGuard` as a temporary in the `match` scrutinee and
  then called `show_view_trades`, which re-locks the same non-reentrant
  `std::sync::Mutex` (`src/ui.rs` ~505 → 410). Fixed at 37614ad by binding the call to
  `let res = db.lock()…expire_option(id);` (guard dropped before the refresh) — the
  same pattern Assign already used. Verified: Expire + Delete now refresh without hanging.
- Regression watch: any callback that both `db.lock()`s and then calls a function that
  re-locks (`show_view_trades`, reports, etc.) can re-introduce this. If the TUI hangs
  after a button press, confirm with
  `sudo gdb -p <pid> -batch -ex "bt" | grep ui.rs` — a `show_view_trades` frame under a
  `show_trade_actions::{closure}` frame, both blocked in a mutex lock = this deadlock.
- Note the DB write (e.g. `expire_option`) completes before the UI re-lock, so if it
  ever hangs again, relaunching still shows the intended data change.

## Environment gotchas
- Do **not** run the app from `/tmp` — it can be wiped mid-session (tmpfs cleanup),
  losing the DB and closing the terminal. Use a stable dir like `/home/ubuntu/ottest`.
- If konsole closes, Chrome may take the foreground and steal keystrokes; re-activate
  konsole with `wmctrl -ia <konsole_wid>` before sending input.

## What to verify for enum / validation changes
- Add validation: invalid Type → dialog `Type must be 'stock' or 'option'`; invalid
  Action → `Action must be one of: buy_to_open, sell_to_open, buy_to_close, sell_to_close`;
  option with blank Option Type/Strike/Expiration → `Option Type must be 'call' or 'put'`
  / `Invalid strike` / `Expiration is required for options`; none is saved. Type/Action
  are case-insensitive (e.g. `Option` is accepted and stored lowercase).
- Round-trip: a saved trade should appear in View/Edit Trades with the exact stored
  values (proves `ToSql` write + `FromSql` read).
- Reports: P/L per symbol = sum of sells `(price*qty)-fees` minus buys `(price*qty)+fees`.
  E.g. option buy 10x1 (−$10) + stock sell 20x1 (+$20) = `$10.00` over 2 trades.

## Devin Secrets Needed
- None. Fully local; no external services or credentials.
