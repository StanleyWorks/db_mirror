mod config;
mod copy_tables;
mod create_connection;
mod get_db_tables;

use copy_tables::copy_tables;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let start_time = Instant::now();

    match copy_tables().await {
        Ok(_) => {
            let elapsed = start_time.elapsed();
            println!("Database mirror completed in {:.2?}", elapsed)
        }
        Err(err) => eprintln!("{}", err),
    };
}
