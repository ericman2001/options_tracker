use crate::date::{days_to_expiration, format_dte, today};
use crate::db::{Action, Database, OptionStatus, OptionType, Trade, TradeType};
use cursive::align::HAlign;
use cursive::theme::{Color, PaletteColor};
use cursive::traits::*;
use cursive::views::{Dialog, EditView, LinearLayout, ListView, SelectView, TextView};
use cursive::Cursive;
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

    let mut select = SelectView::new().h_align(HAlign::Center);

    select.add_item("Add New Trade", 1);
    select.add_item("View/Edit Trades", 2);
    select.add_item("View Reports", 3);
    select.add_item("Quit", 4);

    let db_clone = db.clone();
    select.set_on_submit(move |s, item: &i32| match item {
        1 => show_add_trade(s, db_clone.clone(), None),
        2 => show_view_trades(s, db_clone.clone()),
        3 => show_reports(s, db_clone.clone()),
        4 => s.quit(),
        _ => {}
    });

    siv.add_layer(
        Dialog::around(select.scrollable().fixed_size((40, 10)))
            .title("Stock Options Tracker")
            .button("Quit", |s| s.quit()),
    );

    // Surface a non-blocking alert about options past expiration that are still
    // open, so the user can go resolve them.
    if let Ok(unresolved) = db.lock().expect("Failed to lock database").get_all_trades() {
        maybe_show_expiration_alert(siv, &unresolved);
    }
}

