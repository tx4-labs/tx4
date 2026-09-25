//! TX4 CLI binary.
//!
//! Phase-1C: `migrate` only. No business commands.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("migrate") => match migrate().await {
            Ok(()) => {
                eprintln!("migrate: ok");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("migrate: {err}");
                ExitCode::FAILURE
            }
        },
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: tx4-cli migrate");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: tx4-cli migrate");
            ExitCode::from(2)
        }
    }
}

async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let config = tx4_config::Config::from_env()?;
    let pool = tx4_persistence::connect_pool(&config).await?;
    match tx4_persistence::run_migrations(&pool).await {
        Ok(()) => {
            tx4_persistence::close_pool(&pool).await;
            Ok(())
        }
        Err(err) => {
            tx4_persistence::close_pool(&pool).await;
            Err(err.into())
        }
    }
}
