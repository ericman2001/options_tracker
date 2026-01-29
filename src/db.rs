use rusqlite::{params, Connection, Result};

#[derive(Debug, Clone)]
pub struct Trade {
    pub id: Option<i64>,
    pub symbol: String,
    pub trade_type: String, // "stock" or "option"
    pub action: String,     // "buy" or "sell"
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