fn show_add_trade(siv: &mut Cursive, db: Arc<Mutex<Database>>, trade: Option<Trade>) {
    let is_edit = trade.is_some();
    let title = if is_edit {
        "Edit Trade"
    } else {
        "Add New Trade"
    };

    let trade = trade.unwrap_or_default();

    let form = ListView::new()
        .child(
            "Symbol:",
            EditView::new()
                .content(trade.symbol.clone())
                .with_name("symbol")
                .fixed_width(20),
        )
        .child(
            "Type (stock/option):",
            EditView::new()
                .content(trade.trade_type.to_string())
                .with_name("trade_type")
                .fixed_width(20),
        )
        .child(
            "Action:",
            EditView::new()
                .content(trade.action.to_string())
                .with_name("action")
                .fixed_width(20),
        )
        .child(
            "Price:",
            EditView::new()
                .content(format_amount(trade.price))
                .with_name("price")
                .fixed_width(20),
        )
        .child(
            "Quantity:",
            EditView::new()
                .content(format_amount(trade.quantity))
                .with_name("quantity")
                .fixed_width(20),
        )
        .child(
            "Date (YYYY-MM-DD):",
            EditView::new()
                .content(trade.date.clone())
                .with_name("date")
                .fixed_width(20),
        )
        .child(
            "Fees:",
            EditView::new()
                .content(format_amount(trade.fees))
                .with_name("fees")
                .fixed_width(20),
        )
        .child(
            "Option Type (call/put):",
            EditView::new()
                .content(
                    trade
                        .option_type
                        .as_ref()
                        .map(|t| t.to_string())
                        .unwrap_or_default(),
                )
                .with_name("option_type")
                .fixed_width(20),
        )
        .child(
            "Strike:",
            EditView::new()
                .content(trade.strike.map(format_amount).unwrap_or_default())
                .with_name("strike")
                .fixed_width(20),
        )
        .child(
            "Expiration (YYYY-MM-DD):",
            EditView::new()
                .content(trade.expiration.clone().unwrap_or_default())
                .with_name("expiration")
                .fixed_width(20),
        )
        .child(
            "Comment:",
            EditView::new()
                .content(trade.comment.clone())
                .with_name("comment")
                .fixed_width(20),
        );

    let trade_id = trade.id;
    let existing_status = trade.status.clone();
    let existing_assigned_from = trade.assigned_from;
    let db_clone = db.clone();

    let help = TextView::new(
        "Action: buy_to_open, sell_to_open, buy_to_close, sell_to_close\n\
         Option Type / Strike / Expiration are required only when Type is 'option'.",
    );
    let body = LinearLayout::vertical()
        .child(help)
        .child(form.scrollable().fixed_size((56, 18)));

    siv.add_layer(
        Dialog::around(body)
            .title(title)
            .button("Save", move |s| {
                let parsed = match read_and_validate_form(s) {
                    Some(p) => p,
                    None => return,
                };

                let status = if parsed.trade_type == TradeType::Option {
                    // Preserve an existing option's lifecycle status on edit;
                    // new options start Open.
                    Some(existing_status.clone().unwrap_or(OptionStatus::Open))
                } else {
                    None
                };

                let new_trade = Trade {
                    id: trade_id,
                    symbol: parsed.symbol.clone(),
                    trade_type: parsed.trade_type.clone(),
                    action: parsed.action.clone(),
                    price: parsed.price,
                    quantity: parsed.quantity,
                    date: parsed.date,
                    fees: parsed.fees,
                    comment: parsed.comment,
                    option_type: parsed.option_type.clone(),
                    strike: parsed.strike,
                    expiration: parsed.expiration,
                    status,
                    assigned_from: existing_assigned_from,
                };

                // Covered-call warning: writing a call below the underlying's
                // break-even would lock in a loss if assigned. Warn (do not
                // block) and let the user confirm.
                if matches!(parsed.action, Action::SellToOpen)
                    && parsed.trade_type == TradeType::Option
                    && parsed.option_type == Some(OptionType::Call)
                {
                    let break_even = db_clone
                        .lock()
                        .expect("Failed to lock database")
                        .get_break_even(&parsed.symbol)
                        .ok()
                        .flatten();
                    if let (Some(be), Some(strike)) = (break_even, parsed.strike) {
                        if strike < be {
                            let db_inner = db_clone.clone();
                            let trade_inner = new_trade.clone();
                            s.add_layer(
                                Dialog::text(format!(
                                    "Warning: strike ${:.2} is below the {} break-even of ${:.2}. \
                                     If assigned, this covered call locks in a loss.",
                                    strike, parsed.symbol, be
                                ))
                                .title("Covered call below break-even")
                                .button("Save Anyway", move |s| {
                                    s.pop_layer();
                                    persist_trade(s, &db_inner, &trade_inner);
                                })
                                .button("Cancel", |s| {
                                    s.pop_layer();
                                }),
                            );
                            return;
                        }
                    }
                }

                persist_trade(s, &db_clone, &new_trade);
            })
            .button("Cancel", move |s| {
                s.pop_layer();
            }),
    );
}

// Values read from the Add/Edit form after validation.
struct ParsedTrade {
    symbol: String,
    trade_type: TradeType,
    action: Action,
    price: f64,
    quantity: f64,
    date: String,
    fees: f64,
    comment: String,
    option_type: Option<OptionType>,
    strike: Option<f64>,
    expiration: Option<String>,
}

