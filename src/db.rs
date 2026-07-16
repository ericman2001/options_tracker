use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Connection, Result,
};

#[derive(Debug, Clone)]
pub enum TradeType {
    Stock,
    Option,
}

#[derive(Debug, Clone)]
pub enum Action {
    Buy,
    Sell,
}

impl TradeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeType::Stock => "stock",
            TradeType::Option => "option",
        }
    }

    fn from_str(value: &str) -> Result<Self, FromSqlError> {
        match value {
            "stock" => Ok(TradeType::Stock),
            "option" => Ok(TradeType::Option),
            _ => Err(FromSqlError::Other(Box::from("Invalid trade_type"))),
        }
    }
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Buy => "buy",
            Action::Sell => "sell",
        }
    }

    fn from_str(value: &str) -> Result<Self, FromSqlError> {
        match value {
            "buy" => Ok(Action::Buy),
            "sell" => Ok(Action::Sell),
            _ => Err(FromSqlError::Other(Box::from("Invalid action"))),
        }
    }
}

impl ToSql for TradeType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for TradeType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let value = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                TradeType::from_str(value)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for Action {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for Action {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let value = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                Action::from_str(value)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

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
}

impl Default for Trade {
    fn default() -> Self {
        Trade {
            id: None,
            symbol: String::new(),
            trade_type: TradeType::Stock,
            action: Action::Buy,
            price: 0.0,
            quantity: 0.0,
            date: String::new(),
            fees: 0.0,
            comment: String::new(),
        }
    }
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
                comment TEXT
            )",
            [],
        )?;
        Ok(())
    }

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

    pub fn get_all_trades(&self) -> Result<Vec<Trade>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, symbol, trade_type, action, price, quantity, date, fees, comment
             FROM trades ORDER BY date DESC, id DESC"
        )?;

        let trades = stmt.query_map([], |row| {
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
            })
        })?;

        trades.collect()
    }

    pub fn update_trade(&self, trade: &Trade) -> Result<()> {
        if let Some(id) = trade.id {
            self.conn.execute(
                "UPDATE trades 
                 SET symbol = ?1, trade_type = ?2, action = ?3, price = ?4, 
                     quantity = ?5, date = ?6, fees = ?7, comment = ?8
                 WHERE id = ?9",
                params![
                    trade.symbol,
                    trade.trade_type,
                    trade.action,
                    trade.price,
                    trade.quantity,
                    trade.date,
                    trade.fees,
                    trade.comment,
                    id,
                ],
            )?;
        }
        Ok(())
    }

    pub fn delete_trade(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM trades WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_report_by_symbol(&self) -> Result<Vec<(String, f64, i32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, 
                    SUM(CASE 
                        WHEN action = 'sell' THEN (price * quantity) - fees
                        WHEN action = 'buy' THEN -(price * quantity) - fees
                        ELSE 0 
                    END) as profit_loss,
                    COUNT(*) as trade_count
             FROM trades 
             GROUP BY symbol
             ORDER BY symbol"
        )?;

        let reports = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        })?;

        reports.collect()
    }
}

use std::fmt;

impl fmt::Display for TradeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<TradeType> for String {
    fn from(t: TradeType) -> String {
        t.to_string()
    }
}

impl From<String> for TradeType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "option" => TradeType::Option,
            _ => TradeType::Stock,
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<Action> for String {
    fn from(a: Action) -> String {
        a.to_string()
    }
}

impl From<String> for Action {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "sell" => Action::Sell,
            _ => Action::Buy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trade(symbol: &str, action: Action, price: f64, quantity: f64, fees: f64) -> Trade {
        Trade {
            id: None,
            symbol: symbol.to_string(),
            trade_type: TradeType::Stock,
            action,
            price,
            quantity,
            date: "2024-01-15".to_string(),
            fees,
            comment: String::new(),
        }
    }

    fn new_test_db() -> Database {
        // An in-memory database is isolated per connection and needs no cleanup.
        Database::new(":memory:").expect("failed to create in-memory database")
    }

    #[test]
    fn trade_type_as_str() {
        assert_eq!(TradeType::Stock.as_str(), "stock");
        assert_eq!(TradeType::Option.as_str(), "option");
    }

    #[test]
    fn trade_type_from_str_valid() {
        assert!(matches!(TradeType::from_str("stock"), Ok(TradeType::Stock)));
        assert!(matches!(
            TradeType::from_str("option"),
            Ok(TradeType::Option)
        ));
    }

