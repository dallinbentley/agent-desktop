use agent_computer_shared::protocol::*;
use agent_computer_shared::types::*;
use agent_computer_shared::errors;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::sync::Mutex;

mod app;
mod cdp_engine;
mod detector;
mod refmap;

// MARK: - Daemon State

/// Shared daemon state accessible by all handlers.
pub struct DaemonState {
    /// Ref map: ref_id (e.g. "e1") -> ElementRef
    pub ref_map: HashMap<String, ElementRef>,
    /// When the ref map was last built
    pub ref_map_timestamp: Option<std::time::Instant>,
    /// Active CDP connections: app_name -> port
    pub cdp_connections: HashMap<String, u16>,
    /// Deterministic port assignments: app_name -> port
    pub port_assignments: HashMap<String, u16>,
}

impl DaemonState {
    fn new() -> Self {
        Self {
            ref_map: HashMap::new(),
            ref_map_timestamp: None,
            cdp_connections: HashMap::new(),
            port_assignments: HashMap::new(),
        }
    }

    pub fn ref_map_count(&self) -> i32 {
        self.ref_map.len() as i32
    }

    pub fn ref_map_age_ms(&self) -> Option<f64> {
        self.ref_map_timestamp.map(|t| t.elapsed().as_secs_f64() * 1000.0)
    }
}

// MARK: - Logging

fn log(msg: &str) {
    let now = chrono::Local::now();
    eprintln!("[{}] {}", now.format("%H:%M:%S%.3f"), msg);
}

// MARK: - Command Dispatch

/// Central dispatch function. Other agents' modules plug into this.
/// Returns a Response for the given command and args.
pub fn handle_command(
    command: &str,
    args: &serde_json::Value,
    id: &str,
    state: &mut DaemonState,
) -> Response {
    let start = std::time::Instant::now();
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    match command {
        "snapshot" => {
            let _args: SnapshotArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => SnapshotArgs {
                    interactive: true,
                    compact: false,
                    depth: None,
                    app: None,
                },
            };
            // Stub: return mock snapshot
            Response::ok(
                id.to_string(),
                ResponseData::Snapshot(SnapshotData {
                    text: "[No snapshot engine available yet]".to_string(),
                    ref_count: 0,
                    app: "Unknown".to_string(),
                    window: None,
                }),
                elapsed(),
            )
        }
        "click" => {
            let click_args: ClickArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("click requires args with ref or x/y"),
                        elapsed(),
                    );
                }
            };
            // Stub: return mock click
            let coords = if let (Some(x), Some(y)) = (click_args.x, click_args.y) {
                Point { x, y }
            } else {
                Point { x: 0.0, y: 0.0 }
            };
            Response::ok(
                id.to_string(),
                ResponseData::Click(ClickData {
                    r#ref: click_args.r#ref,
                    coordinates: coords,
                    element: None,
                }),
                elapsed(),
            )
        }
        "fill" => {
            let fill_args: FillArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("fill requires args with ref and text"),
                        elapsed(),
                    );
                }
            };
            Response::ok(
                id.to_string(),
                ResponseData::Fill(FillData {
                    r#ref: fill_args.r#ref.clone(),
                    text: fill_args.text.clone(),
                }),
                elapsed(),
            )
        }
        "type" => {
            let type_args: TypeArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("type requires args with text"),
                        elapsed(),
                    );
                }
            };
            Response::ok(
                id.to_string(),
                ResponseData::Type(TypeData {
                    r#ref: type_args.r#ref,
                    text: type_args.text,
                }),
                elapsed(),
            )
        }
        "press" => {
            let press_args: PressArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("press requires args with key"),
                        elapsed(),
                    );
                }
            };
            Response::ok(
                id.to_string(),
                ResponseData::Press(PressData {
                    key: press_args.key,
                    modifiers: press_args.modifiers.unwrap_or_default(),
                }),
                elapsed(),
            )
        }
        "scroll" => {
            let scroll_args: ScrollArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("scroll requires args with direction"),
                        elapsed(),
                    );
                }
            };
            Response::ok(
                id.to_string(),
                ResponseData::Scroll(ScrollData {
                    direction: scroll_args.direction,
                    amount: scroll_args.amount.unwrap_or(300),
                }),
                elapsed(),
            )
        }
        "screenshot" => {
            let _args: ScreenshotArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => ScreenshotArgs {
                    full: false,
                    app: None,
                },
            };
            // Stub
            Response::ok(
                id.to_string(),
                ResponseData::Screenshot(ScreenshotData {
                    path: "/tmp/screenshot.png".to_string(),
                    width: 1728,
                    height: 1117,
                    scale: 2,
                    window_origin_x: None,
                    window_origin_y: None,
                    app_name: None,
                }),
                elapsed(),
            )
        }
        "open" => {
            let open_args: OpenArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("open requires args with target"),
                        elapsed(),
                    );
                }
            };
            app::handle_open(id, &open_args, state, start)
        }
        "get" => {
            let get_args: GetArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("get requires args with what"),
                        elapsed(),
                    );
                }
            };
            app::handle_get(id, &get_args, state, start)
        }
        "status" => app::handle_status(id, state, start),
        _ => Response::fail(
            id.to_string(),
            errors::invalid_command(&format!("Unknown command: '{}'", command)),
            elapsed(),
        ),
    }
}

