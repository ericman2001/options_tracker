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