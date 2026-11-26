const { invoke } = window.__TAURI__.core;
const hudWindow = window.__TAURI__.window.getCurrentWindow();
const hud = document.getElementById("hud");
let last = "";

function render(summaries) {
  const key = JSON.stringify(summaries);
  if (key === last) return;
  last = key;
  hud.replaceChildren();
  if (!summaries.length) {
    const empty = document.createElement("span");
    empty.className = "hud-empty";
    empty.textContent = "No sessions";
    hud.append(empty);
  }
  for (const summary of summaries) {
    const ring = document.createElement("div");
    ring.className = "hud-logo";
    ring.dataset.ring = summary.ring;
    ring.title = `${summary.harness}: ${summary.session_count} session${summary.session_count === 1 ? "" : "s"}, ${summary.status}`;
    ring.setAttribute("aria-label", ring.title);
    const icon = document.createElement("span");
    icon.className = "harness-icon";
    icon.dataset.harness = summary.harness;
    icon.dataset.status = summary.status;
    const glyph = document.createElement("span");
    glyph.className = "glyph";
    glyph.textContent = summary.harness === "claude" ? "✳" : summary.harness === "codex" ? "▌" : "◆";
    icon.append(glyph);
    ring.append(icon);
    hud.append(ring);
  }
  hudWindow.setSize(new window.__TAURI__.dpi.LogicalSize(66, Math.max(72, summaries.length * 56 + 16))).catch(console.error);
}

async function refresh() {
  try { render(await invoke("hud_summaries_command")); }
  catch (error) { console.error("Could not refresh HUD", error); }
}

hud.addEventListener("mousedown", (event) => {
  if (event.button === 0) hudWindow.startDragging().catch(console.error);
});
refresh();
setInterval(refresh, 2000);
