# Stock Options Tracker

A terminal-based stock and options trading tracker built with Rust, featuring a TUI interface powered by Cursive.

## Features

This app is geared toward the **wheel** options strategy: it tracks both the
underlying stock and option legs, and helps you avoid common pitfalls (notably
writing a covered call below your break-even).

- **Trade Entry**: Enter both stock and option trades with full details
  - Stock symbol
  - Trade type (stock or option)
  - Action with open/close semantics: `buy_to_open`, `sell_to_open`,
    `buy_to_close`, `sell_to_close`. On stock, `sell_to_open`/`buy_to_close`
    represent opening and covering a short position.
  - Price per unit, Quantity, Date, Fees, Comment
  - Option legs additionally capture Option Type (`call`/`put`), Strike, and
    Expiration (`YYYY-MM-DD`)

- **Option Lifecycle**: Each option carries a status — `open`, `closed`,
  `assigned`, `exercised`, or `expired` — with lifecycle actions in the
  View/Edit screen:
  - **Assign / Exercise**: closes the option (no extra cash flow beyond the
    premium booked at open) and auto-generates a **linked stock trade** at the
    strike for `qty x 100` shares. The buy/sell direction follows the option's
    type and long/short side: a **short** put assigned buys shares and a short
    call assigned sells shares; a **long** put exercised sells shares and a long
    call exercised buys shares. Deleting the option cleans up the linked row, and
    editing the option re-syncs it (the linked row's strike/quantity track the
    edit, or the row is removed if the option leaves an assigned/exercised
    status).
  - **Expire**: closes the option worthless with no cash flow (a sold option
    keeps its premium; a bought option realizes its loss — both already booked
    at open).
  - Open options past their expiration are flagged with a **non-blocking
    alert** prompting you to resolve them (never auto-resolved). Days-to-
    expiration (DTE) is shown per option.

- **Trade Management**: Review and edit past trades
  - View all trades in a list format (options show type/strike/expiration/status/DTE)
  - Edit, delete, or run lifecycle actions on trades
  - Trades are sorted by date (most recent first)

- **Reports**: Generate profit/loss reports by symbol
  - Total profit/loss for each symbol (options use the 100x contract multiplier)
  - Current net share position (long/short/flat) and per-underlying break-even
  - Number of trades per symbol

## Technology Stack

- **Language**: Rust
- **Database**: SQLite (via rusqlite with bundled SQLite)
- **User Interface**: Cursive (Terminal User Interface)

## Installation

### Prerequisites

- Rust toolchain (1.70 or later recommended)
- Cargo (comes with Rust)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/ericman2001/options_tracker.git
cd options_tracker

# Build the project
cargo build --release

# Run the application
cargo run --release
```

## Usage

### Navigation

The application uses an intuitive dialog-based interface:
- **Main Menu**: Use arrow keys (↑/↓) to navigate, Enter to select
- **Forms**: Use Tab to move between fields, type to edit text fields, and use the dropdown selectors (Type, Action, Option Type) via Enter/arrow keys or the mouse; click buttons or use keyboard shortcuts
- **Lists**: Use arrow keys to navigate, Enter to select items

### Adding a Trade

1. Select "Add New Trade" from the main menu
2. Fill in the required fields:
   - **Symbol**: Stock ticker (e.g., AAPL, TSLA)
   - **Type**: Dropdown selector — choose `stock` or `option`
   - **Action**: Dropdown selector — `buy_to_open`, `sell_to_open`,
     `buy_to_close`, or `sell_to_close`
   - **Price**: Price per unit (per share; for options this is the premium per share)
   - **Quantity**: Number of shares (stock) or contracts (option)
   - **Date**: Transaction date in YYYY-MM-DD format (e.g., 2024-01-15)
   - **Fees**: Transaction fees (e.g., 5.00)
   - **Option Type / Strike / Expiration**: shown only when Type is `option`
     (Option Type is a `call`/`put` dropdown)
   - **Comment**: Optional notes
3. Click "Save" or press the keyboard shortcut to save

Dropdowns (Type, Action, Option Type) open on Enter or a mouse click; pick a
value with the arrow keys + Enter or by clicking it. Selecting `stock` hides the
Option Type / Strike / Expiration fields, and `option` reveals them.

When you sell-to-open a **call** whose strike is below the underlying's current
break-even, a non-blocking warning appears ("Save Anyway" / "Cancel") because
assignment would lock in a loss.

### Editing Trades

1. Select "View/Edit Trades" from the main menu
2. Navigate to the trade you want to edit using arrow keys
3. Press Enter to select the trade
4. Choose "Edit" from the dialog
5. Modify the fields as needed
6. Click "Save" to save changes

### Deleting Trades

1. Select "View/Edit Trades" from the main menu
2. Navigate to the trade you want to delete
3. Press Enter to select the trade
4. Choose "Delete" from the dialog
5. The trade will be removed immediately

### Viewing Reports

1. Select "View Reports" from the main menu
2. The report shows:
   - Each symbol traded
   - Total profit/loss (considering buy costs and sell revenues, minus fees)
   - Number of trades for that symbol

## Data Storage

The application stores all data in a local SQLite database file named `options_tracker.db` in the directory where you run the application. This file is automatically created on first run.

Dates and expirations are stored as ISO 8601 `YYYY-MM-DD` in SQLite `TEXT`
columns. SQLite has **no native date type**; zero-padded ISO text is the
idiomatic choice because it sorts chronologically (`ORDER BY date DESC`) and is
exactly what the UI displays — so we do not convert dates to `INTEGER`/`REAL`.

"Today" and days-to-expiration are derived purely from `std::time` (no `chrono`
or `time` dependency): the system clock is converted to a Unix day count and
then to a civil `(year, month, day)` via Howard Hinnant's integer
`civil_from_days` algorithm. This date is **UTC-based**, which is acceptable
because it only drives the non-blocking expiration alert and DTE display.

## Profit/Loss Calculation

Each trade contributes a signed cash flow:
- **Buy (`*_to_open`/`*_to_close`)**: `-(price × quantity × multiplier) - fees`
- **Sell (`*_to_open`/`*_to_close`)**: `(price × quantity × multiplier) - fees`
- **multiplier**: `100` for options (one contract = 100 shares), `1` for stock
- **Total P/L**: sum of all cash flows for a symbol

Assigned/exercised and expired options add no cash flow at their terminal event
(the premium was booked when the option was opened; for assignment the linked
stock row carries the strike cash flow).

**Break-even** for a symbol's net share position is derived from the whole
ledger as `-(sum of all cash flows) / net_shares`, which folds in collected
option premium and all fees. It works for both long and short positions and is
`None` when the position is flat.

This allows tracking of:
- Long positions (buy low, sell high)
- Short positions (sell high, buy low)
- Cash-secured puts and covered calls across the full wheel cycle
- Partial positions and multiple entries/exits

## License

This project is open source and available under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
