use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};
use crate::db::Trade;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    AddTrade,
    ViewTrades,
    EditTrade,
    Reports,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputField {
    Symbol,
    TradeType,
    Action,
    Price,
    Quantity,
    Date,
    Fees,
    Comment,
}

pub struct App {
    pub current_screen: Screen,
    pub selected_menu_item: usize,
    pub selected_trade_index: usize,
    pub trades: Vec<Trade>,
    pub current_trade: Trade,
    pub current_input_field: InputField,
    pub input_buffer: String,
    pub message: Option<String>,
    pub reports: Vec<(String, f64, i32)>,
}

impl App {
    pub fn new() -> Self {
        App {
            current_screen: Screen::MainMenu,
            selected_menu_item: 0,
            selected_trade_index: 0,
            trades: Vec::new(),
            current_trade: Trade::default(),
            current_input_field: InputField::Symbol,
            input_buffer: String::new(),
            message: None,
            reports: Vec::new(),
        }
    }

    pub fn next_menu_item(&mut self) {
        self.selected_menu_item = (self.selected_menu_item + 1) % 4;
    }

    pub fn previous_menu_item(&mut self) {
        if self.selected_menu_item == 0 {
            self.selected_menu_item = 3;
        } else {
            self.selected_menu_item -= 1;
        }
    }

    pub fn next_field(&mut self) {
        self.current_input_field = match self.current_input_field {
            InputField::Symbol => InputField::TradeType,
            InputField::TradeType => InputField::Action,
            InputField::Action => InputField::Price,
            InputField::Price => InputField::Quantity,
            InputField::Quantity => InputField::Date,
            InputField::Date => InputField::Fees,
            InputField::Fees => InputField::Comment,
            InputField::Comment => InputField::Symbol,
        };
    }

    pub fn previous_field(&mut self) {
        self.current_input_field = match self.current_input_field {
            InputField::Symbol => InputField::Comment,
            InputField::TradeType => InputField::Symbol,
            InputField::Action => InputField::TradeType,
            InputField::Price => InputField::Action,
            InputField::Quantity => InputField::Price,
            InputField::Date => InputField::Quantity,
            InputField::Fees => InputField::Date,
            InputField::Comment => InputField::Fees,
        };
    }

    pub fn next_trade(&mut self) {
        if !self.trades.is_empty() {
            self.selected_trade_index = (self.selected_trade_index + 1) % self.trades.len();
        }
    }

    pub fn previous_trade(&mut self) {
        if !self.trades.is_empty() {
            if self.selected_trade_index == 0 {
                self.selected_trade_index = self.trades.len() - 1;
            } else {
                self.selected_trade_index -= 1;
            }
        }
    }
}

impl Trade {
    pub fn default() -> Self {
        Trade {
            id: None,
            symbol: String::new(),
            trade_type: String::from("stock"),
            action: String::from("buy"),
            price: 0.0,
            quantity: 0.0,
            date: String::new(),
            fees: 0.0,
            comment: String::new(),
        }
    }
}

pub fn render_main_menu(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Stock Options Tracker")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let menu_items = vec![
        "Add New Trade",
        "View/Edit Trades",
        "View Reports",
        "Quit",
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let style = if i == app.selected_menu_item {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(item).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"));
    f.render_widget(list, chunks[1]);

    let help = Paragraph::new("↑/↓: Navigate | Enter: Select | q: Quit")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

pub fn render_add_trade(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Add New Trade")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let form_area = chunks[1];
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
        ])
        .split(form_area);

    let fields = [
        ("Symbol", &app.current_trade.symbol, InputField::Symbol),
        ("Type (stock/option)", &app.current_trade.trade_type, InputField::TradeType),
        ("Action (buy/sell)", &app.current_trade.action, InputField::Action),
        ("Price", &format!("{:.2}", app.current_trade.price), InputField::Price),
        ("Quantity", &format!("{:.2}", app.current_trade.quantity), InputField::Quantity),
        ("Date (YYYY-MM-DD)", &app.current_trade.date, InputField::Date),
        ("Fees", &format!("{:.2}", app.current_trade.fees), InputField::Fees),
    ];

    for (i, (label, value, field)) in fields.iter().enumerate() {
        let is_selected = *field == app.current_input_field;
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let display_value = if is_selected && !app.input_buffer.is_empty() {
            &app.input_buffer
        } else {
            value
        };

        let text = format!("{}: {}", label, display_value);
        let paragraph = Paragraph::new(text)
            .style(style)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(paragraph, form_chunks[i]);
    }

    // Comment field
    let is_selected = app.current_input_field == InputField::Comment;
    let style = if is_selected {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let display_value = if is_selected && !app.input_buffer.is_empty() {
        &app.input_buffer
    } else {
        &app.current_trade.comment
    };

    let comment_text = format!("Comment: {}", display_value);
    let comment = Paragraph::new(comment_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(comment, form_chunks[7]);

    let help_text = if let Some(msg) = &app.message {
        msg.clone()
    } else {
        "Tab/Shift+Tab: Navigate | Type to edit | Enter: Save | Esc: Cancel".to_string()
    };
    
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

pub fn render_view_trades(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("View/Edit Trades")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    if app.trades.is_empty() {
        let empty = Paragraph::new("No trades found. Press 'a' to add a new trade.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Trades"));
        f.render_widget(empty, chunks[1]);
    } else {
        let header = Row::new(vec!["ID", "Symbol", "Type", "Action", "Price", "Qty", "Date", "Fees"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = app.trades.iter().enumerate().map(|(i, trade)| {
            let style = if i == app.selected_trade_index {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                trade.id.map_or("N/A".to_string(), |id| id.to_string()),
                trade.symbol.clone(),
                trade.trade_type.clone(),
                trade.action.clone(),
                format!("{:.2}", trade.price),
                format!("{:.2}", trade.quantity),
                trade.date.clone(),
                format!("{:.2}", trade.fees),
            ]).style(style)
        }).collect();

        let widths = [
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(8),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Trades"));
        f.render_widget(table, chunks[1]);
    }

    let help = Paragraph::new("↑/↓: Navigate | e: Edit | d: Delete | Esc: Back")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

pub fn render_edit_trade(f: &mut Frame, app: &App) {
    // Reuse the add trade UI for editing
    render_add_trade(f, app);
}

pub fn render_reports(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Profit/Loss Report by Symbol")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    if app.reports.is_empty() {
        let empty = Paragraph::new("No trades found.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Reports"));
        f.render_widget(empty, chunks[1]);
    } else {
        let header = Row::new(vec!["Symbol", "Profit/Loss", "# Trades"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = app.reports.iter().map(|(symbol, profit_loss, count)| {
            let style = if *profit_loss >= 0.0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            Row::new(vec![
                symbol.clone(),
                format!("${:.2}", profit_loss),
                count.to_string(),
            ]).style(style)
        }).collect();

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Reports"));
        f.render_widget(table, chunks[1]);
    }

    let help = Paragraph::new("Esc: Back to Main Menu")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

pub fn render(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::MainMenu => render_main_menu(f, app),
        Screen::AddTrade => render_add_trade(f, app),
        Screen::ViewTrades => render_view_trades(f, app),
        Screen::EditTrade => render_edit_trade(f, app),
        Screen::Reports => render_reports(f, app),
    }
}
