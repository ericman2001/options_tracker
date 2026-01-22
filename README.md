# Stock Options Tracker

A terminal-based stock and options trading tracker built with Rust, featuring a TUI interface powered by ratatui.

## Features

- **Trade Entry**: Enter both stock and option trades with full details
  - Stock symbol
  - Trade type (stock or option)
  - Action (buy or sell, including short sells)
  - Price per unit
  - Quantity
  - Date of transaction
  - Fees paid
  - Comment/notes field

- **Trade Management**: Review and edit past trades
  - View all trades in a table format
  - Edit existing trades
  - Delete trades
  - Trades are sorted by date (most recent first)

- **Reports**: Generate profit/loss reports by symbol
  - See total profit/loss for each stock symbol
  - Number of trades per symbol
  - Color-coded results (green for profit, red for loss)

## Technology Stack

- **Language**: Rust
- **Database**: SQLite (via rusqlite with bundled SQLite)
- **User Interface**: ratatui (Terminal User Interface)
- **Terminal Control**: crossterm

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

- **Main Menu**: Use ↑/↓ arrow keys to navigate, Enter to select, 'q' to quit
- **Add/Edit Trade**: Tab/Shift+Tab to move between fields, type to edit, Enter to save, Esc to cancel
- **View Trades**: ↑/↓ to navigate trades, 'e' to edit, 'd' to delete, Esc to go back
- **Reports**: Esc to go back to main menu

### Adding a Trade

1. Select "Add New Trade" from the main menu
2. Fill in the required fields:
   - **Symbol**: Stock ticker (e.g., AAPL, TSLA)
   - **Type**: Enter "stock" or "option"
   - **Action**: Enter "buy" or "sell"
   - **Price**: Price per unit (e.g., 150.50)
   - **Quantity**: Number of shares/contracts (e.g., 100)
   - **Date**: Transaction date in YYYY-MM-DD format (e.g., 2024-01-15)
   - **Fees**: Transaction fees (e.g., 5.00)
   - **Comment**: Optional notes
3. Press Enter to save

### Editing Trades

1. Select "View/Edit Trades" from the main menu
2. Navigate to the trade you want to edit using ↑/↓
3. Press 'e' to edit the selected trade
4. Modify the fields as needed
5. Press Enter to save changes

### Viewing Reports

1. Select "View Reports" from the main menu
2. The report shows:
   - Each symbol traded
   - Total profit/loss (considering buy costs and sell revenues, minus fees)
   - Number of trades for that symbol

## Data Storage

The application stores all data in a local SQLite database file named `options_tracker.db` in the directory where you run the application. This file is automatically created on first run.

## Profit/Loss Calculation

The profit/loss calculation follows this logic:
- **Buy transactions**: -(price × quantity) - fees (money spent)
- **Sell transactions**: (price × quantity) - fees (money received)
- **Total P/L**: Sum of all transactions for a symbol

This allows tracking of:
- Long positions (buy low, sell high)
- Short positions (sell high, buy low)
- Partial positions and multiple entries/exits

## License

This project is open source and available under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
