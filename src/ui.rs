use cursive::views::{Dialog, TextView, SelectView, EditView, ListView};
use cursive::traits::*;
use cursive::Cursive;
use cursive::align::HAlign;
use cursive::theme::{Color, PaletteColor};
use crate::db::{Database, Trade};
use std::sync::{Arc, Mutex};

pub fn run_ui(db: Database) {
    let db = Arc::new(Mutex::new(db));
    let mut siv = cursive::default();
    
    // Set up theme
    let mut theme = siv.current_theme().clone();
    theme.palette[PaletteColor::Background] = Color::TerminalDefault;
    theme.palette[PaletteColor::View] = Color::TerminalDefault;
    theme.palette[PaletteColor::Primary] = Color::Light(cursive::theme::BaseColor::White);
    theme.palette[PaletteColor::TitlePrimary] = Color::Dark(cursive::theme::BaseColor::Black);
    theme.palette[PaletteColor::TitlePrimary] = Color::Dark(cursive::theme::BaseColor::Red);
    siv.set_theme(theme);
    
    show_main_menu(&mut siv, db);
    
    siv.run();
}

fn show_main_menu(siv: &mut Cursive, db: Arc<Mutex<Database>>) {
    // Clear all layers first
    while siv.pop_layer().is_some() {}
    
    let mut select = SelectView::new()
        .h_align(HAlign::Center);
    
    select.add_item("Add New Trade", 1);
    select.add_item("View/Edit Trades", 2);
    select.add_item("View Reports", 3);
    select.add_item("Quit", 4);
    
    let db_clone = db.clone();
    select.set_on_submit(move |s, item: &i32| {
        match item {
            1 => show_add_trade(s, db_clone.clone(), None),
            2 => show_view_trades(s, db_clone.clone()),
            3 => show_reports(s, db_clone.clone()),
            4 => s.quit(),
            _ => {}
        }
    });
    
    siv.add_layer(
        Dialog::around(select.scrollable().fixed_size((40, 10)))
            .title("Stock Options Tracker")
            .button("Quit", |s| s.quit())
    );
}

