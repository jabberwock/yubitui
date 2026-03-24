use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod diagnostics;
mod ui;
mod utils;
mod yubikey;

use app::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Run diagnostics and exit
    #[arg(short, long)]
    check: bool,

    /// List detected YubiKeys and exit
    #[arg(short, long)]
    list: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("YubiTUI starting...");

    if args.list {
        return list_yubikeys();
    }

    if args.check {
        return run_diagnostics();
    }

    // Run the TUI application
    let mut app = App::new()?;
    app.run()?;

    Ok(())
}

fn list_yubikeys() -> Result<()> {
    use yubikey::detection::detect_yubikeys;

    println!("Detecting YubiKeys...\n");
    let keys = detect_yubikeys()?;

    if keys.is_empty() {
        println!("❌ No YubiKeys detected.");
        println!("\nTroubleshooting:");
        println!("  • Ensure your YubiKey is plugged in");
        println!("  • Check that pcscd is running: systemctl status pcscd");
        println!("  • Try: pcsc_scan");
        return Ok(());
    }

    println!("✅ Found {} YubiKey(s):\n", keys.len());
    for (i, key) in keys.iter().enumerate() {
        println!("{}. {}", i + 1, key);
    }

    Ok(())
}

fn run_diagnostics() -> Result<()> {
    use diagnostics::Diagnostics;

    println!("Running system diagnostics...\n");
    let diag = Diagnostics::run()?;
    println!("{}", diag);

    if diag.has_errors() {
        println!("\n⚠️  Issues detected. Run 'yubitui' to see recommended fixes.");
        std::process::exit(1);
    } else {
        println!("\n✅ All checks passed!");
    }

    Ok(())
}
