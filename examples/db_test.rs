use options_tracker::db::{Database, Trade};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Stock Options Tracker Database...\n");

    // Create test database
    let db = Database::new("test_options.db")?;
    println!("✓ Database created successfully");

    // Test adding a trade
    let trade1 = Trade {
        id: None,
        symbol: "AAPL".to_string(),
        trade_type: "stock".to_string().into(),
        action: "buy".to_string().into(),
        price: 150.50,
        quantity: 100.0,
        date: "2024-01-15".to_string(),
        fees: 5.00,
        comment: "Initial purchase".to_string(),
    };

    let id1 = db.add_trade(&trade1)?;
    println!("✓ Added trade 1 with ID: {}", id1);

    // Add another trade
    let trade2 = Trade {
        id: None,
        symbol: "AAPL".to_string(),
        trade_type: "stock".to_string().into(),
        action: "sell".to_string().into(),
        price: 165.75,
        quantity: 100.0,
        date: "2024-02-15".to_string(),
        fees: 5.00,
        comment: "Sold for profit".to_string(),
    };

    let id2 = db.add_trade(&trade2)?;
    println!("✓ Added trade 2 with ID: {}", id2);

    // Add a trade for a different symbol
    let trade3 = Trade {
        id: None,
        symbol: "TSLA".to_string(),
        trade_type: "option".to_string().into(),
        action: "buy".to_string().into(),
        price: 50.00,
        quantity: 10.0,
        date: "2024-03-01".to_string(),
        fees: 2.50,
        comment: "Call option".to_string(),
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
    for (symbol, profit_loss, count) in reports {
        println!("  - {}: ${:.2} ({} trades)", symbol, profit_loss, count);
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
