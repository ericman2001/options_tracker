# Agent Guidelines for options_tracker

## Build and Development Commands

### Basic Build Commands
```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Build with optimizations
cargo build --release

# Run the application
cargo run

# Run in release mode
cargo run --release
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test function
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests with detailed output
cargo test -- --show-output

# Run example programs
cargo run --example db_test

# Run specific example
cargo run --example db_test
```

### Code Quality
```bash
# Check code formatting
cargo fmt

# Check for clippy warnings
cargo clippy

# Check with clippy and fix issues
cargo clippy --fix

# Run full check
cargo check
```

## Code Style Guidelines

### Import Style
- Import standard library modules first with `use std::`
- Import dependencies second with `use crate::`
- Group imports alphabetically within each section
- Use full paths for clarity when needed
- Prefer specific imports over wildcard imports
```rust
// Good
use std::sync::Arc;
use crate::db::Database;

// Avoid
use *;
```

### Formatting
- Follow rustfmt default configuration
- Run `cargo fmt` before committing
- Use consistent indentation (4 spaces)
- Align closing braces with opening keywords
```rust
// Good
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::new("options_tracker.db")?;
    Ok(())
}

// Avoid
fn main() -> Result<(), Box<dyn std::error::Error>>{
    let db = Database::new("options_tracker.db")?;
    Ok(())
}
```

### Naming Conventions
- Use clear, descriptive names
- Type names: CamelCase for structs/enums (e.g., `Trade`, `TradeType`)
- Function names: snake_case for functions (e.g., `show_main_menu`)
- Constant names: SNAKE_CASE for constants (if any)
- Module names: snake_case for modules
- Fields: camelCase for struct fields
- Make names self-documenting
```rust
pub struct Trade {
    pub id: Option<i64>,
    pub symbol: String,
    pub trade_type: TradeType,
    pub action: Action,
    // ...
}
```

### Error Handling
- Use Rust's Result types extensively
- Return `Result<T, E>` from functions
- Use `?` operator for error propagation
- Provide useful error messages
- Handle errors gracefully in UI layer
```rust
pub fn add_trade(&self, trade: &Trade) -> Result<i64> {
    self.conn.execute(
        "INSERT INTO trades (symbol, trade_type, action, price, quantity, date, fees, comment)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            trade.symbol,
            trade.trade_type,
            trade.action,
            trade.price,
            trade.quantity,
            trade.date,
            trade.fees,
            trade.comment,
        ],
    )?;
    Ok(self.conn.last_insert_rowid())
}
```

### Type Definitions
- Use enums for type safety (e.g., `TradeType`, `Action`)
- Implement `Display`, `From`, `Into` traits for enums
- Use enums for state management and variants
- Keep types focused and specific
```rust
#[derive(Debug, Clone)]
pub enum TradeType {
    Stock,
    Option,
}

impl From<TradeType> for String {
    fn from(t: TradeType) -> String {
        t.to_string()
    }
}
```

### Code Organization
- Organize code logically: enums → structs → impl blocks
- Keep related functions grouped together
- Use descriptive function names
- Keep functions focused and single-purpose
- Follow existing module structure in src/

### Database Operations
- Use rusqlite's parameterized queries to prevent SQL injection
- Handle connection errors appropriately
- Use transactions for multi-step operations
- Wrap database operations in `Result` types
- Close connections properly or let rusqlite manage automatically

### UI Development
- Use Cursive's idiomatic patterns for terminal UI
- Handle user input validation thoroughly
- Provide clear error messages in dialog boxes
- Use Arc<Mutex> for shared state between UI and database
- Keep UI logic clean and separate from business logic

### Testing
- Write tests for critical database operations
- Mock database connections when testing UI
- Test error scenarios
- Verify input validation

### Comments and Documentation
- Add module-level documentation for public APIs
- Document complex logic briefly
- Avoid redundant comments
- Use clear variable and function names to reduce need for comments

## Domain Model (wheel-strategy tracker)

The app tracks stock and option trades to support the options "wheel" strategy.
Key domain rules (all enums use the `string_enum!` macro in `src/macros.rs`):

- **Action** has open/close semantics for both stock and options:
  `buy_to_open`, `sell_to_open`, `buy_to_close`, `sell_to_close`. On stock,
  `sell_to_open`/`buy_to_close` open and cover a short. Cash-flow direction
  depends only on the buy/sell side (`Action::is_buy`); open/close is
  informational.
- **OptionType**: `call`, `put`. **OptionStatus**: `open`, `closed`,
  `assigned`, `exercised`, `expired`. Both `assigned` and `exercised` trigger
  the compound stock event, but the resulting share direction depends on the
  option's long/short side (see below); `closed`/`expired` produce no linked
  stock row.
- **100x multiplier**: option cash flow is `price * quantity * 100` (one
  contract = 100 shares); stock is `price * quantity`. See `Trade::multiplier`
  / `Trade::cash_flow`.
- **Assignment/exercise is a compound event** (`Database::assign_option` →
  `Database::insert_linked_stock_row`): it sets the option status and inserts a
  linked stock row at the strike (`assigned_from` = option id) for `qty * 100`
  shares. Direction depends on option type **and** long/short side: short put
  assigned → buy, short call assigned → sell, long put exercised → sell, long
  call exercised → buy. Late reconciliation (past expiration) is allowed.
  Deleting the option deletes its linked rows; editing an option re-reconciles
  them — the linked row is regenerated while the option stays in a
  stock-generating status and removed otherwise (`delete_trade`,
  `expire_option`, `update_trade`). Linked rows are read-only in the UI.
  All of these multi-write operations run inside a transaction
  (`Connection::unchecked_transaction`).
- **Expiration** (`Database::expire_option`) closes an option with no extra
  cash flow (premium already booked at open).
- **Break-even** (`Database::get_break_even`) = `-(sum of all cash flows) /
  net_shares`, folding in premium and fees; `None` when flat.
- **Covered-call warning**: selling a call (`sell_to_open`) below break-even
  shows a non-blocking "Save Anyway"/"Cancel" dialog (distinct from the
  early-return `Dialog::info` used for validation errors).

## Dates

- No date crate (`chrono`/`time`). Dates and expirations are stored as ISO 8601
  `YYYY-MM-DD` in SQLite `TEXT`. SQLite has no native date type; zero-padded ISO
  text sorts chronologically and is what the UI displays — do **not** convert to
  `INTEGER`/`REAL`.
- `src/date.rs` derives "today" and days-to-expiration from `std::time` only,
  via Howard Hinnant's integer `civil_from_days`/`days_from_civil`. The civil
  date is **UTC-based**, acceptable because it only drives the non-blocking
  expiration alert and DTE display.

## No backwards compatibility

There are no real users. Redefine the schema directly in `Database::init_schema`
and delete any existing `options_tracker.db` rather than writing migrations.