// Reads and validates every form field, showing an error dialog and returning
// None on the first problem.
fn read_and_validate_form(s: &mut Cursive) -> Option<ParsedTrade> {
    let read_field = |s: &mut Cursive, name: &str| {
        s.call_on_name(name, |view: &mut EditView| view.get_content().to_string())
    };

    let fields = (|| {
        Some((
            read_field(s, "symbol")?,
            read_field(s, "trade_type")?,
            read_field(s, "action")?,
            read_field(s, "price")?,
            read_field(s, "quantity")?,
            read_field(s, "date")?,
            read_field(s, "fees")?,
            read_field(s, "option_type")?,
            read_field(s, "strike")?,
            read_field(s, "expiration")?,
            read_field(s, "comment")?,
        ))
    })();

    let (
        symbol,
        trade_type,
        action,
        price_str,
        quantity_str,
        date,
        fees_str,
        option_type_str,
        strike_str,
        expiration_str,
        comment,
    ) = match fields {
        Some(values) => values,
        None => {
            s.add_layer(Dialog::info(
                "Internal error: could not read one or more form fields",
            ));
            return None;
        }
    };

    let symbol = symbol.to_uppercase();
    if symbol.is_empty() {
        s.add_layer(Dialog::info("Symbol is required"));
        return None;
    }

    let trade_type = match trade_type.parse::<TradeType>() {
        Ok(t) => t,
        Err(_) => {
            s.add_layer(Dialog::info("Type must be 'stock' or 'option'"));
            return None;
        }
    };

    let action = match action.parse::<Action>() {
        Ok(a) => a,
        Err(_) => {
            s.add_layer(Dialog::info(
                "Action must be one of: buy_to_open, sell_to_open, buy_to_close, sell_to_close",
            ));
            return None;
        }
    };

    let price = parse_amount(s, &price_str, "price", true)?;
    let quantity = parse_amount(s, &quantity_str, "quantity", false)?;
    let fees = parse_amount(s, &fees_str, "fees", true)?;

    if date.is_empty() {
        s.add_layer(Dialog::info("Date is required"));
        return None;
    }
    if !is_valid_date_format(&date) {
        s.add_layer(Dialog::info("Invalid date format. Use YYYY-MM-DD"));
        return None;
    }

    // Option-specific fields are required (and validated) only for options.
    let (option_type, strike, expiration) = if trade_type == TradeType::Option {
        let option_type = match option_type_str.parse::<OptionType>() {
            Ok(t) => t,
            Err(_) => {
                s.add_layer(Dialog::info("Option Type must be 'call' or 'put'"));
                return None;
            }
        };
        let strike = parse_amount(s, &strike_str, "strike", false)?;
        if expiration_str.is_empty() {
            s.add_layer(Dialog::info("Expiration is required for options"));
            return None;
        }
        if !is_valid_date_format(&expiration_str) {
            s.add_layer(Dialog::info("Invalid expiration format. Use YYYY-MM-DD"));
            return None;
        }
        (Some(option_type), Some(strike), Some(expiration_str))
    } else {
        (None, None, None)
    };

    Some(ParsedTrade {
        symbol,
        trade_type,
        action,
        price,
        quantity,
        date,
        fees,
        comment,
        option_type,
        strike,
        expiration,
    })
}

// Adds or updates a trade, then shows a confirmation dialog (or an error).
fn persist_trade(s: &mut Cursive, db: &Arc<Mutex<Database>>, trade: &Trade) {
    let result = if trade.id.is_some() {
        db.lock()
            .expect("Failed to lock database")
            .update_trade(trade)
    } else {
        db.lock()
            .expect("Failed to lock database")
            .add_trade(trade)
            .map(|_| ())
    };

    match result {
        Ok(_) => {
            s.pop_layer();
            s.add_layer(Dialog::text("Trade saved successfully!").button("OK", |s| {
                s.pop_layer();
            }));
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error: {}", e)));
        }
    }
}

fn show_view_trades(siv: &mut Cursive, db: Arc<Mutex<Database>>) {
    let trades = match db.lock().expect("Failed to lock database").get_all_trades() {
        Ok(trades) => trades,
        Err(e) => {
            show_dialog_with_back(siv, format!("Database error: {}", e));
            return;
        }
    };

    if trades.is_empty() {
        show_dialog_with_back(siv, "No trades found".to_string());
        return;
    }

    let now = today();
    let mut select = SelectView::new().h_align(HAlign::Left);

    for trade in trades.iter() {
        select.add_item(format_trade_row(trade, &now), trade.clone());
    }

    let db_clone = db.clone();
    select.set_on_submit(move |s, trade: &Trade| {
        show_trade_actions(s, db_clone.clone(), trade.clone());
    });

    siv.add_layer(
        Dialog::around(select.scrollable().scroll_x(true).fixed_size((90, 20)))
            .title("View/Edit Trades")
            .button("Back", |s| {
                s.pop_layer();
            }),
    );

    maybe_show_expiration_alert(siv, &trades);
}

