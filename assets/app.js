const params = new URLSearchParams(window.location.search);
const pin = params.get('pin') || '';
const host = window.location.hostname;
const HTTP = `http://${host}:7070`;
const WS_URL = `ws://${host}:7071`;

const statusEl = document.getElementById('status');

// --- WebSocket for mouse movement ---
let ws = null;
let wsReady = false;

function connectWs() {
  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    ws.send(pin); // PIN handshake
    wsReady = true;
    statusEl.textContent = 'connected';
  };

  ws.onclose = () => {
    wsReady = false;
    statusEl.textContent = 'disconnected — retrying...';
    setTimeout(connectWs, 2000);
  };

  ws.onerror = () => ws.close();
}

connectWs();

// --- Trackpad touch tracking ---
const pad = document.getElementById('trackpad');
let lastX = null, lastY = null;
let tapStartTime = 0, tapStartX = 0, tapStartY = 0;

pad.addEventListener('touchstart', e => {
  e.preventDefault();
  const t = e.touches[0];
  lastX = t.clientX;
  lastY = t.clientY;
  tapStartTime = Date.now();
  tapStartX = t.clientX;
  tapStartY = t.clientY;
});

pad.addEventListener('touchmove', e => {
  e.preventDefault();
  const t = e.touches[0];
  const dx = t.clientX - lastX;
  const dy = t.clientY - lastY;
  lastX = t.clientX;
  lastY = t.clientY;

  const speed = Math.sqrt(dx * dx + dy * dy);
  if (speed === 0 || !wsReady) return;

  // Acceleration: output scales as speed^1.5 so slow drags stay the same
  // but fast flicks are amplified (speed=10 → ~3× multiplier). Capped at 4×
  // so a hard flick can't fling the cursor across the screen.
  const factor = Math.min(Math.pow(speed, 0.5), 4);
  ws.send(JSON.stringify({ dx: Math.round(dx * factor), dy: Math.round(dy * factor) }));
});

pad.addEventListener('touchend', e => {
  e.preventDefault();
  const dt = Date.now() - tapStartTime;
  const dist = Math.sqrt((lastX - tapStartX) ** 2 + (lastY - tapStartY) ** 2);
  if (dt < 200 && dist < 10) click_btn('left');
  lastX = null;
  lastY = null;
});

// --- Discrete actions via HTTP ---
function post(path) {
  return fetch(`${HTTP}${path}?pin=${pin}`, { method: 'POST' }).catch(() => {});
}

function click_btn(button) {
  post(`/mouse/click?pin=${pin}&button=${button}`);
}

function volume(action) {
  post(`/volume/${action}`).then(() => refreshVol());
}

function refreshVol() {
  fetch(`${HTTP}/volume?pin=${pin}`)
    .then(r => r.text())
    .then(v => { document.getElementById('vol-display').textContent = `vol: ${v}%`; })
    .catch(() => {});
}

refreshVol();
setInterval(refreshVol, 5000);

// --- Macro keys ---
function macro_key(name) {
  post(`/macro/${name}`);
}

function launch(app) {
  post(`/macro/${app}`);
}