fn show_add_trade(siv: &mut Cursive, db: Arc<Mutex<Database>>, trade: Option<Trade>) {
    let is_edit = trade.is_some();
    let title = if is_edit { "Edit Trade" } else { "Add New Trade" };
    
    let trade = trade.unwrap_or_default();
    
    let form = ListView::new()
        .child("Symbol:", EditView::new()
            .content(trade.symbol.clone())
            .with_name("symbol")
            .fixed_width(20))
        .child("Type (stock/option):", EditView::new()
            .content(trade.trade_type.to_string())
            .with_name("trade_type")
            .fixed_width(20))
        .child("Action (buy/sell):", EditView::new()
            .content(trade.action.to_string())
            .with_name("action")
            .fixed_width(20))
        .child("Price:", EditView::new()
            .content(if trade.price > 0.0 { format!("{:.2}", trade.price) } else { String::new() })
            .with_name("price")
            .fixed_width(20))
        .child("Quantity:", EditView::new()
            .content(if trade.quantity > 0.0 { format!("{:.2}", trade.quantity) } else { String::new() })
            .with_name("quantity")
            .fixed_width(20))
        .child("Date (YYYY-MM-DD):", EditView::new()
            .content(trade.date.clone())
            .with_name("date")
            .fixed_width(20))
        .child("Fees:", EditView::new()
            .content(if trade.fees > 0.0 { format!("{:.2}", trade.fees) } else { String::new() })
            .with_name("fees")
            .fixed_width(20))
        .child("Comment:", EditView::new()
            .content(trade.comment.clone())
            .with_name("comment")
            .fixed_width(20));
    
    let trade_id = trade.id;
    let db_clone = db.clone();
    
    siv.add_layer(
        Dialog::around(form.scrollable().fixed_size((50, 20)))
            .title(title)
            .button("Save", move |s| {
                let symbol = s.call_on_name("symbol", |view: &mut EditView| {
                    view.get_content().to_string().to_uppercase()
                }).unwrap_or_default();
                
                let trade_type = s.call_on_name("trade_type", |view: &mut EditView| {
                    view.get_content().to_string().to_lowercase()
                }).unwrap_or_default();
                
                let action = s.call_on_name("action", |view: &mut EditView| {
                    view.get_content().to_string().to_lowercase()
                }).unwrap_or_default();
                
                let price_str = s.call_on_name("price", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                let quantity_str = s.call_on_name("quantity", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                let date = s.call_on_name("date", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                let fees_str = s.call_on_name("fees", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                let comment = s.call_on_name("comment", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                // Validate inputs
                if symbol.is_empty() {
                    s.add_layer(Dialog::info("Symbol is required"));
                    return;
                }
                
                if trade_type != "stock" && trade_type != "option" {
                    s.add_layer(Dialog::info("Type must be 'stock' or 'option'"));
                    return;
                }
                
                if action != "buy" && action != "sell" {
                    s.add_layer(Dialog::info("Action must be 'buy' or 'sell'"));
                    return;
                }
                
                let price = match price_str.parse::<f64>() {
                    Ok(p) if p >= 0.0 => p,
                    _ => {
                        s.add_layer(Dialog::info("Invalid price"));
                        return;
                    }
                };
                
                let quantity = match quantity_str.parse::<f64>() {
                    Ok(q) if q > 0.0 => q,
                    _ => {
                        s.add_layer(Dialog::info("Invalid quantity"));
                        return;
                    }
                };
                
                let fees = match fees_str.parse::<f64>() {
                    Ok(f) if f >= 0.0 => f,
                    _ => {
                        s.add_layer(Dialog::info("Invalid fees"));
                        return;
                    }
                };
                
                if date.is_empty() {
                    s.add_layer(Dialog::info("Date is required"));
                    return;
                }
                
                // Validate date format (YYYY-MM-DD)
                if !is_valid_date_format(&date) {
                    s.add_layer(Dialog::info("Invalid date format. Use YYYY-MM-DD"));
                    return;
                }
                
                let new_trade = Trade {
                    id: trade_id,
                    symbol,
                    trade_type: trade_type.into(),
                    action: action.into(),
                    price,
                    quantity,
                    date,
                    fees,
                    comment,
                };
                
                let result = if trade_id.is_some() {
                    db_clone.lock().expect("Failed to lock database").update_trade(&new_trade)
                } else {
                    db_clone.lock().expect("Failed to lock database").add_trade(&new_trade).map(|_| ())
                };
                
                match result {
                    Ok(_) => {
                        s.pop_layer();
                        s.add_layer(Dialog::info("Trade saved successfully!")
                            .button("OK", |s| {
                                s.pop_layer();
                            }));
                    }
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Error: {}", e)));
                    }
                }
            })
            .button("Cancel", move |s| {
                s.pop_layer();
            })
    );
}

fn show_view_trades(siv: &mut Cursive, db: Arc<Mutex<Database>>) {
    let trades = match db.lock().expect("Failed to lock database").get_all_trades() {
        Ok(trades) => trades,
        Err(e) => {
            siv.add_layer(
                Dialog::info(format!("Database error: {}", e))
                    .button("Back", |s| {
                        s.pop_layer();
                    })
            );
            return;
        }
    };
    
    if trades.is_empty() {
        siv.add_layer(
            Dialog::info("No trades found")
                .button("Back", |s| {
                    s.pop_layer();
                })
        );
        return;
    }
    
    let mut select = SelectView::new().h_align(HAlign::Left);
    
    for trade in trades.iter() {
        let display = format!(
            "{:<6} {:<8} {:<6} {:<6} ${:<8.2} {:<6.2} {} ${}",
            trade.id.unwrap_or(0),
            trade.symbol,
            trade.trade_type.as_str(),
            trade.action.as_str(),
            trade.price,
            trade.quantity,
            trade.date,
            trade.fees
        );
        select.add_item(display, trade.clone());
    }
    
    let db_clone = db.clone();
    let db_clone2 = db.clone();
    
    select.set_on_submit(move |s, trade: &Trade| {
        s.add_layer(
            Dialog::text("What would you like to do?")
                .button("Edit", {
                    let trade = trade.clone();
                    let db = db_clone.clone();
                    move |s| {
                        s.pop_layer();
                        show_add_trade(s, db.clone(), Some(trade.clone()));
                    }
                })
                .button("Delete", {
                    let trade = trade.clone();
                    let db = db_clone2.clone();
                    move |s| {
                        if let Some(id) = trade.id {
                            match db.lock().expect("Failed to lock database").delete_trade(id) {
                                Ok(_) => {
                                    s.pop_layer();
                                    s.pop_layer();
                                    show_view_trades(s, db.clone());
                                }
                                Err(e) => {
                                    s.add_layer(Dialog::info(format!("Error deleting trade: {}", e)));
                                }
                            }
                        }
                    }
                })
                .button("Cancel", |s| {
                    s.pop_layer();
                })
        );
    });
    
    siv.add_layer(
        Dialog::around(select.scrollable().fixed_size((80, 20)))
            .title("View/Edit Trades")
            .button("Back", |s| {
                s.pop_layer();
            })
    );
}

fn show_reports(siv: &mut Cursive, db: Arc<Mutex<Database>>) {
    let reports = match db.lock().expect("Failed to lock database").get_report_by_symbol() {
        Ok(reports) => reports,
        Err(e) => {
            siv.add_layer(
                Dialog::info(format!("Database error: {}", e))
                    .button("Back", |s| {
                        s.pop_layer();
                    })
            );
            return;
        }
    };
    
    if reports.is_empty() {
        siv.add_layer(
            Dialog::info("No trades found")
                .button("Back", |s| {
                    s.pop_layer();
                })
        );
        return;
    }
    
    let mut content = String::new();
    content.push_str("Symbol       Profit/Loss    Trades\n");
    content.push_str("=========================================\n");
    
    for (symbol, profit_loss, count) in reports {
        content.push_str(&format!(
            "{:<12} ${:<13.2} {}\n",
            symbol, profit_loss, count
        ));
    }
    
    siv.add_layer(
        Dialog::around(TextView::new(content))
            .title("Profit/Loss Report by Symbol")
            .button("Back", |s| {
                s.pop_layer();
            })
    );
}

fn is_valid_date_format(date: &str) -> bool {
    // Check basic format: YYYY-MM-DD
    if date.len() != 10 {
        return false;
    }
    
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    
    // Check that year, month, day are valid numbers
    let year = parts[0].parse::<i32>().ok();
    let month = parts[1].parse::<u32>().ok();
    let day = parts[2].parse::<u32>().ok();
    
    if let (Some(y), Some(m), Some(d)) = (year, month, day) {
        // Basic validation
        (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
    } else {
        false
    }
}