// MARK: - Stale Socket Handling

fn handle_stale_socket(path: &PathBuf) {
    if !path.exists() {
        return;
    }

    // Try to connect — if it succeeds, another daemon is running
    match std::os::unix::net::UnixStream::connect(path) {
        Ok(_) => {
            log("Another daemon is already running. Exiting.");
            std::process::exit(1);
        }
        Err(_) => {
            log("Stale socket found, removing...");
            let _ = std::fs::remove_file(path);
        }
    }
}

// MARK: - Client Handler

async fn handle_client(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<DaemonState>>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(json) => {
                        let id = json
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let command = json
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args = json
                            .get("args")
                            .cloned()
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                        let mut state = state.lock().await;
                        handle_command(&command, &args, &id, &mut state)
                    }
                    Err(_) => Response::fail(
                        "unknown".to_string(),
                        errors::invalid_command("Malformed JSON request"),
                        0.0,
                    ),
                };

                // Serialize and send response as JSON line
                match serde_json::to_string(&response) {
                    Ok(json) => {
                        let mut resp_bytes = json.into_bytes();
                        resp_bytes.push(b'\n');
                        if writer.write_all(&resp_bytes).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to serialize response: {}", e));
                        break;
                    }
                }
            }
            Err(e) => {
                log(&format!("Read error: {}", e));
                break;
            }
        }
    }
}

// MARK: - Main Entry Point

#[tokio::main]
async fn main() {
    log("agent-computer-daemon starting...");
    log(&format!("PID: {}", std::process::id()));

    let socket_dir = daemon_socket_dir();
    let socket_path = daemon_socket_path();

    // Create socket directory
    if let Err(e) = std::fs::create_dir_all(&socket_dir) {
        log(&format!("ERROR: Failed to create socket directory: {}", e));
        std::process::exit(1);
    }

    // Handle stale socket
    handle_stale_socket(&socket_path);

    // Bind Unix listener
    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("ERROR: Failed to bind socket: {}", e));
            std::process::exit(1);
        }
    };

    log(&format!("Listening on {}", socket_path.display()));

    let state = Arc::new(Mutex::new(DaemonState::new()));

    // Spawn accept loop with graceful shutdown
    let socket_path_clone = socket_path.clone();
    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        log("Client connected");
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            handle_client(stream, state).await;
                            log("Client disconnected");
                        });
                    }
                    Err(e) => {
                        log(&format!("Accept error: {}", e));
                    }
                }
            }
        } => {}
        _ = async {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler");
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to install SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => { log("Received SIGTERM"); }
                _ = sigint.recv() => { log("Received SIGINT"); }
            }
        } => {}
    }

    // Cleanup
    log("Shutting down...");
    let _ = std::fs::remove_file(&socket_path_clone);
    log("Daemon exited cleanly");
}
