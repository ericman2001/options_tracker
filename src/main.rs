use options_tracker::db::Database;
use options_tracker::ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let db = Database::new("options_tracker.db")?;
    
    // Run UI
    ui::run_ui(db);
    
    Ok(())
}
