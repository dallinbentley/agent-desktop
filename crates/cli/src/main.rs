use clap::{Parser, Subcommand};

mod connection;
mod output;

use agent_computer_shared::protocol::*;

// MARK: - Root Command

#[derive(Parser)]
#[command(
    name = "agent-computer",
    about = "Control macOS GUI applications via accessibility and input simulation.",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output raw JSON response
    #[arg(long, global = true)]
    json: bool,

    /// Timeout in milliseconds
    #[arg(long, global = true)]
    timeout: Option<u64>,

    /// Enable verbose output
    #[arg(long, global = true)]
    verbose: bool,
}

// MARK: - Subcommands

#[derive(Subcommand)]
enum Commands {
    /// Take an accessibility tree snapshot
    Snapshot {
        /// Show only interactive elements with @refs
        #[arg(short, long)]
        interactive: bool,

        /// Compact output format
        #[arg(short, long)]
        compact: bool,

        /// Maximum tree depth
        #[arg(short = 'd', long)]
        depth: Option<u32>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,

        /// CSS selector to scope snapshot (CDP only)
        #[arg(short, long)]
        selector: Option<String>,
    },

    /// Click an element by @ref or coordinates
    Click {
        /// Element @ref (e.g. @e3) or X coordinate
        ref_or_x: String,

        /// Y coordinate (when using coordinate pair)
        y: Option<f64>,

        /// Double-click
        #[arg(long)]
        double: bool,

        /// Right-click
        #[arg(long)]
        right: bool,

        /// Bring app to foreground (required for coordinate clicks with --app)
        #[arg(long)]
        foreground: bool,

        /// Target app name (headless click)
        #[arg(long)]
        app: Option<String>,

        /// Skip post-click wait for SPA navigation
        #[arg(long)]
        no_wait: bool,
    },