// Builds the per-trade action dialog (lifecycle actions for open options,
// edit/delete otherwise). Linked auto-generated stock rows are read-only.
fn show_trade_actions(siv: &mut Cursive, db: Arc<Mutex<Database>>, trade: Trade) {
    if let Some(option_id) = trade.assigned_from {
        siv.add_layer(
            Dialog::text(format!(
                "This stock row was auto-generated by the assignment/exercise of \
                 option #{}. Manage or remove it via that option.",
                option_id
            ))
            .button("Back", |s| {
                s.pop_layer();
            }),
        );
        return;
    }

    let mut dialog = Dialog::text("What would you like to do?");

    let is_open_option =
        trade.trade_type == TradeType::Option && trade.status == Some(OptionStatus::Open);
    if is_open_option {
        for (label, status) in [
            ("Assign", OptionStatus::Assigned),
            ("Exercise", OptionStatus::Exercised),
        ] {
            let db = db.clone();
            let id = trade.id;
            dialog = dialog.button(label, move |s| {
                if let Some(id) = id {
                    let res = db
                        .lock()
                        .expect("Failed to lock database")
                        .assign_option(id, status.clone());
                    match res {
                        Ok(_) => {
                            s.pop_layer();
                            s.pop_layer();
                            show_view_trades(s, db.clone());
                        }
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Error: {}", e)));
                        }
                    }
                }
            });
        }
        let db_expire = db.clone();
        let expire_id = trade.id;
        dialog = dialog.button("Expire", move |s| {
            if let Some(id) = expire_id {
                // Bind the result so the database lock is released before we
                // rebuild the trade list (which re-locks the same Mutex).
                let res = db_expire
                    .lock()
                    .expect("Failed to lock database")
                    .expire_option(id);
                match res {
                    Ok(_) => {
                        s.pop_layer();
                        s.pop_layer();
                        show_view_trades(s, db_expire.clone());
                    }
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Error: {}", e)));
                    }
                }
            }
        });
    }

    let db_edit = db.clone();
    let trade_edit = trade.clone();
    dialog = dialog.button("Edit", move |s| {
        s.pop_layer();
        show_add_trade(s, db_edit.clone(), Some(trade_edit.clone()));
    });

    let db_delete = db.clone();
    let delete_id = trade.id;
    dialog = dialog.button("Delete", move |s| {
        if let Some(id) = delete_id {
            // Release the lock before rebuilding the list (see Expire above).
            let res = db_delete
                .lock()
                .expect("Failed to lock database")
                .delete_trade(id);
            match res {
                Ok(_) => {
                    s.pop_layer();
                    s.pop_layer();
                    show_view_trades(s, db_delete.clone());
                }
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Error deleting trade: {}", e)));
                }
            }
        }
    });

    dialog = dialog.button("Cancel", |s| {
        s.pop_layer();
    });

    siv.add_layer(dialog);
}

fn show_reports(siv: &mut Cursive, db: Arc<Mutex<Database>>) {
    let reports = match db
        .lock()
        .expect("Failed to lock database")
        .get_report_by_symbol()
    {
        Ok(reports) => reports,
        Err(e) => {
            show_dialog_with_back(siv, format!("Database error: {}", e));
            return;
        }
    };

    if reports.is_empty() {
        show_dialog_with_back(siv, "No trades found".to_string());
        return;
    }

    let mut content = String::new();
    content.push_str(&format!(
        "{:<8} {:>14} {:>7} {:>14} {:>12}\n",
        "Symbol", "Profit/Loss", "Trades", "Net Position", "Break-Even"
    ));
    content.push_str(&"=".repeat(60));
    content.push('\n');

    for report in reports {
        content.push_str(&format!(
            "{:<8} {:>14} {:>7} {:>14} {:>12}\n",
            report.symbol,
            format!("${:.2}", report.profit_loss),
            report.trade_count,
            format_position(report.net_shares),
            report
                .break_even
                .map(|b| format!("${:.2}", b))
                .unwrap_or_else(|| "-".to_string()),
        ));
    }

    siv.add_layer(
        Dialog::around(TextView::new(content))
            .title("Profit/Loss Report by Symbol")
            .button("Back", |s| {
                s.pop_layer();
            }),
    );
}

