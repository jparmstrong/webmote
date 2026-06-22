# Webmote

LAN remote control for your **Linux or macOS** desktop. Run it, scan the QR code on your phone, and control the mouse and volume from your browser.

## Requirements

- Rust (stable) — install via [rustup](https://rustup.rs)

**Linux**
- An X11 or XWayland session (most Wayland desktops include XWayland)
- `wpctl` (PipeWire) or `pactl` (PulseAudio) for volume control — one of these is already present on most distros

**macOS**
- Grant the terminal (or app) running webmote **Accessibility** permission: System Settings → Privacy & Security → Accessibility. This is required for synthesizing mouse and key events.
- Volume control uses the built-in `osascript` — nothing to install.

## Install

```bash
git clone <repo>
cd webmote
cargo build --release
```

The binary will be at `target/release/webmote`.

The web UI is served from the `assets/` directory at runtime (not baked into the
binary), so it must be reachable when you run. At startup webmote looks for it in:

1. `$WEBMOTE_ASSETS`, if set
2. `assets/` next to the binary
3. `./assets` in the current directory

Running with `cargo run` from the project root works out of the box. To install
system-wide, install the binary and point it at the assets:

```bash
cargo install --path .
export WEBMOTE_ASSETS=/path/to/webmote/assets   # or copy assets/ next to the binary
```

## Run

Launch from a terminal **inside your desktop session** — webmote needs access to the display to move the mouse (on Linux, `$DISPLAY` must be set; don't run over a bare SSH shell. On macOS, grant Accessibility permission as noted above):

```bash
cargo run --release
# or, if installed:
webmote
```

On startup you will see:
- An ASCII QR code in the terminal
- The URL printed below it, e.g. `http://192.168.1.42:7070/?pin=3817`

Scan the QR code on your phone or open the URL in any browser on the same network.

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 7070 | HTTP | Web UI + discrete actions (clicks, volume) |
| 7071 | WebSocket | Real-time mouse movement stream |

Both ports must be reachable from your phone. If you have a firewall, open them:

```bash
# firewalld
sudo firewall-cmd --add-port=7070/tcp --add-port=7071/tcp

# ufw
sudo ufw allow 7070/tcp
sudo ufw allow 7071/tcp
```

## Security

A random 4-digit PIN is generated once and saved to `~/.config/webmote/pin`, then embedded in the QR code URL. All requests (HTTP and WebSocket) require the correct PIN — anyone without it gets a 403.

The PIN is reused across restarts so the QR code stays stable; delete `~/.config/webmote/pin` to rotate it.

## Controls

| Control | Action |
|---------|--------|
| Drag on trackpad area | Move mouse |
| Left / Right buttons | Mouse click |
| Swipe up/down on scroll strip | Scroll |
| − / Mute / + buttons | Volume down / toggle mute / up |
| Fullscreen button | `F11` on Linux, ⌃⌘F on macOS |
| Apps button | Activities overview (Linux) / Mission Control (macOS) |

Volume and the OS-specific buttons adapt to the platform automatically. On macOS, volume changes show the native volume HUD.

## Troubleshooting

**Mouse doesn't move**
- Linux: run from a terminal in your desktop session, not a bare SSH shell — `$DISPLAY` must be set.
- macOS: grant Accessibility permission to the terminal/app (System Settings → Privacy & Security → Accessibility), then restart webmote.

**Volume controls do nothing**
The startup log prints the detected backend: `Volume backend: wpctl`, `pactl`, or `macOS (osascript)`.
- Linux: if neither `wpctl` (PipeWire) nor `pactl` (PulseAudio) is on your `$PATH`, install `pipewire-utils` or `pulseaudio-utils`.
- macOS: volume uses the built-in `osascript`; if it does nothing, confirm Accessibility permission is granted (the media keys that show the HUD require it).

**Can't connect from phone**
Confirm both devices are on the same network and the firewall ports are open. The URL printed at startup shows the correct LAN IP.