    #[test]
    fn trade_type_from_str_invalid() {
        assert!(TradeType::from_str("bond").is_err());
        assert!(TradeType::from_str("").is_err());
        assert!(TradeType::from_str("Stock").is_err());
    }

    #[test]
    fn action_as_str() {
        assert_eq!(Action::Buy.as_str(), "buy");
        assert_eq!(Action::Sell.as_str(), "sell");
    }

    #[test]
    fn action_from_str_valid() {
        assert!(matches!(Action::from_str("buy"), Ok(Action::Buy)));
        assert!(matches!(Action::from_str("sell"), Ok(Action::Sell)));
    }

    #[test]
    fn action_from_str_invalid() {
        assert!(Action::from_str("hold").is_err());
        assert!(Action::from_str("").is_err());
        assert!(Action::from_str("Buy").is_err());
    }

    #[test]
    fn trade_type_display() {
        assert_eq!(TradeType::Stock.to_string(), "stock");
        assert_eq!(TradeType::Option.to_string(), "option");
    }

    #[test]
    fn action_display() {
        assert_eq!(Action::Buy.to_string(), "buy");
        assert_eq!(Action::Sell.to_string(), "sell");
    }

    #[test]
    fn trade_type_into_string() {
        let s: String = TradeType::Option.into();
        assert_eq!(s, "option");
    }

    #[test]
    fn action_into_string() {
        let s: String = Action::Sell.into();
        assert_eq!(s, "sell");
    }

    #[test]
    fn trade_type_from_string_is_case_insensitive() {
        assert!(matches!(
            TradeType::from("OPTION".to_string()),
            TradeType::Option
        ));
        assert!(matches!(
            TradeType::from("Option".to_string()),
            TradeType::Option
        ));
        assert!(matches!(
            TradeType::from("stock".to_string()),
            TradeType::Stock
        ));
    }

    #[test]
    fn trade_type_from_string_defaults_to_stock() {
        assert!(matches!(
            TradeType::from("garbage".to_string()),
            TradeType::Stock
        ));
        assert!(matches!(TradeType::from(String::new()), TradeType::Stock));
    }

    #[test]
    fn action_from_string_is_case_insensitive() {
        assert!(matches!(Action::from("SELL".to_string()), Action::Sell));
        assert!(matches!(Action::from("Sell".to_string()), Action::Sell));
        assert!(matches!(Action::from("buy".to_string()), Action::Buy));
    }

    #[test]
    fn action_from_string_defaults_to_buy() {
        assert!(matches!(Action::from("garbage".to_string()), Action::Buy));
        assert!(matches!(Action::from(String::new()), Action::Buy));
    }

    #[test]
    fn trade_default_values() {
        let trade = Trade::default();
        assert_eq!(trade.id, None);
        assert_eq!(trade.symbol, "");
        assert!(matches!(trade.trade_type, TradeType::Stock));
        assert!(matches!(trade.action, Action::Buy));
        assert_eq!(trade.price, 0.0);
        assert_eq!(trade.quantity, 0.0);
        assert_eq!(trade.date, "");
        assert_eq!(trade.fees, 0.0);
        assert_eq!(trade.comment, "");
    }

    #[test]
    fn new_database_starts_empty() {
        let db = new_test_db();
        let trades = db.get_all_trades().unwrap();
        assert!(trades.is_empty());
    }