// Formats one row of the trade list, including option details and DTE.
fn format_trade_row(trade: &Trade, today: &str) -> String {
    let base = format!(
        "#{:<4} {:<6} {:<7} {:<13} ${:<8.2} x{:<6.2} {} fee ${:.2}",
        trade.id.unwrap_or(0),
        trade.symbol,
        trade.trade_type.as_str(),
        trade.action.as_str(),
        trade.price,
        trade.quantity,
        trade.date,
        trade.fees,
    );

    if trade.trade_type == TradeType::Option {
        let option_type = trade
            .option_type
            .as_ref()
            .map(|t| t.as_str().to_uppercase())
            .unwrap_or_default();
        let strike = trade
            .strike
            .map(|s| format!("${:.2}", s))
            .unwrap_or_else(|| "?".to_string());
        let expiration = trade.expiration.clone().unwrap_or_default();
        let status = trade.status.as_ref().map(|s| s.as_str()).unwrap_or("open");
        let dte = trade
            .expiration
            .as_ref()
            .and_then(|exp| days_to_expiration(today, exp))
            .map(format_dte)
            .unwrap_or_default();

        let mut extra = format!(" [{} {} exp {} {}", option_type, strike, expiration, status);
        if !dte.is_empty() {
            extra.push_str(&format!(", {}", dte));
        }
        extra.push(']');

        // Flag an open option whose expiration has passed.
        if status == "open" {
            if let Some(exp) = trade.expiration.as_ref() {
                if exp.as_str() < today {
                    extra.push_str(" <-- UNRESOLVED, past expiration");
                }
            }
        }
        format!("{}{}", base, extra)
    } else if let Some(option_id) = trade.assigned_from {
        format!("{}  (auto from option #{})", base, option_id)
    } else {
        base
    }
}

// Open options whose expiration has already passed.
fn unresolved_expirations<'a>(trades: &'a [Trade], today: &str) -> Vec<&'a Trade> {
    trades
        .iter()
        .filter(|t| {
            t.trade_type == TradeType::Option
                && t.status == Some(OptionStatus::Open)
                && t.expiration
                    .as_ref()
                    .map(|exp| exp.as_str() < today)
                    .unwrap_or(false)
        })
        .collect()
}

// If any open option has passed its expiration, layer a non-blocking alert on
// top prompting the user to resolve it (mark expired, assign, or exercise).
fn maybe_show_expiration_alert(siv: &mut Cursive, trades: &[Trade]) {
    let now = today();
    let unresolved = unresolved_expirations(trades, &now);
    if unresolved.is_empty() {
        return;
    }

    let mut msg = format!(
        "{} open option(s) are past expiration and need to be resolved \
         (mark Expired, or record Assignment/Exercise):\n\n",
        unresolved.len()
    );
    for t in unresolved {
        msg.push_str(&format!(
            "  #{} {} {} strike {} exp {}\n",
            t.id.unwrap_or(0),
            t.symbol,
            t.option_type
                .as_ref()
                .map(|o| o.as_str())
                .unwrap_or("option"),
            t.strike.map(|s| format!("${:.2}", s)).unwrap_or_default(),
            t.expiration.clone().unwrap_or_default(),
        ));
    }

    siv.add_layer(
        Dialog::around(TextView::new(msg))
            .title("Unresolved expirations")
            .button("OK", |s| {
                s.pop_layer();
            }),
    );
}

// Formats a monetary amount for an edit field, leaving it blank for
// non-positive values.
fn format_amount(value: f64) -> String {
    if value > 0.0 {
        format!("{:.2}", value)
    } else {
        String::new()
    }
}