    /// Clear and fill a text field
    Fill {
        /// Element @ref (e.g. @e4)
        r#ref: String,

        /// Text to fill
        text: String,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Type text, optionally into a specific element
    Type {
        /// Element @ref (optional) or text to type
        ref_or_text: String,

        /// Text to type (when ref is provided)
        text: Option<String>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Press a key or key combination (e.g. cmd+c, enter)
    Press {
        /// Key combo (e.g. cmd+shift+s, enter, tab)
        key: String,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Scroll in a direction
    Scroll {
        /// Direction: up, down, left, right
        direction: String,

        /// Amount in pixels (default: 300)
        amount: Option<i32>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Capture a screenshot
    Screenshot {
        /// Capture full screen instead of frontmost window
        #[arg(long)]
        full: bool,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Open or focus an application
    Open {
        /// Application name or bundle ID
        target: String,

        /// Relaunch with Chrome DevTools Protocol enabled (launches hidden)
        #[arg(long)]
        with_cdp: bool,

        /// Launch in background without stealing focus
        #[arg(long)]
        background: bool,
    },

    /// Get information (text, title, apps, windows)
    Get {
        /// What to get: text, title, apps, windows
        what: String,

        /// Element @ref (for text/title)
        r#ref: Option<String>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Wait for an element, time, or page load state
    Wait {
        /// Element @ref (e.g. @e5) or milliseconds (e.g. 2000)
        ref_or_ms: Option<String>,

        /// Wait for page load state: networkidle, domcontentloaded, load
        #[arg(long)]
        load: Option<String>,

        /// Target app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Show daemon status and permissions
    Status,

    /// Download and install agent-browser binary + Chrome for Testing
    InstallBrowser,
}

// MARK: - Helpers

/// Parse an @ref string: strip @, validate e\d+ format.
fn parse_ref(input: &str) -> Option<String> {
    let stripped = input.strip_prefix('@').unwrap_or(input);
    // Validate e\d+ format
    if stripped.starts_with('e') && stripped.len() > 1 && stripped[1..].chars().all(|c| c.is_ascii_digit())
    {
        Some(stripped.to_string())
    } else {
        None
    }
}

/// Parse key combo like "cmd+shift+s" into (key, modifiers).
fn parse_key_combo(input: &str) -> (String, Vec<String>) {
    let parts: Vec<String> = input.to_lowercase().split('+').map(|s| s.to_string()).collect();
    if parts.is_empty() {
        return (input.to_string(), vec![]);
    }

    let modifier_names = [
        "cmd", "command", "shift", "alt", "option", "ctrl", "control", "fn",
    ];

    let mut modifiers = Vec::new();
    let mut key_parts = Vec::new();

    for part in &parts {
        if modifier_names.contains(&part.as_str()) {
            // Normalize modifier names
            let normalized = match part.as_str() {
                "command" => "cmd",
                "option" => "alt",
                "control" => "ctrl",
                _ => part.as_str(),
            };
            modifiers.push(normalized.to_string());
        } else {
            key_parts.push(part.clone());
        }
    }

    let key = if key_parts.is_empty() {
        parts.last().unwrap().clone()
    } else {
        key_parts.join("+")
    };

    (key, modifiers)
}

// MARK: - Main

fn main() {
    let cli = Cli::parse();

    let (command, args) = match &cli.command {
        Commands::Snapshot {
            interactive,
            compact,
            depth,
            app,
            selector,
        } => {
            let args = SnapshotArgs {
                interactive: *interactive,
                compact: *compact,
                depth: *depth,
                app: app.clone(),
                selector: selector.clone(),
            };
            ("snapshot", serde_json::to_value(args).unwrap())
        }
        Commands::Click {
            ref_or_x,
            y,
            double,
            right,
            foreground,
            app,
            no_wait,
        } => {
            if let Some(y_val) = y {
                // Coordinate pair mode
                match ref_or_x.parse::<f64>() {
                    Ok(x) => {
                        let args = ClickArgs {
                            r#ref: None,
                            x: Some(x),
                            y: Some(*y_val),
                            double: *double,
                            right: *right,
                            foreground: *foreground,
                            app: app.clone(),
                            no_wait: *no_wait,
                        };
                        ("click", serde_json::to_value(args).unwrap())
                    }
                    Err(_) => {
                        eprintln!("Error: Invalid X coordinate '{}'", ref_or_x);
                        std::process::exit(1);
                    }
                }
            } else if let Some(parsed_ref) = parse_ref(ref_or_x) {
                let args = ClickArgs {
                    r#ref: Some(parsed_ref),
                    x: None,
                    y: None,
                    double: *double,
                    right: *right,
                    foreground: *foreground,
                    app: app.clone(),
                    no_wait: *no_wait,
                };
                ("click", serde_json::to_value(args).unwrap())
            } else if ref_or_x.parse::<f64>().is_ok() {
                eprintln!("Error: Click by coordinates requires both X and Y values.");
                std::process::exit(1);
            } else {
                eprintln!(
                    "Error: Invalid ref '{}'. Use @e<number> format (e.g. @e3) or provide X Y coordinates.",
                    ref_or_x
                );
                std::process::exit(1);
            }
        }
        Commands::Fill { r#ref, text, app } => {
            match parse_ref(r#ref) {
                Some(parsed_ref) => {
                    let args = FillArgs {
                        r#ref: parsed_ref,
                        text: text.clone(),
                        app: app.clone(),
                    };
                    ("fill", serde_json::to_value(args).unwrap())
                }
                None => {
                    eprintln!(
                        "Error: Invalid ref '{}'. Use @e<number> format (e.g. @e4).",
                        r#ref
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::Type {
            ref_or_text,
            text,
            app,
        } => {
            if let Some(text_val) = text {
                // Two arguments: first is ref, second is text
                match parse_ref(ref_or_text) {
                    Some(parsed_ref) => {
                        let args = TypeArgs {
                            r#ref: Some(parsed_ref),
                            text: text_val.clone(),
                            app: app.clone(),
                        };
                        ("type", serde_json::to_value(args).unwrap())
                    }
                    None => {
                        eprintln!(
                            "Error: Invalid ref '{}'. Use @e<number> format (e.g. @e4).",
                            ref_or_text
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                // Single argument: just text
                let args = TypeArgs {
                    r#ref: None,
                    text: ref_or_text.clone(),
                    app: app.clone(),
                };
                ("type", serde_json::to_value(args).unwrap())
            }
        }
        Commands::Press { key, app } => {
            let (parsed_key, modifiers) = parse_key_combo(key);
            let args = PressArgs {
                key: parsed_key,
                modifiers: if modifiers.is_empty() {
                    None
                } else {
                    Some(modifiers)
                },
                app: app.clone(),
            };
            ("press", serde_json::to_value(args).unwrap())
        }
        Commands::Scroll { direction, amount, app } => {
            let dir = direction.to_lowercase();
            let valid = ["up", "down", "left", "right"];
            if !valid.contains(&dir.as_str()) {
                eprintln!(
                    "Error: Invalid direction '{}'. Use: up, down, left, right.",
                    direction
                );
                std::process::exit(1);
            }
            let args = ScrollArgs {
                direction: dir,
                amount: *amount,
                r#ref: None,
                app: app.clone(),
            };
            ("scroll", serde_json::to_value(args).unwrap())
        }
        Commands::Screenshot { full, app } => {
            let args = ScreenshotArgs {
                full: *full,
                app: app.clone(),
            };
            ("screenshot", serde_json::to_value(args).unwrap())
        }
        Commands::Open { target, with_cdp, background } => {
            let args = OpenArgs {
                target: target.clone(),
                with_cdp: *with_cdp,
                background: *background || *with_cdp, // --with-cdp implies background
            };
            ("open", serde_json::to_value(args).unwrap())
        }
        Commands::Get { what, r#ref, app } => {
            let valid_whats = ["text", "title", "url", "apps", "windows"];
            let what_lower = what.to_lowercase();
            if !valid_whats.contains(&what_lower.as_str()) {
                eprintln!(
                    "Error: Invalid target '{}'. Use: text, title, url, apps, windows.",
                    what
                );
                std::process::exit(1);
            }

            let parsed_ref = if let Some(ref_str) = r#ref {
                match parse_ref(ref_str) {
                    Some(r) => Some(r),
                    None => {
                        eprintln!(
                            "Error: Invalid ref '{}'. Use @e<number> format (e.g. @e3).",
                            ref_str
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let args = GetArgs {
                what: what_lower,
                r#ref: parsed_ref,
                app: app.clone(),
            };
            ("get", serde_json::to_value(args).unwrap())
        }
        Commands::Wait { ref_or_ms, load, app } => {
            let args = WaitArgs {
                ref_or_ms: ref_or_ms.clone(),
                load: load.clone(),
                app: app.clone(),
            };
            ("wait", serde_json::to_value(args).unwrap())
        }
        Commands::Status => {
            ("status", serde_json::Value::Object(serde_json::Map::new()))
        }
        Commands::InstallBrowser => {
            // Handle locally — no daemon needed
            install_browser_command();
            return;
        }
    };

    // Build request
    let request = Request {
        id: uuid_v4(),
        command: command.to_string(),
        args,
        options: Some(RequestOptions {
            timeout: cli.timeout,
            json: Some(cli.json),
            verbose: Some(cli.verbose),
        }),
    };

    // Send to daemon
    match connection::send(&request, cli.verbose) {
        Ok(response) => {
            let success = output::print_response(&response, cli.json);
            if !success {
                std::process::exit(1);
            }
        }
        Err(e) => {
            output::print_local_error(
                &e.to_string(),
                Some("Is the daemon running? Try 'agent-computer status'."),
            );
            std::process::exit(1);
        }
    }
}

/// Install agent-browser binary and Chrome for Testing.
/// This is a local command that doesn't require the daemon.
fn install_browser_command() {
    use std::io::Read;
    use std::process::Command;

    const AGENT_BROWSER_VERSION: &str = "0.22.1";

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Error: Cannot determine home directory.");
            std::process::exit(1);
        }
    };

    let bin_dir = home.join(".agent-computer/bin");
    let target_path = bin_dir.join("agent-browser");

    // Check if already installed
    if target_path.exists() {
        eprintln!("agent-browser already installed at {}", target_path.display());
        eprintln!("Running 'agent-browser install' to ensure Chrome for Testing is set up...");
        let status = Command::new(&target_path)
            .arg("install")
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("✓ Chrome for Testing is ready.");
                return;
            }
            Ok(s) => {
                eprintln!("Warning: 'agent-browser install' exited with {}", s);
            }
            Err(e) => {
                eprintln!("Warning: Failed to run 'agent-browser install': {}", e);
            }
        }
        return;
    }

    // Detect platform
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        eprintln!("Error: Unsupported OS for agent-browser binary.");
        std::process::exit(1);
    };

    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        eprintln!("Error: Unsupported architecture for agent-browser binary.");
        std::process::exit(1);
    };

    let binary_name = format!("agent-browser-{}-{}", os, arch);
    eprintln!("Installing agent-browser v{} ({}-{})...", AGENT_BROWSER_VERSION, os, arch);

    // Create directories
    if let Err(e) = std::fs::create_dir_all(&bin_dir) {
        eprintln!("Error: Failed to create {}: {}", bin_dir.display(), e);
        std::process::exit(1);
    }

    // Download
    let url = format!(
        "https://registry.npmjs.org/agent-browser/-/agent-browser-{}.tgz",
        AGENT_BROWSER_VERSION
    );
    eprintln!("Downloading from {}...", url);

    let tmp_dir = bin_dir.join(".download-tmp");
    if tmp_dir.exists() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let tgz_path = tmp_dir.join("agent-browser.tgz");

    let response = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            eprintln!("Error: Download failed: {}", e);
            std::process::exit(1);
        }
    };

    let mut body = Vec::new();
    if let Err(e) = response.into_reader().read_to_end(&mut body) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        eprintln!("Error: Failed to read response: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = std::fs::write(&tgz_path, &body) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        eprintln!("Error: Failed to save download: {}", e);
        std::process::exit(1);
    }

    eprintln!("Downloaded {} bytes, extracting...", body.len());

    // Extract
    let tar_result = Command::new("tar")
        .arg("xzf")
        .arg(tgz_path.to_str().unwrap())
        .arg("-C")
        .arg(tmp_dir.to_str().unwrap())
        .output();

    match tar_result {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_dir_all(&tmp_dir);
            eprintln!("Error: tar extraction failed: {}", stderr);
            std::process::exit(1);
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            eprintln!("Error: Failed to run tar: {}", e);
            std::process::exit(1);
        }
        _ => {}
    }

    let extracted = tmp_dir.join("package/bin").join(&binary_name);
    if !extracted.exists() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        eprintln!("Error: Binary '{}' not found in npm package.", binary_name);
        std::process::exit(1);
    }

    if let Err(e) = std::fs::copy(&extracted, &target_path) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        eprintln!("Error: Failed to copy binary: {}", e);
        std::process::exit(1);
    }

    // chmod +x
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&target_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&target_path, perms);
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
    eprintln!("✓ Installed agent-browser at {}", target_path.display());

    // Run agent-browser install for Chrome for Testing
    eprintln!("Running 'agent-browser install' to download Chrome for Testing...");
    match Command::new(&target_path).arg("install").status() {
        Ok(s) if s.success() => {
            eprintln!("✓ Chrome for Testing installed. Browser automation is ready!");
        }
        Ok(s) => {
            eprintln!("Warning: 'agent-browser install' exited with {}", s);
        }
        Err(e) => {
            eprintln!("Warning: Failed to run 'agent-browser install': {}", e);
        }
    }
}

/// Generate a simple UUID v4.
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    format!("{:x}-{:x}", t, pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ref_valid() {
        assert_eq!(parse_ref("@e1"), Some("e1".to_string()));
        assert_eq!(parse_ref("@e42"), Some("e42".to_string()));
        assert_eq!(parse_ref("e3"), Some("e3".to_string()));
    }

    #[test]
    fn test_parse_ref_invalid() {
        assert_eq!(parse_ref("@f1"), None);
        assert_eq!(parse_ref("e"), None);
        assert_eq!(parse_ref("hello"), None);
        assert_eq!(parse_ref("@eabc"), None);
    }

    #[test]
    fn test_parse_key_combo_simple() {
        let (key, mods) = parse_key_combo("enter");
        assert_eq!(key, "enter");
        assert!(mods.is_empty());
    }

    #[test]
    fn test_parse_key_combo_with_modifiers() {
        let (key, mods) = parse_key_combo("cmd+shift+s");
        assert_eq!(key, "s");
        assert_eq!(mods, vec!["cmd", "shift"]);
    }

    #[test]
    fn test_parse_key_combo_normalize() {
        let (key, mods) = parse_key_combo("command+option+a");
        assert_eq!(key, "a");
        assert_eq!(mods, vec!["cmd", "alt"]);
    }
}
