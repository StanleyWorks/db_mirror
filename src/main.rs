use std::io::Write;
mod config;
mod copy_tables;
mod create_connection;
mod get_db_tables;

use copy_tables::copy_tables;
use env_logger::{Builder, Env};
use log::{info, warn};
use std::time::Instant;

fn main() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}]: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.args()
            )
        })
        .init();

    // Now start the runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main());
}

async fn async_main() {
    use log::{info, warn};
    let start_time = Instant::now();

    info!("App started");

    match copy_tables().await {
        Ok(_) => {
            let elapsed = start_time.elapsed();
            info!("Database mirror completed in {:.2?}", elapsed)
        }
        Err(err) => warn!("{}", err),
    };
}
