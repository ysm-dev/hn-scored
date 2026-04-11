use std::process::ExitCode;

use chrono::Utc;

use hn_scored::{app, cli};

#[tokio::main]
async fn main() -> ExitCode {
    let args = cli::parse();
    let cycle_time = hn_scored::time::Timestamp::from_datetime(Utc::now());
    match app::run_once(&args, cycle_time, None).await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("[ERROR] {error}");
            ExitCode::from(1)
        }
    }
}
