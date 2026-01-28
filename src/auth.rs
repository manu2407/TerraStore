//! Terra Store v3.0 - Authentication Module
//!
//! The "Gatekeeper" - Handles sudo privilege management with a background
//! keep-alive thread to prevent timeout during package browsing.

use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed: incorrect password")]
    InvalidPassword,

    #[error("Authentication cancelled by user")]
    Cancelled,

    #[error("Failed to spawn sudo process: {0}")]
    ProcessError(#[from] io::Error),

    #[error("Sudo not available on this system")]
    SudoNotFound,
}

/// Authentication manager that handles sudo privileges
pub struct AuthManager {
    /// Flag to signal the keep-alive thread to stop
    running: Arc<AtomicBool>,
    /// Handle to the keep-alive thread
    keepalive_handle: Option<thread::JoinHandle<()>>,
}

impl AuthManager {
    /// Create a new AuthManager (does not authenticate yet)
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            keepalive_handle: None,
        }
    }

    /// Check if we currently have sudo privileges (without prompting)
    pub fn has_privileges() -> bool {
        Command::new("sudo")
            .args(["-n", "true"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Authenticate with sudo, prompting for password if needed
    ///
    /// Returns Ok(()) if authentication succeeds, or an AuthError otherwise.
    pub fn authenticate(&mut self) -> Result<(), AuthError> {
        // Check if we already have privileges
        if Self::has_privileges() {
            self.spawn_keepalive();
            return Ok(());
        }

        // Prompt for password securely
        print!(":: Administrative privileges required.\n");
        print!("   Password: ");
        io::stdout().flush()?;

        let password = rpassword::read_password().map_err(|_| AuthError::Cancelled)?;

        if password.is_empty() {
            return Err(AuthError::Cancelled);
        }

        // Validate the password with sudo -S -v
        let mut child = Command::new("sudo")
            .args(["-S", "-v"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{}", password)?;
        }

        let status = child.wait()?;

        if status.success() {
            println!("   ✓ Authentication successful\n");
            self.spawn_keepalive();
            Ok(())
        } else {
            Err(AuthError::InvalidPassword)
        }
    }

    /// Spawn the background keep-alive thread
    ///
    /// This thread runs `sudo -v` every 60 seconds to prevent sudo timeout.
    fn spawn_keepalive(&mut self) {
        // Don't spawn multiple threads
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                // Refresh sudo timestamp
                let _ = Command::new("sudo")
                    .args(["-n", "-v"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();

                // Sleep for 60 seconds, but check running flag every second
                for _ in 0..60 {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });

        self.keepalive_handle = Some(handle);
    }

    /// Stop the keep-alive thread gracefully
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.keepalive_handle.take() {
            // Give it a moment to notice the flag change
            let _ = handle.join();
        }
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AuthManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_creation() {
        let manager = AuthManager::new();
        assert!(!manager.running.load(Ordering::SeqCst));
    }
}
