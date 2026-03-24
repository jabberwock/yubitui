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
    
    // When running the TUI, log to a file to avoid interfering with the display
    if !args.list && !args.check {
        // TUI mode - log to file
        let log_path = std::env::temp_dir().join("yubitui.log");
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        
        if let Some(file) = log_file {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| log_level.into()),
                )
                .with(tracing_subscriber::fmt::layer().with_writer(std::sync::Arc::new(file)))
                .init();
        } else {
            // Fallback: no logging if file can't be created
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "error".into()),
                )
                .with(tracing_subscriber::fmt::layer().with_writer(std::io::sink))
                .init();
        }
    } else {
        // CLI mode - log to stdout as normal
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| log_level.into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    if args.list || args.check {
        tracing::info!("YubiTUI starting...");
    }

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
    
    // First, let's try to list all PC/SC readers to debug
    match pcsc::Context::establish(pcsc::Scope::System) {
        Ok(ctx) => {
            let mut readers_buf = [0; 2048];
            match ctx.list_readers(&mut readers_buf) {
                Ok(readers) => {
                    println!("📡 PC/SC Readers found:");
                    for reader in readers {
                        if let Ok(reader_str) = reader.to_str() {
                            println!("  • {}", reader_str);
                        }
                    }
                    println!();
                }
                Err(e) => {
                    println!("⚠️  Could not list PC/SC readers: {:?}\n", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Could not establish PC/SC context: {:?}\n", e);
            println!("PC/SC daemon may not be running or accessible.\n");
        }
    }
    
    let keys = detect_yubikeys()?;

    if keys.is_empty() {
        println!("❌ No YubiKeys detected.\n");
        println!("Troubleshooting:");
        println!("  • Ensure your YubiKey is plugged in");
        
        #[cfg(target_os = "macos")]
        {
            println!("  • On macOS, PC/SC should work automatically");
            println!("  • Check: ps aux | grep ctkpcscd");
        }
        
        #[cfg(target_os = "linux")]
        {
            println!("  • Check that pcscd is running: systemctl status pcscd");
            println!("  • Or start it: sudo systemctl start pcscd");
        }
        
        #[cfg(target_os = "windows")]
        {
            println!("  • PC/SC service should be running by default");
            println!("  • Check: Get-Service SCardSvr");
        }
        
        println!("  • Try: pcsc_scan (install with: brew install pcsc-tools / apt-get install pcsc-tools)");
        println!("  • Some readers require the YubiKey to be removed and reinserted");
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