    #[test]
    fn add_trade_returns_incrementing_ids() {
        let db = new_test_db();
        let id1 = db
            .add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 10.0, 1.0))
            .unwrap();
        let id2 = db
            .add_trade(&sample_trade("TSLA", Action::Buy, 200.0, 5.0, 1.0))
            .unwrap();
        assert!(id2 > id1);
    }

    #[test]
    fn add_and_get_all_trades_roundtrip() {
        let db = new_test_db();
        let mut trade = sample_trade("AAPL", Action::Buy, 150.5, 100.0, 5.0);
        trade.trade_type = TradeType::Option;
        trade.comment = "roundtrip".to_string();
        let id = db.add_trade(&trade).unwrap();

        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades.len(), 1);
        let stored = &trades[0];
        assert_eq!(stored.id, Some(id));
        assert_eq!(stored.symbol, "AAPL");
        assert!(matches!(stored.trade_type, TradeType::Option));
        assert!(matches!(stored.action, Action::Buy));
        assert_eq!(stored.price, 150.5);
        assert_eq!(stored.quantity, 100.0);
        assert_eq!(stored.date, "2024-01-15");
        assert_eq!(stored.fees, 5.0);
        assert_eq!(stored.comment, "roundtrip");
    }

    #[test]
    fn get_all_trades_orders_by_date_desc() {
        let db = new_test_db();
        let mut older = sample_trade("AAPL", Action::Buy, 1.0, 1.0, 0.0);
        older.date = "2024-01-01".to_string();
        let mut newer = sample_trade("TSLA", Action::Buy, 1.0, 1.0, 0.0);
        newer.date = "2024-06-01".to_string();

        db.add_trade(&older).unwrap();
        db.add_trade(&newer).unwrap();

        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades[0].date, "2024-06-01");
        assert_eq!(trades[1].date, "2024-01-01");
    }

    #[test]
    fn update_trade_modifies_existing_row() {
        let db = new_test_db();
        let id = db
            .add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 10.0, 1.0))
            .unwrap();

        let updated = Trade {
            id: Some(id),
            symbol: "MSFT".to_string(),
            trade_type: TradeType::Option,
            action: Action::Sell,
            price: 300.0,
            quantity: 20.0,
            date: "2024-02-02".to_string(),
            fees: 2.5,
            comment: "updated".to_string(),
        };
        db.update_trade(&updated).unwrap();

        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades.len(), 1);
        let stored = &trades[0];
        assert_eq!(stored.symbol, "MSFT");
        assert!(matches!(stored.trade_type, TradeType::Option));
        assert!(matches!(stored.action, Action::Sell));
        assert_eq!(stored.price, 300.0);
        assert_eq!(stored.comment, "updated");
    }

    #[test]
    fn update_trade_without_id_is_noop() {
        let db = new_test_db();
        db.add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 10.0, 1.0))
            .unwrap();

        // id is None, so nothing should be updated.
        let ghost = sample_trade("ZZZZ", Action::Sell, 1.0, 1.0, 0.0);
        db.update_trade(&ghost).unwrap();

        let trades = db.get_all_trades().unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, "AAPL");
    }

    #[test]
    fn delete_trade_removes_row() {
        let db = new_test_db();
        let id = db
            .add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 10.0, 1.0))
            .unwrap();
        db.delete_trade(id).unwrap();
        assert!(db.get_all_trades().unwrap().is_empty());
    }

    #[test]
    fn delete_trade_with_unknown_id_is_noop() {
        let db = new_test_db();
        db.add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 10.0, 1.0))
            .unwrap();
        db.delete_trade(9999).unwrap();
        assert_eq!(db.get_all_trades().unwrap().len(), 1);
    }

    #[test]
    fn report_computes_profit_loss_per_symbol() {
        let db = new_test_db();
        // Buy 100 @ 150 with 5 fees -> -15005
        db.add_trade(&sample_trade("AAPL", Action::Buy, 150.0, 100.0, 5.0))
            .unwrap();
        // Sell 100 @ 165 with 5 fees -> +16495
        db.add_trade(&sample_trade("AAPL", Action::Sell, 165.0, 100.0, 5.0))
            .unwrap();

        let report = db.get_report_by_symbol().unwrap();
        assert_eq!(report.len(), 1);
        let (symbol, profit_loss, count) = &report[0];
        assert_eq!(symbol, "AAPL");
        assert_eq!(*count, 2);
        // (165*100 - 5) - (150*100 + 5) = 16495 - 15005 = 1490
        assert!((profit_loss - 1490.0).abs() < 1e-9);
    }

    #[test]
    fn report_groups_and_sorts_by_symbol() {
        let db = new_test_db();
        db.add_trade(&sample_trade("TSLA", Action::Buy, 200.0, 1.0, 0.0))
            .unwrap();
        db.add_trade(&sample_trade("AAPL", Action::Buy, 100.0, 1.0, 0.0))
            .unwrap();
        db.add_trade(&sample_trade("AAPL", Action::Sell, 120.0, 1.0, 0.0))
            .unwrap();

        let report = db.get_report_by_symbol().unwrap();
        assert_eq!(report.len(), 2);
        // Alphabetical ordering.
        assert_eq!(report[0].0, "AAPL");
        assert_eq!(report[1].0, "TSLA");
        // AAPL: 120 - 100 = 20 across 2 trades.
        assert!((report[0].1 - 20.0).abs() < 1e-9);
        assert_eq!(report[0].2, 2);
        // TSLA: single buy of 200.
        assert!((report[1].1 + 200.0).abs() < 1e-9);
        assert_eq!(report[1].2, 1);
    }

    #[test]
    fn report_is_empty_without_trades() {
        let db = new_test_db();
        assert!(db.get_report_by_symbol().unwrap().is_empty());
    }
}
