use crate::mouse::{parse_button, MouseCmd};
use std::net::TcpListener;
use std::sync::mpsc::Sender;
use tungstenite::{accept, Message};

pub fn run_ws_server(port: u16, pin: String, tx: Sender<MouseCmd>) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .unwrap_or_else(|e| panic!("Cannot bind WebSocket port {port}: {e}"));

    for stream in listener.incoming() {
        match stream {
            Ok(tcp) => {
                let pin = pin.clone();
                let tx = tx.clone();
                std::thread::spawn(move || handle_connection(tcp, pin, tx));
            }
            Err(e) => eprintln!("WS accept error: {e}"),
        }
    }
}

fn handle_connection(stream: std::net::TcpStream, pin: String, tx: Sender<MouseCmd>) {
    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WS handshake failed: {e}");
            return;
        }
    };

    // First frame must be the PIN
    match ws.read() {
        Ok(Message::Text(received_pin)) => {
            if received_pin.trim() != pin {
                let _ = ws.close(None);
                return;
            }
        }
        _ => {
            let _ = ws.close(None);
            return;
        }
    }

    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                if let Some(cmd) = parse_mouse_move(&text) {
                    let _ = tx.send(cmd);
                } else if let Some(cmd) = parse_mouse_click(&text) {
                    let _ = tx.send(cmd);
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
}

/// Parses {"dx":N,"dy":N}
fn parse_mouse_move(s: &str) -> Option<MouseCmd> {
    let dx = extract_int(s, "dx")?;
    let dy = extract_int(s, "dy")?;
    Some(MouseCmd::Move { dx, dy })
}

/// Parses {"button":"left"} etc — WebSocket click is optional but convenient
fn parse_mouse_click(s: &str) -> Option<MouseCmd> {
    let btn_str = extract_str(s, "button")?;
    let button = parse_button(&btn_str)?;
    Some(MouseCmd::Click { button })
}

/// Pulls an integer value for a given key from a flat JSON object.
/// e.g. extract_int(`{"dx":5,"dy":-3}`, "dx") == Some(5)
pub fn extract_int(json: &str, key: &str) -> Option<i32> {
    let pattern = format!("\"{key}\":");
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Pulls a string value for a given key from a flat JSON object.
/// e.g. extract_str(`{"button":"left"}`, "button") == Some("left")
fn extract_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find('"')?;
    Some(json[start..start + end].to_string())
}
