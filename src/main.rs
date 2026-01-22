use options_tracker::db::{Database, Trade};
use options_tracker::ui::{App, Screen, InputField};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let db = Database::new("options_tracker.db")?;
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app, &db);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    db: &Database,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| options_tracker::ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.current_screen {
                Screen::MainMenu => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => app.previous_menu_item(),
                        KeyCode::Down => app.next_menu_item(),
                        KeyCode::Enter => {
                            match app.selected_menu_item {
                                0 => {
                                    app.current_screen = Screen::AddTrade;
                                    app.current_trade = Trade::default();
                                    app.current_input_field = InputField::Symbol;
                                    app.input_buffer.clear();
                                    app.message = None;
                                }
                                1 => {
                                    app.trades = db.get_all_trades().unwrap_or_default();
                                    app.selected_trade_index = 0;
                                    app.current_screen = Screen::ViewTrades;
                                }
                                2 => {
                                    app.reports = db.get_report_by_symbol().unwrap_or_default();
                                    app.current_screen = Screen::Reports;
                                }
                                3 => return Ok(()),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                Screen::AddTrade | Screen::EditTrade => {
                    match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::MainMenu;
                            app.input_buffer.clear();
                            app.message = None;
                        }
                        KeyCode::Tab => {
                            // Save current field value before moving to next
                            if !app.input_buffer.is_empty() {
                                update_current_field(app);
                            }
                            app.next_field();
                        }
                        KeyCode::BackTab => {
                            // Save current field value before moving to previous
                            if !app.input_buffer.is_empty() {
                                update_current_field(app);
                            }
                            app.previous_field();
                        }
                        KeyCode::Enter => {
                            // Save current field value
                            if !app.input_buffer.is_empty() {
                                update_current_field(app);
                            }
                            
                            // Validate and save trade
                            if validate_trade(&app.current_trade) {
                                let result = if app.current_screen == Screen::EditTrade {
                                    db.update_trade(&app.current_trade)
                                } else {
                                    db.add_trade(&app.current_trade).map(|_| ())
                                };
                                
                                match result {
                                    Ok(_) => {
                                        app.current_screen = Screen::MainMenu;
                                        app.input_buffer.clear();
                                        app.message = None;
                                    }
                                    Err(e) => {
                                        app.message = Some(format!("Error saving trade: {}", e));
                                    }
                                }
                            } else {
                                app.message = Some("Please fill in all required fields correctly".to_string());
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                            app.message = None;
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        _ => {}
                    }
                }
                Screen::ViewTrades => {
                    match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::MainMenu;
                        }
                        KeyCode::Up => app.previous_trade(),
                        KeyCode::Down => app.next_trade(),
                        KeyCode::Char('e') => {
                            if !app.trades.is_empty() {
                                app.current_trade = app.trades[app.selected_trade_index].clone();
                                app.current_input_field = InputField::Symbol;
                                app.input_buffer.clear();
                                app.current_screen = Screen::EditTrade;
                                app.message = None;
                            }
                        }
                        KeyCode::Char('d') => {
                            if !app.trades.is_empty() {
                                if let Some(id) = app.trades[app.selected_trade_index].id {
                                    let _ = db.delete_trade(id);
                                    app.trades = db.get_all_trades().unwrap_or_default();
                                    if app.selected_trade_index >= app.trades.len() && app.selected_trade_index > 0 {
                                        app.selected_trade_index = app.trades.len() - 1;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Screen::Reports => {
                    if key.code == KeyCode::Esc {
                        app.current_screen = Screen::MainMenu;
                    }
                }
            }
        }
    }
}

fn update_current_field(app: &mut App) {
    match app.current_input_field {
        InputField::Symbol => {
            app.current_trade.symbol = app.input_buffer.clone().to_uppercase();
            app.input_buffer.clear();
        }
        InputField::TradeType => {
            let input = app.input_buffer.to_lowercase();
            if input == "stock" || input == "option" {
                app.current_trade.trade_type = input;
                app.input_buffer.clear();
            }
        }
        InputField::Action => {
            let input = app.input_buffer.to_lowercase();
            if input == "buy" || input == "sell" {
                app.current_trade.action = input;
                app.input_buffer.clear();
            }
        }
        InputField::Price => {
            if let Ok(price) = app.input_buffer.parse::<f64>() {
                app.current_trade.price = price;
                app.input_buffer.clear();
            }
        }
        InputField::Quantity => {
            if let Ok(quantity) = app.input_buffer.parse::<f64>() {
                app.current_trade.quantity = quantity;
                app.input_buffer.clear();
            }
        }
        InputField::Date => {
            app.current_trade.date = app.input_buffer.clone();
            app.input_buffer.clear();
        }
        InputField::Fees => {
            if let Ok(fees) = app.input_buffer.parse::<f64>() {
                app.current_trade.fees = fees;
                app.input_buffer.clear();
            }
        }
        InputField::Comment => {
            app.current_trade.comment = app.input_buffer.clone();
            app.input_buffer.clear();
        }
    }
}

fn validate_trade(trade: &Trade) -> bool {
    !trade.symbol.is_empty()
        && (trade.trade_type == "stock" || trade.trade_type == "option")
        && (trade.action == "buy" || trade.action == "sell")
        && trade.price >= 0.0
        && trade.quantity > 0.0
        && !trade.date.is_empty()
        && trade.fees >= 0.0
}
