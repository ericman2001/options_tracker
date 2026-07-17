use rusqlite::{params, Connection, OptionalExtension, Result};

string_enum! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TradeType {
        Stock => "stock",
        Option => "option",
    }
    error = "trade_type",
}

string_enum! {
    /// Open/close order semantics that apply to both stock and options. On
    /// stock, `SellToOpen`/`BuyToClose` represent opening and covering a short
    /// position. Cash-flow direction depends only on the buy/sell side (see
    /// [`Action::is_buy`]); the open/close distinction is informational.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Action {
        BuyToOpen => "buy_to_open",
        SellToOpen => "sell_to_open",
        BuyToClose => "buy_to_close",
        SellToClose => "sell_to_close",
    }
    error = "action",
}

impl Action {
    /// True for the buy side (cash outflow), false for the sell side (inflow).
    pub fn is_buy(&self) -> bool {
        matches!(self, Action::BuyToOpen | Action::BuyToClose)
    }
}

string_enum! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OptionType {
        Call => "call",
        Put => "put",
    }
    error = "option_type",
}

string_enum! {
    /// Lifecycle of an option position. `Assigned` and `Exercised` are treated
    /// identically for position math (both trigger the compound stock event);
    /// `Closed` and `Expired` produce no linked stock row.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OptionStatus {
        Open => "open",
        Closed => "closed",
        Assigned => "assigned",
        Exercised => "exercised",
        Expired => "expired",
    }
    error = "option_status",
}

impl OptionStatus {
    /// Whether this terminal status generates a linked stock trade at strike.
    pub fn triggers_stock_event(&self) -> bool {
        matches!(self, OptionStatus::Assigned | OptionStatus::Exercised)
    }
}

/// Number of shares represented by a single option contract.
pub const OPTION_MULTIPLIER: f64 = 100.0;

#[derive(Debug, Clone)]
pub struct Trade {
    pub id: Option<i64>,
    pub symbol: String,
    pub trade_type: TradeType,
    pub action: Action,
    pub price: f64,
    pub quantity: f64,
    pub date: String,
    pub fees: f64,
    pub comment: String,
    // Option-only fields; all `None` for plain stock trades.
    pub option_type: Option<OptionType>,
    pub strike: Option<f64>,
    pub expiration: Option<String>,
    pub status: Option<OptionStatus>,
    /// Links an auto-generated stock row back to the option that produced it via
    /// assignment/exercise. `None` for user-entered rows.
    pub assigned_from: Option<i64>,
}

impl Default for Trade {
    fn default() -> Self {
        Trade {
            id: None,
            symbol: String::new(),
            trade_type: TradeType::Stock,
            action: Action::BuyToOpen,
            price: 0.0,
            quantity: 0.0,
            date: String::new(),
            fees: 0.0,
            comment: String::new(),
            option_type: None,
            strike: None,
            expiration: None,
            status: None,
            assigned_from: None,
        }
    }
}

impl Trade {
    /// Shares per unit for this trade: [`OPTION_MULTIPLIER`] for options, 1 for
    /// stock. Used for cash-flow and share-ledger math.
    pub fn multiplier(&self) -> f64 {
        match self.trade_type {
            TradeType::Option => OPTION_MULTIPLIER,
            TradeType::Stock => 1.0,
        }
    }

    /// Signed cash flow of this trade: positive when cash comes in (sell),
    /// negative when cash goes out (buy). Fees always reduce cash.
    pub fn cash_flow(&self) -> f64 {
        let gross = self.price * self.quantity * self.multiplier();
        if self.action.is_buy() {
            -gross - self.fees
        } else {
            gross - self.fees
        }
    }

    /// Signed share count contributed to a symbol's ledger by a stock trade:
    /// positive for buys, negative for sells. Options hold no shares (0).
    pub fn signed_shares(&self) -> f64 {
        if self.trade_type != TradeType::Stock {
            return 0.0;
        }
        if self.action.is_buy() {
            self.quantity
        } else {
            -self.quantity
        }
    }
}

