mod mouse;
mod ui;
mod volume;
mod ws;

use enigo::Key;
use mouse::{parse_button, MouseCmd};
use qrcode::{render::unicode, QrCode};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_http::{Method, Response, Server};
use ws::extract_int;

fn main() {
    let lan_ip = detect_lan_ip();
    let pin = generate_pin();
    let vol_backend = volume::detect();

    // Channel all input commands flow through to the Enigo thread
    let (tx, rx) = mpsc::channel::<MouseCmd>();

    // Thread that owns Enigo and executes every input command
    std::thread::spawn(move || mouse::run_input_thread(rx));

    // WebSocket server for mouse movement (port 7071)
    {
        let ws_tx = tx.clone();
        let ws_pin = pin.clone();
        std::thread::spawn(move || ws::run_ws_server(7071, ws_pin, ws_tx));
    }

    let assets = ui::assets_dir();
    println!("Serving UI from: {}", assets.display());

    let url = format!("http://{}:7070/?pin={}", lan_ip, pin);
    print_qr(&url);
    println!("\n  {url}\n");

    // HTTP server for UI and discrete action routes (port 7070)
    let server = Server::http("0.0.0.0:7070").expect("cannot bind port 7070");
    println!("Listening on port 7070  (WebSocket on 7071)");

    for mut request in server.incoming_requests() {
        let url_str = request.url().to_string();
        let path = url_str.split('?').next().unwrap_or("/").to_string();

        // UI assets (everything GET except the /volume action) are served without
        // the PIN — they carry no privileges, and the browser requests them with no
        // query string. The PIN gates the action routes below and the WS handshake.
        if request.method() == &Method::Get && path != "/volume" {
            match ui::load(&assets, &path) {
                Some((bytes, content_type)) => {
                    let _ = request.respond(Response::from_data(bytes).with_header(
                        tiny_http::Header::from_bytes("Content-Type", content_type).unwrap(),
                    ));
                }
                None => {
                    let _ =
                        request.respond(Response::from_string("Not Found").with_status_code(404));
                }
            }
            continue;
        }

        if !check_pin(&url_str, &pin) {
            let _ = request.respond(Response::from_string("Forbidden").with_status_code(403));
            continue;
        }

        match (request.method(), path.as_str()) {
            (Method::Post, "/mouse/click") => {
                let body = read_body(&mut request);
                let btn_str = extract_query_param(&url_str, "button")
                    .or_else(|| extract_str_from_body(&body, "button"))
                    .unwrap_or_default();
                if let Some(button) = parse_button(&btn_str) {
                    let _ = tx.send(MouseCmd::Click { button });
                }
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/mouse/scroll") => {
                let body = read_body(&mut request);
                let dy = extract_query_int(&url_str, "dy")
                    .or_else(|| extract_int(&body, "dy"))
                    .unwrap_or(3);
                let _ = tx.send(MouseCmd::Scroll { dy });
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/volume/up") => {
                vol_up(&tx, vol_backend);
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/volume/down") => {
                vol_down(&tx, vol_backend);
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/volume/mute") => {
                vol_mute(&tx, vol_backend);
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Get, "/volume") => {
                let pct = volume::get(vol_backend);
                let _ = request.respond(Response::from_string(pct.to_string()));
            }

            (Method::Post, "/macro/fullscreen") => {
                let _ = tx.send(fullscreen_cmd());
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/macro/apps") => {
                let _ = tx.send(apps_cmd());
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/macro/netflix") => {
                open_url("https://www.netflix.com");
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/macro/youtubetv") => {
                open_url("https://tv.youtube.com");
                let _ = request.respond(Response::from_string("ok"));
            }

            (Method::Post, "/macro/youtube") => {
                open_url("https://www.youtube.com");
                let _ = request.respond(Response::from_string("ok"));
            }

            _ => {
                let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
            }
        }
    }
}

fn detect_lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:1")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Opens a URL in the user's default browser (`xdg-open` on Linux, `open` on macOS).
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(not(target_os = "macos"))]
    let opener = "xdg-open";

    if std::process::Command::new(opener).arg(url).spawn().is_err() {
        eprintln!("Could not open URL with {opener}: {url}");
    }
}

