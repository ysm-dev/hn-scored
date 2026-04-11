use clap::Parser;

use crate::config::{AppConfig, BASE_DISCOVERY_URL, normalize_base_url};

#[derive(Debug, Parser)]
#[command(name = "hn-scored", version, about = None)]
struct Args {
    #[arg(long, default_value = "./state.json")]
    state: std::path::PathBuf,
    #[arg(long, default_value = "./dist")]
    output: std::path::PathBuf,
    #[arg(long, default_value = "https://hn.ysm.dev", value_parser = parse_base_url)]
    base_url: String,
}

pub fn parse() -> AppConfig {
    let args = Args::parse();
    AppConfig {
        state_path: args.state,
        output_dir: args.output,
        base_url: args.base_url,
        api_base_url: BASE_DISCOVERY_URL.to_string(),
    }
}

fn parse_base_url(value: &str) -> Result<String, String> {
    normalize_base_url(value).map_err(|error| error.to_string())
}