// Describes a net share position as long/short/flat.
fn format_position(net_shares: f64) -> String {
    if net_shares.abs() < f64::EPSILON {
        "flat".to_string()
    } else if net_shares > 0.0 {
        format!("long {:.0}", net_shares)
    } else {
        format!("short {:.0}", net_shares.abs())
    }
}

// Parses a user-entered f64, showing an error dialog and returning None when
// the input is invalid. When allow_zero is false the value must be strictly
// positive.
fn parse_amount(siv: &mut Cursive, raw: &str, label: &str, allow_zero: bool) -> Option<f64> {
    match raw.parse::<f64>() {
        Ok(value) if value > 0.0 || (allow_zero && value == 0.0) => Some(value),
        _ => {
            siv.add_layer(Dialog::info(format!("Invalid {}", label)));
            None
        }
    }
}

// Shows an informational dialog with a single "Back" button that pops itself.
fn show_dialog_with_back(siv: &mut Cursive, message: String) {
    siv.add_layer(Dialog::text(message).button("Back", |s| {
        s.pop_layer();
    }));
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
        if !(1900..=2100).contains(&y) || !(1..=12).contains(&m) {
            return false;
        }
        (1..=days_in_month(y, m)).contains(&d)
    } else {
        false
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_dates() {
        assert!(is_valid_date_format("2024-01-15"));
        assert!(is_valid_date_format("1900-01-01"));
        assert!(is_valid_date_format("2100-12-31"));
        assert!(is_valid_date_format("2024-02-29")); // leap year
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(!is_valid_date_format(""));
        assert!(!is_valid_date_format("2024-1-1"));
        assert!(!is_valid_date_format("2024-01-155"));
    }

    #[test]
    fn rejects_wrong_separator_count() {
        // Correct length but not three dash-separated parts.
        assert!(!is_valid_date_format("2024/01/15"));
        assert!(!is_valid_date_format("20240115xx"));
    }

    #[test]
    fn rejects_non_numeric_parts() {
        assert!(!is_valid_date_format("abcd-01-15"));
        assert!(!is_valid_date_format("2024-ab-15"));
        assert!(!is_valid_date_format("2024-01-cd"));
    }

    #[test]
    fn rejects_out_of_range_year() {
        assert!(!is_valid_date_format("1899-01-01"));
        assert!(!is_valid_date_format("2101-01-01"));
    }

    #[test]
    fn rejects_out_of_range_month() {
        assert!(!is_valid_date_format("2024-00-15"));
        assert!(!is_valid_date_format("2024-13-15"));
    }

    #[test]
    fn rejects_out_of_range_day() {
        assert!(!is_valid_date_format("2024-01-00"));
        assert!(!is_valid_date_format("2024-01-32"));
    }

    #[test]
    fn rejects_impossible_days() {
        assert!(!is_valid_date_format("2024-02-31"));
        assert!(!is_valid_date_format("2023-02-29")); // non-leap year
        assert!(!is_valid_date_format("2024-04-31"));
    }

    #[test]
    fn position_labels() {
        assert_eq!(format_position(0.0), "flat");
        assert_eq!(format_position(100.0), "long 100");
        assert_eq!(format_position(-200.0), "short 200");
    }

    #[test]
    fn unresolved_expirations_flags_only_past_open_options() {
        let mut open_past = Trade {
            trade_type: TradeType::Option,
            option_type: Some(OptionType::Put),
            strike: Some(100.0),
            expiration: Some("2020-01-01".to_string()),
            status: Some(OptionStatus::Open),
            ..Default::default()
        };
        open_past.id = Some(1);

        let mut open_future = open_past.clone();
        open_future.id = Some(2);
        open_future.expiration = Some("2999-01-01".to_string());

        let mut closed_past = open_past.clone();
        closed_past.id = Some(3);
        closed_past.status = Some(OptionStatus::Expired);

        let trades = vec![open_past, open_future, closed_past];
        let unresolved = unresolved_expirations(&trades, "2024-01-01");
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].id, Some(1));
    }
}
