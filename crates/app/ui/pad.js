const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const byId = (id) => document.getElementById(id);
const openButton = byId("open-on-phone");
const stopButton = byId("stop-sharing");
const pairing = byId("pairing");
const qr = byId("qr");
const sharingUrl = byId("sharing-url");
const hotkeyInput = byId("hotkey");
const saveHotkey = byId("save-hotkey");

let sharingRequest = 0;
let sharingBusy = false;

function setMessage(id, message) {
  const element = byId(id);
  element.textContent = message ?? "";
  element.hidden = !message;
}

async function refreshHarnesses() {
  const list = byId("harnesses");
  try {
    const harnesses = await invoke("list_harnesses");
    list.replaceChildren(...harnesses.map(({ name, skill_count }) => {
      const chip = document.createElement("li");
      chip.className = "chip";
      const count = document.createElement("span");
      count.className = "count";
      count.textContent = skill_count;
      count.setAttribute("aria-label", `${skill_count} ${skill_count === 1 ? "skill" : "skills"}`);
      chip.append(name, count);
      return chip;
    }));
    setMessage("harness-message", harnesses.length ? null : "No skills found.");
  } catch (error) {
    list.replaceChildren();
    setMessage("harness-message", `Could not list harnesses: ${error}`);
  }
}

function renderSharing(info) {
  const active = Boolean(info);
  const status = byId("sharing-status");
  const label = active ? "Sharing" : "Sharing off";
  if (status.textContent !== label) status.textContent = label;
  byId("sharing-dot").classList.toggle("active", active);
  openButton.hidden = active;
  openButton.disabled = sharingBusy;
  stopButton.hidden = !active;
  stopButton.disabled = sharingBusy;
  pairing.hidden = !active;

  if (!active) {
    qr.removeAttribute("src");
    sharingUrl.textContent = "";
  } else if (sharingUrl.textContent !== info.url) {
    qr.src = `data:image/svg+xml,${encodeURIComponent(info.qr_svg)}`;
    sharingUrl.textContent = info.url;
  }
}

async function refreshSharing() {
  const request = ++sharingRequest;
  try {
    const info = await invoke("sharing_status");
    if (request === sharingRequest) renderSharing(info);
  } catch (error) {
    if (request === sharingRequest) {
      setMessage("sharing-error", `Could not check sharing: ${error}`);
    }
  }
}

async function changeSharing(command) {
  sharingBusy = true;
  ++sharingRequest;
  openButton.disabled = true;
  stopButton.disabled = true;
  setMessage("sharing-error", null);
  try {
    await invoke(command);
  } catch (error) {
    setMessage("sharing-error", error);
  } finally {
    sharingBusy = false;
    await refreshSharing();
  }
}

openButton.addEventListener("click", () => changeSharing("start_sharing"));
stopButton.addEventListener("click", () => changeSharing("stop_sharing"));

async function loadHotkey() {
  try {
    const { hotkey, warning } = await invoke("get_hotkey");
    hotkeyInput.value = hotkey;
    setMessage("hotkey-error", warning);
    hotkeyInput.disabled = false;
    saveHotkey.disabled = false;
  } catch (error) {
    setMessage("hotkey-error", `Could not load the hotkey: ${error}`);
  }
}

byId("hotkey-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const input = hotkeyInput.value;
  hotkeyInput.readOnly = true;
  saveHotkey.disabled = true;
  setMessage("hotkey-error", null);
  setMessage("hotkey-status", null);
  try {
    const { hotkey } = await invoke("set_hotkey", { hotkey: input });
    hotkeyInput.value = hotkey;
    setMessage("hotkey-status", `Hotkey saved. Use ${hotkey} to show or hide the Pad.`);
  } catch (error) {
    setMessage("hotkey-error", error);
  } finally {
    hotkeyInput.readOnly = false;
    saveHotkey.disabled = false;
  }
});

function lastSeen(unix) {
  const seconds = Math.max(0, Math.round(Date.now() / 1000 - unix));
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.round(seconds / 60)} min ago`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)} h ago`;
  return `${Math.round(seconds / 86400)} days ago`;
}

let renamingPhone = null;

async function phoneAction(command, args) {
  setMessage("phones-error", null);
  try {
    await invoke(command, args);
  } catch (error) {
    setMessage("phones-error", error);
  }
  renamingPhone = null;
  refreshPhones();
}

function button(label, className, onClick) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = `btn ${className}`;
  b.textContent = label;
  b.addEventListener("click", onClick);
  return b;
}

async function refreshPhones() {
  if (renamingPhone) return; // don't clobber the rename field while typing
  let phones;
  try {
    phones = await invoke("list_phones");
  } catch (error) {
    setMessage("phones-error", `Could not list phones: ${error}`);
    return;
  }
  byId("phones-message").hidden = phones.length > 0;
  byId("phones").replaceChildren(...phones.map((phone) => {
    const item = document.createElement("li");
    item.className = "phone";
    const dot = document.createElement("span");
    dot.className = `dot${phone.online ? " active" : ""}`;
    dot.setAttribute("aria-label", phone.online ? "Online" : "Offline");
    const who = document.createElement("div");
    who.className = "who";
    const name = document.createElement("b");
    name.textContent = phone.name;
    const seen = document.createElement("span");
    seen.className = "muted";
    seen.textContent = phone.online ? "Online" : `Last seen ${lastSeen(phone.last_seen)}`;
    who.append(name, seen);
    const actions = document.createElement("div");
    actions.className = "actions";
    actions.append(
      button("Rename", "", () => {
        renamingPhone = phone.id;
        const input = document.createElement("input");
        input.value = phone.name;
        input.setAttribute("aria-label", `New name for ${phone.name}`);
        input.addEventListener("keydown", (event) => {
          if (event.key === "Enter") phoneAction("rename_phone", { id: phone.id, name: input.value });
          if (event.key === "Escape") { renamingPhone = null; refreshPhones(); }
        });
        input.addEventListener("blur", () => phoneAction("rename_phone", { id: phone.id, name: input.value }));
        name.replaceWith(input);
        input.focus();
        input.select();
      }),
      button("Disconnect", "", () => phoneAction("disconnect_phone", { id: phone.id })),
      button("Forget", "danger", () => phoneAction("forget_phone", { id: phone.id })),
    );
    item.append(dot, who, actions);
    return item;
  }));
}

const sendEnter = byId("send-enter");
invoke("settings").then((settings) => {
  sendEnter.checked = settings.send_enter;
  sendEnter.disabled = false;
}).catch((error) => setMessage("sharing-error", `Could not load settings: ${error}`));
sendEnter.addEventListener("change", async () => {
  try {
    await invoke("set_send_enter", { on: sendEnter.checked });
  } catch (error) {
    sendEnter.checked = !sendEnter.checked;
    setMessage("sharing-error", `Could not save the setting: ${error}`);
  }
});

window.addEventListener("focus", refreshHarnesses);
refreshHarnesses();
refreshPhones();
setInterval(() => { if (!document.hidden) refreshPhones(); }, 2000);
loadHotkey();
listen("sharing-changed", ({ payload }) => {
  setMessage("sharing-error", payload);
  refreshSharing();
}).catch((error) => setMessage("sharing-error", `Could not listen for tray updates: ${error}`));
refreshSharing();
setInterval(() => {
  if (!document.hidden && !sharingBusy) refreshSharing();
}, 1000);