// Volume up/down/mute. On macOS, synthesise the media keys through enigo so the
// system volume HUD appears; on other platforms drive the audio server directly.

fn vol_up(tx: &mpsc::Sender<MouseCmd>, backend: volume::Backend) {
    #[cfg(target_os = "macos")]
    let _ = tx.send(MouseCmd::Combo { modifiers: vec![], key: Key::VolumeUp });
    #[cfg(not(target_os = "macos"))]
    let _ = tx;
    volume::up(backend); // no-op on macOS (handled by the media key above)
}

fn vol_down(tx: &mpsc::Sender<MouseCmd>, backend: volume::Backend) {
    #[cfg(target_os = "macos")]
    let _ = tx.send(MouseCmd::Combo { modifiers: vec![], key: Key::VolumeDown });
    #[cfg(not(target_os = "macos"))]
    let _ = tx;
    volume::down(backend);
}

fn vol_mute(tx: &mpsc::Sender<MouseCmd>, backend: volume::Backend) {
    #[cfg(target_os = "macos")]
    let _ = tx.send(MouseCmd::Combo { modifiers: vec![], key: Key::VolumeMute });
    #[cfg(not(target_os = "macos"))]
    let _ = tx;
    volume::mute(backend);
}

/// Toggle fullscreen for the focused window. macOS uses ⌃⌘F (the standard
/// system fullscreen shortcut); elsewhere F11 (browsers, most apps).
fn fullscreen_cmd() -> MouseCmd {
    #[cfg(target_os = "macos")]
    {
        MouseCmd::Combo {
            modifiers: vec![Key::Control, Key::Meta],
            key: Key::Unicode('f'),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        MouseCmd::Combo {
            modifiers: vec![],
            key: Key::F11,
        }
    }
}

/// Open the app/window overview. macOS uses Mission Control (⌃↑); on Linux the
/// Super/Meta key opens the GNOME-style activities overview.
fn apps_cmd() -> MouseCmd {
    #[cfg(target_os = "macos")]
    {
        MouseCmd::Combo {
            modifiers: vec![Key::Control],
            key: Key::UpArrow,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        MouseCmd::Combo {
            modifiers: vec![],
            key: Key::Meta,
        }
    }
}

/// Returns a stable 4-digit PIN: read from the config file if present,
/// otherwise generate a fresh random one and persist it for next time.
fn generate_pin() -> String {
    let path = pin_file_path();

    if let Some(ref p) = path
        && let Ok(existing) = std::fs::read_to_string(p)
    {
        let trimmed = existing.trim();
        if trimmed.len() == 4 && trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return trimmed.to_string();
        }
    }

    let pin = random_pin();
    if let Some(ref p) = path {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, &pin);
    }
    pin
}

/// `$XDG_CONFIG_HOME/webmote/pin`, falling back to `$HOME/.config/webmote/pin`.
fn pin_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("webmote").join("pin"))
}

fn random_pin() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1234);
    format!("{:04}", nanos % 10_000)
}

fn print_qr(url: &str) {
    match QrCode::new(url.as_bytes()) {
        Ok(code) => {
            let image = code.render::<unicode::Dense1x2>().build();
            println!("{image}");
        }
        Err(e) => eprintln!("QR error: {e}"),
    }
}

fn check_pin(url: &str, pin: &str) -> bool {
    extract_query_param(url, "pin").as_deref() == Some(pin)
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next() == Some(key) {
            return Some(parts.next().unwrap_or("").to_string());
        }
    }
    None
}

fn extract_query_int(url: &str, key: &str) -> Option<i32> {
    extract_query_param(url, key)?.parse().ok()
}

fn read_body(request: &mut tiny_http::Request) -> String {
    let mut body = String::new();
    let _ = std::io::Read::read_to_string(&mut request.as_reader(), &mut body);
    body
}

fn extract_str_from_body(body: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    let start = body.find(&pattern)? + pattern.len();
    let end = body[start..].find('"')?;
    Some(body[start..start + end].to_string())
}