/// Aggregated per-symbol report row.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolReport {
    pub symbol: String,
    pub profit_loss: f64,
    pub trade_count: i32,
    /// Net share position: positive = long, negative = short, 0 = flat.
    pub net_shares: f64,
    /// Break-even price for the current net share position, or `None` when flat.
    pub break_even: Option<f64>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                trade_type TEXT NOT NULL,
                action TEXT NOT NULL,
                price REAL NOT NULL,
                quantity REAL NOT NULL,
                date TEXT NOT NULL,
                fees REAL NOT NULL,
                comment TEXT,
                option_type TEXT,
                strike REAL,
                expiration TEXT,
                status TEXT,
                assigned_from INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_trade(&self, trade: &Trade) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO trades
                (symbol, trade_type, action, price, quantity, date, fees, comment,
                 option_type, strike, expiration, status, assigned_from)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                trade.symbol,
                trade.trade_type,
                trade.action,
                trade.price,
                trade.quantity,
                trade.date,
                trade.fees,
                trade.comment,
                trade.option_type,
                trade.strike,
                trade.expiration,
                trade.status,
                trade.assigned_from,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn row_to_trade(row: &rusqlite::Row<'_>) -> Result<Trade> {
        Ok(Trade {
            id: Some(row.get(0)?),
            symbol: row.get(1)?,
            trade_type: row.get(2)?,
            action: row.get(3)?,
            price: row.get(4)?,
            quantity: row.get(5)?,
            date: row.get(6)?,
            fees: row.get(7)?,
            comment: row.get(8)?,
            option_type: row.get(9)?,
            strike: row.get(10)?,
            expiration: row.get(11)?,
            status: row.get(12)?,
            assigned_from: row.get(13)?,
        })
    }

    const SELECT_COLUMNS: &'static str = "id, symbol, trade_type, action, price, quantity, date, \
         fees, comment, option_type, strike, expiration, status, assigned_from";

    pub fn get_all_trades(&self) -> Result<Vec<Trade>> {
        let sql = format!(
            "SELECT {} FROM trades ORDER BY date DESC, id DESC",
            Self::SELECT_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let trades = stmt.query_map([], Self::row_to_trade)?;
        trades.collect()
    }

    pub fn get_trade(&self, id: i64) -> Result<Option<Trade>> {
        let sql = format!("SELECT {} FROM trades WHERE id = ?1", Self::SELECT_COLUMNS);
        self.conn
            .query_row(&sql, params![id], Self::row_to_trade)
            .optional()
    }

    pub fn update_trade(&self, trade: &Trade) -> Result<()> {
        if let Some(id) = trade.id {
            let tx = self.conn.unchecked_transaction()?;
            self.conn.execute(
                "UPDATE trades
                 SET symbol = ?1, trade_type = ?2, action = ?3, price = ?4,
                     quantity = ?5, date = ?6, fees = ?7, comment = ?8,
                     option_type = ?9, strike = ?10, expiration = ?11,
                     status = ?12, assigned_from = ?13
                 WHERE id = ?14",
                params![
                    trade.symbol,
                    trade.trade_type,
                    trade.action,
                    trade.price,
                    trade.quantity,
                    trade.date,
                    trade.fees,
                    trade.comment,
                    trade.option_type,
                    trade.strike,
                    trade.expiration,
                    trade.status,
                    trade.assigned_from,
                    id,
                ],
            )?;
            // Reconcile auto-generated linked stock rows: clear any existing rows
            // for this option, then regenerate them if the edited option is still
            // in a stock-generating status (Assigned/Exercised). This keeps the
            // linked row's strike/quantity in sync with edits and drops orphans
            // both when the option moves off that status and when its type is
            // changed away from Option.
            self.delete_linked_stock_rows(id)?;
            if trade.trade_type == TradeType::Option {
                if let Some(status) = trade.status.clone() {
                    if status.triggers_stock_event() {
                        self.insert_linked_stock_row(trade, &status)?;
                    }
                }
            }
            tx.commit()?;
        }
        Ok(())
    }

    /// Deletes a trade. When the trade is an option, its auto-generated linked
    /// stock rows are deleted too so the ledger never keeps orphaned assignment
    /// rows.
    pub fn delete_trade(&self, id: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.delete_linked_stock_rows(id)?;
        self.conn
            .execute("DELETE FROM trades WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    fn delete_linked_stock_rows(&self, option_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM trades WHERE assigned_from = ?1",
            params![option_id],
        )?;
        Ok(())
    }

    /// Marks an open option as assigned or exercised and inserts the linked stock
    /// trade at the option's strike. Direction depends on the option's type and
    /// long/short side (short put assigned → buy, short call assigned → sell,
    /// long put exercised → sell, long call exercised → buy), for `qty * 100`
    /// shares. Late reconciliation is allowed — a past expiration does not block
    /// this. No additional option cash flow is recorded; the premium was already
    /// booked when the option was opened.
    pub fn assign_option(&self, option_id: i64, status: OptionStatus) -> Result<i64> {
        if !status.triggers_stock_event() {
            return Err(rusqlite::Error::InvalidParameterName(
                "assign_option requires Assigned or Exercised".to_string(),
            ));
        }
        let option = self
            .get_trade(option_id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;

        let tx = self.conn.unchecked_transaction()?;
        // Replace any previously generated linked rows before regenerating.
        self.delete_linked_stock_rows(option_id)?;
        let stock_id = self.insert_linked_stock_row(&option, &status)?;
        self.conn.execute(
            "UPDATE trades SET status = ?1 WHERE id = ?2",
            params![status, option_id],
        )?;
        tx.commit()?;
        Ok(stock_id)
    }

    /// Inserts the linked stock trade produced by assigning/exercising `option`
    /// at its strike for `qty * 100` shares, tagged with `assigned_from =
    /// option.id`. The buy/sell direction depends on the option type and its
    /// long/short side (see the match below). Returns the new row id. Callers are
    /// responsible for clearing any prior linked rows and for running inside a
    /// transaction alongside the option's status update.
    fn insert_linked_stock_row(&self, option: &Trade, status: &OptionStatus) -> Result<i64> {
        let option_id = option
            .id
            .ok_or_else(|| rusqlite::Error::InvalidParameterName("option has no id".to_string()))?;
        let option_type = option.option_type.clone().ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("trade is not an option".to_string())
        })?;
        let strike = option.strike.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("option has no strike".to_string())
        })?;

        // Share direction depends on both the option type and whether the option
        // was long (bought to open) or short (sold to open):
        //   short put assigned    → buy shares  (put obligates us to buy)
        //   short call assigned   → sell shares (call obligates us to sell)
        //   long put exercised    → sell shares (we exercise our right to sell)
        //   long call exercised   → buy shares  (we exercise our right to buy)
        let stock_action = match (&option_type, option.action.is_buy()) {
            (OptionType::Put, false) => Action::BuyToOpen,
            (OptionType::Call, false) => Action::SellToOpen,
            (OptionType::Put, true) => Action::SellToOpen,
            (OptionType::Call, true) => Action::BuyToOpen,
        };

        let stock = Trade {
            id: None,
            symbol: option.symbol.clone(),
            trade_type: TradeType::Stock,
            action: stock_action,
            price: strike,
            quantity: option.quantity * OPTION_MULTIPLIER,
            date: option.expiration.clone().unwrap_or_else(crate::date::today),
            fees: 0.0,
            comment: format!("Auto: {} {} of option #{}", option_type, status, option_id),
            option_type: None,
            strike: None,
            expiration: None,
            status: None,
            assigned_from: Some(option_id),
        };
        self.add_trade(&stock)
    }

    /// Marks an open option as expired: closes it with no additional cash flow
    /// (the premium was already booked when the option was opened) and removes
    /// any linked stock rows from a prior assignment.
    pub fn expire_option(&self, option_id: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.delete_linked_stock_rows(option_id)?;
        self.conn.execute(
            "UPDATE trades SET status = ?1 WHERE id = ?2",
            params![OptionStatus::Expired, option_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Net signed share position for a symbol (long > 0, short < 0), summed over
    /// stock trades (including assignment-generated rows).
    pub fn net_shares(&self, symbol: &str) -> Result<f64> {
        Ok(self
            .get_all_trades()?
            .iter()
            .filter(|t| t.symbol == symbol)
            .map(Trade::signed_shares)
            .sum())
    }

    /// Break-even price for a symbol's current net share position, derived from
    /// the full ledger: `-(sum of all cash flows) / net_shares`. This folds in
    /// collected option premium and all fees, so it works for both long and
    /// short positions. Returns `None` when the net position is flat.
    pub fn get_break_even(&self, symbol: &str) -> Result<Option<f64>> {
        self.get_break_even_excluding(symbol, None)
    }

    /// Like [`get_break_even`], but ignores the trade whose id equals
    /// `exclude_id` (if any). Used by the covered-call warning when *editing* an
    /// option: the pre-edit version of the option being saved is still in the
    /// ledger, and since option premium folds into break-even it would otherwise
    /// skew the warning threshold. Pass `None` to include every trade.
    pub fn get_break_even_excluding(
        &self,
        symbol: &str,
        exclude_id: Option<i64>,
    ) -> Result<Option<f64>> {
        let trades: Vec<Trade> = self
            .get_all_trades()?
            .into_iter()
            .filter(|t| t.symbol == symbol && (exclude_id.is_none() || t.id != exclude_id))
            .collect();
        let net_shares: f64 = trades.iter().map(Trade::signed_shares).sum();
        if net_shares.abs() < f64::EPSILON {
            return Ok(None);
        }
        let total_cash_flow: f64 = trades.iter().map(Trade::cash_flow).sum();
        Ok(Some(-total_cash_flow / net_shares))
    }

    pub fn get_report_by_symbol(&self) -> Result<Vec<SymbolReport>> {
        let trades = self.get_all_trades()?;
        let mut symbols: Vec<String> = trades.iter().map(|t| t.symbol.clone()).collect();
        symbols.sort();
        symbols.dedup();

        let mut reports = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            let symbol_trades: Vec<&Trade> = trades.iter().filter(|t| t.symbol == symbol).collect();
            let profit_loss: f64 = symbol_trades.iter().map(|t| t.cash_flow()).sum();
            let net_shares: f64 = symbol_trades.iter().map(|t| t.signed_shares()).sum();
            let trade_count = symbol_trades.len() as i32;
            let break_even = self.get_break_even(&symbol)?;
            reports.push(SymbolReport {
                symbol,
                profit_loss,
                trade_count,
                net_shares,
                break_even,
            });
        }
        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_db() -> Database {
        Database::new(":memory:").expect("failed to create in-memory database")
    }

    fn stock(symbol: &str, action: Action, price: f64, quantity: f64, fees: f64) -> Trade {
        Trade {
            symbol: symbol.to_string(),
            trade_type: TradeType::Stock,
            action,
            price,
            quantity,
            date: "2024-01-15".to_string(),
            fees,
            ..Default::default()
        }
    }

    fn option(
        symbol: &str,
        action: Action,
        option_type: OptionType,
        price: f64,
        quantity: f64,
        strike: f64,
        expiration: &str,
    ) -> Trade {
        Trade {
            symbol: symbol.to_string(),
            trade_type: TradeType::Option,
            action,
            price,
            quantity,
            date: "2024-01-15".to_string(),
            fees: 0.0,
            option_type: Some(option_type),
            strike: Some(strike),
            expiration: Some(expiration.to_string()),
            status: Some(OptionStatus::Open),
            ..Default::default()
        }
    }

    #[test]
    fn enum_as_str_and_parse() {
        assert_eq!(TradeType::Option.as_str(), "option");
        assert_eq!(Action::BuyToOpen.as_str(), "buy_to_open");
        assert_eq!(Action::SellToClose.as_str(), "sell_to_close");
        assert_eq!(OptionType::Call.as_str(), "call");
        assert_eq!(OptionStatus::Assigned.as_str(), "assigned");

        assert!(matches!("buy_to_open".parse(), Ok(Action::BuyToOpen)));
        assert!(matches!("SELL_TO_OPEN".parse(), Ok(Action::SellToOpen)));
        assert!(matches!("Put".parse(), Ok(OptionType::Put)));
        assert!(matches!("EXPIRED".parse(), Ok(OptionStatus::Expired)));

        assert!("buy".parse::<Action>().is_err());
        assert!("straddle".parse::<OptionType>().is_err());
        assert!("pending".parse::<OptionStatus>().is_err());
    }

    #[test]
    fn action_is_buy() {
        assert!(Action::BuyToOpen.is_buy());
        assert!(Action::BuyToClose.is_buy());
        assert!(!Action::SellToOpen.is_buy());
        assert!(!Action::SellToClose.is_buy());
    }

    #[test]
    fn option_cash_flow_uses_100x_multiplier() {
        // Sell-to-open a put for $2.00, 1 contract, no fees → +$200 collected.
        let sold_put = option(
            "AAPL",
            Action::SellToOpen,
            OptionType::Put,
            2.0,
            1.0,
            100.0,
            "2024-06-21",
        );
        assert!((sold_put.cash_flow() - 200.0).abs() < 1e-9);
        // Stock keeps a 1x multiplier.
        let bought = stock("AAPL", Action::BuyToOpen, 100.0, 10.0, 0.0);
        assert!((bought.cash_flow() + 1000.0).abs() < 1e-9);
    }

    #[test]
    fn schema_roundtrips_all_option_fields() {
        let db = new_test_db();
        let opt = option(
            "AAPL",
            Action::SellToOpen,
            OptionType::Put,
            2.0,
            1.0,
            100.0,
            "2024-06-21",
        );
        let id = db.add_trade(&opt).unwrap();
        let stored = db.get_trade(id).unwrap().unwrap();
        assert_eq!(stored.symbol, "AAPL");
        assert!(matches!(stored.action, Action::SellToOpen));
        assert_eq!(stored.option_type, Some(OptionType::Put));
        assert_eq!(stored.strike, Some(100.0));
        assert_eq!(stored.expiration, Some("2024-06-21".to_string()));
        assert_eq!(stored.status, Some(OptionStatus::Open));
        assert_eq!(stored.assigned_from, None);
    }

    #[test]
    fn break_even_long_after_put_assignment() {
        let db = new_test_db();
        // Sell a put for $2 premium, then it gets assigned → buy 100 @ 100.
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();

        // Long 100 shares, break-even = 100 - 2 = 98.
        assert!((db.net_shares("AAPL").unwrap() - 100.0).abs() < 1e-9);
        let be = db.get_break_even("AAPL").unwrap().unwrap();
        assert!((be - 98.0).abs() < 1e-9);
    }

    #[test]
    fn put_assignment_creates_long_linked_stock_row() {
        let db = new_test_db();
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                2.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();

        let trades = db.get_all_trades().unwrap();
        let option_row = trades.iter().find(|t| t.id == Some(put_id)).unwrap();
        assert_eq!(option_row.status, Some(OptionStatus::Assigned));

        let linked: Vec<&Trade> = trades
            .iter()
            .filter(|t| t.assigned_from == Some(put_id))
            .collect();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].trade_type, TradeType::Stock);
        assert!(linked[0].action.is_buy());
        assert!((linked[0].quantity - 200.0).abs() < 1e-9); // 2 contracts * 100
        assert!((linked[0].price - 100.0).abs() < 1e-9);
    }

    #[test]
    fn call_assignment_creates_short_linked_stock_row() {
        let db = new_test_db();
        let call_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Call,
                1.0,
                1.0,
                110.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(call_id, OptionStatus::Assigned).unwrap();

        // From flat, an assigned call yields a short position.
        assert!((db.net_shares("AAPL").unwrap() + 100.0).abs() < 1e-9);
        let linked = db
            .get_all_trades()
            .unwrap()
            .into_iter()
            .find(|t| t.assigned_from == Some(call_id))
            .unwrap();
        assert!(!linked.action.is_buy());
    }

    #[test]
    fn long_call_exercise_creates_long_linked_stock_row() {
        let db = new_test_db();
        // Buy a call (long), then exercise it → buy shares at strike.
        let call_id = db
            .add_trade(&option(
                "AAPL",
                Action::BuyToOpen,
                OptionType::Call,
                1.0,
                1.0,
                110.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(call_id, OptionStatus::Exercised).unwrap();

        // Exercising a long call buys shares (from flat: long).
        assert!((db.net_shares("AAPL").unwrap() - 100.0).abs() < 1e-9);
        let linked = db
            .get_all_trades()
            .unwrap()
            .into_iter()
            .find(|t| t.assigned_from == Some(call_id))
            .unwrap();
        assert!(linked.action.is_buy());
        assert!((linked.price - 110.0).abs() < 1e-9);
    }

    #[test]
    fn long_put_exercise_creates_short_linked_stock_row() {
        let db = new_test_db();
        // Buy a put (long), then exercise it → sell shares at strike.
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::BuyToOpen,
                OptionType::Put,
                1.0,
                1.0,
                90.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Exercised).unwrap();

        // Exercising a long put sells shares (from flat: short).
        assert!((db.net_shares("AAPL").unwrap() + 100.0).abs() < 1e-9);
        let linked = db
            .get_all_trades()
            .unwrap()
            .into_iter()
            .find(|t| t.assigned_from == Some(put_id))
            .unwrap();
        assert!(!linked.action.is_buy());
        assert!((linked.price - 90.0).abs() < 1e-9);
    }

    #[test]
    fn assignment_shrinks_existing_long_position() {
        let db = new_test_db();
        // Own 100 shares long, then a covered call gets assigned → sell 100.
        db.add_trade(&stock("AAPL", Action::BuyToOpen, 90.0, 100.0, 0.0))
            .unwrap();
        let call_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Call,
                1.0,
                1.0,
                110.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(call_id, OptionStatus::Assigned).unwrap();
        // 100 long - 100 sold = flat.
        assert!(db.net_shares("AAPL").unwrap().abs() < 1e-9);
        assert_eq!(db.get_break_even("AAPL").unwrap(), None);
    }

    #[test]
    fn deleting_option_cleans_up_linked_stock_rows() {
        let db = new_test_db();
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();
        assert_eq!(db.get_all_trades().unwrap().len(), 2);

        db.delete_trade(put_id).unwrap();
        assert!(db.get_all_trades().unwrap().is_empty());
    }

    #[test]
    fn reverting_assignment_via_expire_removes_linked_row() {
        let db = new_test_db();
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();
        db.expire_option(put_id).unwrap();

        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].status, Some(OptionStatus::Expired));
        assert!(db.net_shares("AAPL").unwrap().abs() < 1e-9);
    }

    #[test]
    fn expired_sold_call_keeps_premium_and_no_stock_row() {
        let db = new_test_db();
        // Sell-to-open a call for $3 premium (1 contract) then it expires worthless.
        let call_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Call,
                3.0,
                1.0,
                110.0,
                "2024-06-21",
            ))
            .unwrap();
        db.expire_option(call_id).unwrap();

        let report = db.get_report_by_symbol().unwrap();
        assert_eq!(report.len(), 1);
        // Premium kept as profit; no linked stock row created.
        assert!((report[0].profit_loss - 300.0).abs() < 1e-9);
        assert!(report[0].net_shares.abs() < 1e-9);
        assert!(db
            .get_all_trades()
            .unwrap()
            .iter()
            .all(|t| t.assigned_from.is_none()));
    }

    #[test]
    fn editing_assigned_option_regenerates_linked_stock_row() {
        let db = new_test_db();
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();

        // Edit the assigned option's strike and quantity while keeping it assigned.
        let mut edited = db.get_trade(put_id).unwrap().unwrap();
        edited.strike = Some(90.0);
        edited.quantity = 2.0;
        db.update_trade(&edited).unwrap();

        let linked: Vec<Trade> = db
            .get_all_trades()
            .unwrap()
            .into_iter()
            .filter(|t| t.assigned_from == Some(put_id))
            .collect();
        assert_eq!(linked.len(), 1);
        assert!((linked[0].price - 90.0).abs() < 1e-9);
        assert!((linked[0].quantity - 200.0).abs() < 1e-9); // 2 contracts * 100
        assert!((db.net_shares("AAPL").unwrap() - 200.0).abs() < 1e-9);
    }

    #[test]
    fn editing_assigned_option_to_stock_removes_linked_rows() {
        let db = new_test_db();
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();
        assert_eq!(db.get_all_trades().unwrap().len(), 2);

        // Change the option row to a plain stock trade (status/option fields cleared).
        let mut edited = db.get_trade(put_id).unwrap().unwrap();
        edited.trade_type = TradeType::Stock;
        edited.option_type = None;
        edited.strike = None;
        edited.expiration = None;
        edited.status = None;
        db.update_trade(&edited).unwrap();

        // No orphaned linked stock row should remain.
        assert!(db
            .get_all_trades()
            .unwrap()
            .iter()
            .all(|t| t.assigned_from.is_none()));
    }

    #[test]
    fn break_even_short_position() {
        let db = new_test_db();
        // Short 100 shares at $50 (no fees). Break-even = 50.
        db.add_trade(&stock("AAPL", Action::SellToOpen, 50.0, 100.0, 0.0))
            .unwrap();
        assert!((db.net_shares("AAPL").unwrap() + 100.0).abs() < 1e-9);
        let be = db.get_break_even("AAPL").unwrap().unwrap();
        assert!((be - 50.0).abs() < 1e-9);
    }

    #[test]
    fn covered_call_below_break_even_detectable() {
        let db = new_test_db();
        // Establish a long at break-even 98 via assigned put.
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();

        let be = db.get_break_even("AAPL").unwrap().unwrap();
        // A call struck at 95 is below break-even (would lock a loss if assigned);
        // one at 105 is safely above.
        assert!(95.0 < be);
        assert!(105.0 > be);
    }

    #[test]
    fn break_even_excluding_ignores_the_named_trade() {
        let db = new_test_db();
        // Long 100 @ $100 via assigned put (premium $2 → break-even 98).
        let put_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Put,
                2.0,
                1.0,
                100.0,
                "2024-06-21",
            ))
            .unwrap();
        db.assign_option(put_id, OptionStatus::Assigned).unwrap();
        // Add an open call whose $5 premium would drag break-even down.
        let call_id = db
            .add_trade(&option(
                "AAPL",
                Action::SellToOpen,
                OptionType::Call,
                5.0,
                1.0,
                105.0,
                "2024-07-19",
            ))
            .unwrap();

        let with_call = db.get_break_even("AAPL").unwrap().unwrap();
        let without_call = db
            .get_break_even_excluding("AAPL", Some(call_id))
            .unwrap()
            .unwrap();
        // Excluding the call's premium raises the break-even back toward 98.
        assert!(without_call > with_call);
        assert!((without_call - 98.0).abs() < 1e-9);
        // Excluding None matches the plain break-even.
        assert_eq!(
            db.get_break_even_excluding("AAPL", None).unwrap(),
            Some(with_call)
        );
    }

    #[test]
    fn report_orders_by_symbol_and_counts_trades() {
        let db = new_test_db();
        db.add_trade(&stock("TSLA", Action::BuyToOpen, 200.0, 1.0, 0.0))
            .unwrap();
        db.add_trade(&stock("AAPL", Action::BuyToOpen, 100.0, 1.0, 0.0))
            .unwrap();
        db.add_trade(&stock("AAPL", Action::SellToClose, 120.0, 1.0, 0.0))
            .unwrap();

        let report = db.get_report_by_symbol().unwrap();
        assert_eq!(report.len(), 2);
        assert_eq!(report[0].symbol, "AAPL");
        assert_eq!(report[0].trade_count, 2);
        assert!((report[0].profit_loss - 20.0).abs() < 1e-9);
        assert_eq!(report[1].symbol, "TSLA");
    }

    #[test]
    fn update_trade_without_id_is_noop() {
        let db = new_test_db();
        db.add_trade(&stock("AAPL", Action::BuyToOpen, 150.0, 10.0, 1.0))
            .unwrap();
        let ghost = stock("ZZZZ", Action::SellToClose, 1.0, 1.0, 0.0);
        db.update_trade(&ghost).unwrap();
        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, "AAPL");
    }

    #[test]
    fn trade_default_values() {
        let trade = Trade::default();
        assert_eq!(trade.id, None);
        assert!(matches!(trade.trade_type, TradeType::Stock));
        assert!(matches!(trade.action, Action::BuyToOpen));
        assert_eq!(trade.option_type, None);
        assert_eq!(trade.status, None);
        assert_eq!(trade.assigned_from, None);
    }
}
