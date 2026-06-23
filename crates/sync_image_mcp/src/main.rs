use std::io::{self, BufReader};

use anyhow::Result;
use sync_image_mcp::protocol::{config_from_env, run_stdio};

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let config = config_from_env()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio(config, BufReader::new(stdin.lock()), stdout.lock())
}

fn print_help() {
    println!(
        "sync-image-mcp\n\n\
         Stdio MCP server for the latest uploaded Claude image.\n\n\
         Required environment:\n\
           CLAUDE_IMAGE_ROOT=/data/claude-images\n\
           CLAUDE_IMAGE_USER=$USER"
    );
}
