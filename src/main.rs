use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rustowl-mcp",
    author,
    version,
    about = "Model Context Protocol server for RustOwl lifetime & borrow inspection"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run MCP server over stdio (default behavior if no subcommand)
    #[arg(long)]
    stdio: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect variable lifetimes and borrows at a specific file coordinate
    Inspect {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        line: u32,
        #[arg(short, long)]
        col: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Inspect { file, line, col }) => {
            eprintln!("Inspecting {}:{}:{}", file.display(), line, col);
            let report = rustowl_mcp::analysis::inspect_cursor(&file, line, col).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        None => {
            eprintln!("Starting rustowl-mcp stdio server...");
            rustowl_mcp::mcp::server::run_stdio_server().await?;
        }
    }

    Ok(())
}
