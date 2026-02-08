//! Terra Store v3.1 - Main Entry Point
//!
//! A native TUI package manager for Arch Linux.
//! Features Zero-Stress indexing for instant package search.

mod auth;
mod database;
mod flatpak;
mod history;
mod package;
mod repos;
#[cfg(feature = "terraflow")]
mod terraflow;
mod theme;
mod ui;

use std::io;
use std::process::ExitCode;

use auth::AuthManager;
use history::History;
use package::PackageSource;
use repos::Repository;
#[cfg(feature = "terraflow")]
use terraflow::TerraFlow;
use ui::{draw, handle_input, init_terminal, restore_terminal, App, AppMode};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ASCII_BANNER: &str = r#"
╔════════════════════════════════════════════════════════════════╗
║                                                                ║
║   ████████╗███████╗██████╗ ██████╗  █████╗                     ║
║   ╚══██╔══╝██╔════╝██╔══██╗██╔══██╗██╔══██╗                    ║
║      ██║   █████╗  ██████╔╝██████╔╝███████║                    ║
║      ██║   ██╔══╝  ██╔══██╗██╔══██╗██╔══██║                    ║
║      ██║   ███████╗██║  ██║██║  ██║██║  ██║                    ║
║      ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝                    ║
║                                                                ║
║   ███████╗████████╗ ██████╗ ██████╗ ███████╗                   ║
║   ██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗██╔════╝                   ║
║   ███████╗   ██║   ██║   ██║██████╔╝█████╗                     ║
║   ╚════██║   ██║   ██║   ██║██╔══██╗██╔══╝                     ║
║   ███████║   ██║   ╚██████╔╝██║  ██║███████╗                   ║
║   ╚══════╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚══════╝                   ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝
"#;

fn main() -> ExitCode {
    // Print banner
    println!("{}", ASCII_BANNER);
    println!("   TERRA STORE v{} | Zero-Stress Edition", VERSION);
    println!("   ─────────────────────────────────────────────────────────\n");

    // Initialize authentication
    let mut auth = AuthManager::new();

    if let Err(e) = auth.authenticate() {
        eprintln!("\n   ✗ {}", e);
        return ExitCode::from(1);
    }

    // Run TUI mode
    match run_tui(&mut auth) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run_tui(auth: &mut AuthManager) -> io::Result<()> {
    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create app state
    let mut app = App::new();

    // Show loading screen
    terminal.draw(|f| draw(f, &mut app))?;

    // Load package database (uses binary cache if available)
    app.load_database();

    // Load installation history
    app.history = History::load();

    // Try to auto-detect TerraFlow config (if feature enabled)
    #[cfg(feature = "terraflow")]
    {
        app.terraflow = TerraFlow::auto_detect();
        if app.terraflow.is_some() {
            app.status = format!(
                "{} | TerraFlow detected",
                app.status
            );
        }
    }

    // Main event loop
    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &mut app))?;

        // Handle input
        let should_break = handle_input(&mut app)?;

        if app.should_quit {
            break;
        }

        if should_break && app.mode == AppMode::Search {
            // User pressed Enter - install the selected package
            if let Some((name, source)) = app.selected_package() {
                let name = name.to_string(); // Clone before leaving TUI

                // Temporarily restore terminal for installation output
                restore_terminal(&mut terminal)?;

                println!("\n   ═══════════════════════════════════════════════════════════");
                println!("   Installing: {}", name);
                println!("   ═══════════════════════════════════════════════════════════\n");

                let result = match source {
                    PackageSource::Official => app.repo_manager.pacman.install(&name),
                    PackageSource::Aur => app.repo_manager.aur.install(&name),
                };

                match result {
                    Ok(()) => {
                        println!(
                            "\n   ═══════════════════════════════════════════════════════════"
                        );
                        println!("   ✓ Successfully installed: {}", name);
                        println!(
                            "   ═══════════════════════════════════════════════════════════"
                        );
                        app.status = format!("✓ Installed {}", name);
                        app.history.record_success(&name, source);
                    }
                    Err(e) => {
                        println!(
                            "\n   ═══════════════════════════════════════════════════════════"
                        );
                        eprintln!("   ✗ Installation failed: {}", e);
                        println!(
                            "   ═══════════════════════════════════════════════════════════"
                        );
                        app.status = format!("✗ Failed: {}", e);
                        app.history.record_failure(&name, source, &e.to_string());
                    }
                }

                println!("\n   Press Enter to continue...");
                let mut _input = String::new();
                let _ = io::stdin().read_line(&mut _input);

                // Re-initialize terminal
                terminal = init_terminal()?;
            }
        }
    }

    // Cleanup
    restore_terminal(&mut terminal)?;
    auth.shutdown();

    println!("\n   Goodbye!\n");
    Ok(())
}
