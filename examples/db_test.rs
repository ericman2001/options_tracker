use options_tracker::db::{Action, Database, OptionStatus, OptionType, Trade, TradeType};
use rust_decimal_macros::dec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Stock Options Tracker Database...\n");

    // Create test database
    let db = Database::new("test_options.db")?;
    println!("✓ Database created successfully");

    // Test adding a trade
    let trade1 = Trade {
        symbol: "AAPL".to_string(),
        trade_type: TradeType::Stock,
        action: Action::BuyToOpen,
        price: dec!(150.50),
        quantity: dec!(100.0),
        date: "2024-01-15".to_string(),
        fees: dec!(5.00),
        comment: "Initial purchase".to_string(),
        ..Default::default()
    };

    let id1 = db.add_trade(&trade1)?;
    println!("✓ Added trade 1 with ID: {}", id1);

    // Add another trade
    let trade2 = Trade {
        symbol: "AAPL".to_string(),
        trade_type: TradeType::Stock,
        action: Action::SellToClose,
        price: dec!(165.75),
        quantity: dec!(100.0),
        date: "2024-02-15".to_string(),
        fees: dec!(5.00),
        comment: "Sold for profit".to_string(),
        ..Default::default()
    };

    let id2 = db.add_trade(&trade2)?;
    println!("✓ Added trade 2 with ID: {}", id2);

    // Add a trade for a different symbol
    let trade3 = Trade {
        symbol: "TSLA".to_string(),
        trade_type: TradeType::Option,
        action: Action::BuyToOpen,
        price: dec!(50.00),
        quantity: dec!(10.0),
        date: "2024-03-01".to_string(),
        fees: dec!(2.50),
        comment: "Call option".to_string(),
        option_type: Some(OptionType::Call),
        strike: Some(dec!(250.0)),
        expiration: Some("2024-06-21".to_string()),
        status: Some(OptionStatus::Open),
        ..Default::default()
    };

    let id3 = db.add_trade(&trade3)?;
    println!("✓ Added trade 3 with ID: {}", id3);

    // Retrieve all trades
    let trades = db.get_all_trades()?;
    println!("\n✓ Retrieved {} trades:", trades.len());
    for trade in &trades {
        println!(
            "  - {} {} {} @ ${:.2}",
            trade.action, trade.quantity, trade.symbol, trade.price
        );
    }

    // Test updating a trade
    let mut updated_trade = trades[0].clone();
    updated_trade.comment = "Updated comment".to_string();
    db.update_trade(&updated_trade)?;
    println!("\n✓ Updated trade successfully");

    // Generate report
    let reports = db.get_report_by_symbol()?;
    println!("\n✓ Generated reports for {} symbols:", reports.len());
    for report in reports {
        println!(
            "  - {}: ${:.2} ({} trades, {})",
            report.symbol,
            report.profit_loss,
            report.trade_count,
            report
                .break_even
                .map(|b| format!("break-even ${:.2}", b))
                .unwrap_or_else(|| "flat".to_string()),
        );
    }

    // Test deleting a trade
    db.delete_trade(id3)?;
    println!("\n✓ Deleted trade {} successfully", id3);

    let final_trades = db.get_all_trades()?;
    println!("✓ Final trade count: {}", final_trades.len());

    // Clean up test database
    std::fs::remove_file("test_options.db")?;
    println!("\n✓ All tests passed!");

    Ok(())
}
