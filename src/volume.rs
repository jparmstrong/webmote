use std::process::Command;

#[derive(Clone, Copy)]
pub enum Backend {
    Wpctl,
    Pactl,
    /// macOS, via `osascript` (`set volume` / `get volume settings`).
    Macos,
}

pub fn detect() -> Backend {
    // Probe in order: PipeWire (Linux), then macOS, then fall back to PulseAudio.
    // Runtime probing (rather than cfg) keeps every variant constructed on every
    // platform, so there's no dead-code lint to fight.
    if probe("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
        eprintln!("Volume backend: wpctl");
        Backend::Wpctl
    } else if probe("osascript", &["-e", "output volume of (get volume settings)"]) {
        eprintln!("Volume backend: macOS (osascript)");
        Backend::Macos
    } else {
        eprintln!("Volume backend: pactl");
        Backend::Pactl
    }
}

/// Runs a command and reports whether it exists and exited successfully.
fn probe(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// On macOS, up/down/mute are no-ops here: main.rs drives them by synthesising the
// media keys via enigo, which both changes the volume and shows the system volume
// HUD (osascript would change it silently). osascript is used only by `get` below,
// to read the level for the on-screen display.

pub fn up(backend: Backend) {
    match backend {
        Backend::Wpctl => run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"]),
        Backend::Pactl => run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", "+5%"]),
        Backend::Macos => {}
    }
}

pub fn down(backend: Backend) {
    match backend {
        Backend::Wpctl => run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"]),
        Backend::Pactl => run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", "-5%"]),
        Backend::Macos => {}
    }
}

pub fn mute(backend: Backend) {
    match backend {
        Backend::Wpctl => run("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]),
        Backend::Pactl => run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"]),
        Backend::Macos => {}
    }
}

/// Returns current volume as 0–100. Returns 0 on parse failure.
pub fn get(backend: Backend) -> u8 {
    match backend {
        Backend::Wpctl => {
            let out = Command::new("wpctl")
                .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
                .output()
                .ok();
            // output looks like: "Volume: 0.54\n" or "Volume: 0.54 [MUTED]\n"
            out.and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    s.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse::<f32>().ok())
                })
                .map(|f| (f * 100.0).round().min(100.0) as u8)
                .unwrap_or(0)
        }
        Backend::Pactl => {
            let out = Command::new("pactl")
                .args(["get-sink-volume", "@DEFAULT_SINK@"])
                .output()
                .ok();
            // output looks like: "Volume: front-left: 39321 /  60% / ..."
            out.and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    s.split('/').nth(1).and_then(|pct| {
                        pct.trim().trim_end_matches('%').parse::<u8>().ok()
                    })
                })
                .unwrap_or(0)
        }
        Backend::Macos => {
            let out = Command::new("osascript")
                .args(["-e", "output volume of (get volume settings)"])
                .output()
                .ok();
            // output is a bare integer 0–100, e.g. "60\n"
            out.and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0)
        }
    }
}

fn run(cmd: &str, args: &[&str]) {
    let _ = Command::new(cmd).args(args).status();
}
