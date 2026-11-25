const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const byId = (id) => document.getElementById(id);
const openButton = byId("open-on-phone");
const stopButton = byId("stop-sharing");
const accessibilityCard = byId("accessibility");
const grantButton = byId("grant-access");
const pairing = byId("pairing");
const qr = byId("qr");
const sharingUrl = byId("sharing-url");
const hotkeyInput = byId("hotkey");
const saveHotkey = byId("save-hotkey");

let sharingRequest = 0;
let sharingBusy = false;
let accessibilityGranted = null;
let checkingAccessibility = false;

function setMessage(id, message) {
  const element = byId(id);
  element.textContent = message ?? "";
  element.hidden = !message;
}

function updateOpenButton() {
  openButton.disabled = sharingBusy || accessibilityGranted !== true;
}

async function refreshAccessibility() {
  if (checkingAccessibility) return;
  checkingAccessibility = true;
  try {
    accessibilityGranted = await invoke("accessibility_granted");
    accessibilityCard.hidden = accessibilityGranted;
    setMessage("accessibility-status-error", null);
  } catch (error) {
    accessibilityGranted = null;
    setMessage("accessibility-status-error", `Could not check Accessibility access: ${error}`);
  } finally {
    checkingAccessibility = false;
    updateOpenButton();
  }
}

grantButton.addEventListener("click", async () => {
  grantButton.disabled = true;
  setMessage("grant-access-error", null);
  try {
    await invoke("open_accessibility_settings");
  } catch (error) {
    setMessage("grant-access-error", `Could not open Accessibility settings: ${error}`);
  } finally {
    grantButton.disabled = false;
    await refreshAccessibility();
  }
});

function renderSharing(info) {
  const active = Boolean(info);
  const status = byId("sharing-status");
  const label = active ? "Sharing" : "Sharing off";
  if (status.textContent !== label) status.textContent = label;
  byId("sharing-dot").classList.toggle("active", active);
  openButton.hidden = active;
  updateOpenButton();
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
    if (command === "start_sharing") refreshAccessibility();
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

const phoneCards = new Map();
const phonesList = byId("phones");
let phoneRequest = 0;
let phoneBusy = false;
let draggingChip = null;
let overCard = null;
let chipCatalog = new Map();
let palette = [];

function button(label, className, onClick) {
  const element = document.createElement("button");
  element.type = "button";
  element.className = `btn ${className}`;
  element.textContent = label;
  element.addEventListener("click", onClick);
  return element;
}

function chipLabel(chip) {
  return chipCatalog.get(chip) ?? chip.slice(2);
}

function chipElement(tag, chip) {
  const element = document.createElement(tag);
  element.className = chip.startsWith("d:") ? "chip dir" : "chip";
  element.textContent = chipLabel(chip);
  return element;
}

function renderPalette(chips, labels) {
  if (JSON.stringify([chips, [...labels]]) === JSON.stringify([palette, [...chipCatalog]])) return;
  finishDrag();
  palette = chips;
  chipCatalog = labels;
  byId("chip-palette").replaceChildren(...chips.map((chip) => {
    const element = chipElement("li", chip);
    element.draggable = true;
    element.addEventListener("dragstart", (event) => {
      draggingChip = chip;
      event.dataTransfer.effectAllowed = "copy";
      event.dataTransfer.setData("text/plain", chip);
    });
    return element;
  }));
}

function highlightDrop(card) {
  if (overCard === card) return;
  overCard?.classList.remove("over");
  overCard = card;
  overCard?.classList.add("over");
}

function finishDrag() {
  draggingChip = null;
  highlightDrop(null);
}

document.addEventListener("dragend", finishDrag);
document.addEventListener("dragover", (event) => {
  if (!draggingChip) return;
  const card = event.target.closest(".phonecard");
  highlightDrop(card);
  if (!card) return;
  event.preventDefault();
  event.dataTransfer.dropEffect = "copy";
});
phonesList.addEventListener("drop", (event) => {
  const card = event.target.closest(".phonecard");
  if (!card || !draggingChip) return;
  event.preventDefault();
  const chip = draggingChip;
  finishDrag();
  addPhoneChip(phoneCards.get(card.dataset.phoneId).phone, chip);
});

async function phoneAction(command, args, message) {
  if (phoneBusy) return;
  phoneBusy = true;
  ++phoneRequest;
  setMessage("phones-error", null);
  byId("phones-status").textContent = "";
  try {
    await invoke(command, args);
    byId("phones-status").textContent = message;
  } catch (error) {
    setMessage("phones-error", error);
  } finally {
    phoneBusy = false;
    await refreshPhones();
  }
}

function addPhoneChip(phone, chip) {
  if (phone.view === null || phone.view.includes(chip)) {
    byId("phones-status").textContent = `${phone.name} already has ${chipLabel(chip)}.`;
    return;
  }
  phoneAction("add_chip_to_phone", { id: phone.id, chip },
    `Added ${chipLabel(chip)} to ${phone.name}.`);
}

function createPhoneCard(id) {
  const item = document.createElement("li");
  item.className = "phonecard";
  item.dataset.phoneId = id;

  const head = document.createElement("div");
  head.className = "phone-head";
  const dot = document.createElement("span");
  dot.className = "dot";
  dot.setAttribute("aria-hidden", "true");
  const name = document.createElement("b");
  const seen = document.createElement("span");
  seen.className = "muted";
  head.append(dot, name, seen);

  const drop = document.createElement("div");
  drop.className = "drop";
  const actions = document.createElement("div");
  actions.className = "actions";
  const card = { item, dot, name, seen, drop };
  const rename = button("Rename", "", () => {
    if (phoneBusy || !name.isConnected) return;
    const input = document.createElement("input");
    input.value = card.phone.name;
    input.setAttribute("aria-label", `New name for ${card.phone.name}`);
    let finished = false;
    function finish(save, restoreFocus) {
      if (finished) return;
      finished = true;
      input.replaceWith(name);
      if (restoreFocus) rename.focus();
      if (save && input.value.trim() && input.value.trim() !== card.phone.name) {
        phoneAction("rename_phone", { id, name: input.value }, "Phone renamed.");
      }
    }
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === "Escape") {
        event.preventDefault();
        finish(event.key === "Enter", true);
      }
    });
    input.addEventListener("blur", () => finish(true, false));
    name.replaceWith(input);
    input.focus();
    input.select();
  });
  card.rename = rename;
  actions.append(rename,
    button("Disconnect", "", () => phoneAction("disconnect_phone", { id }, "Phone disconnected.")),
    button("Forget", "danger", () => phoneAction("forget_phone", { id }, "Phone forgotten.")),
  );
  item.append(head, drop, actions);
  return card;
}

function renderPhoneView(card, phone) {
  const chips = phone.view ?? palette;
  const available = phone.view === null ? [] : palette.filter((chip) => !chips.includes(chip));
  const key = JSON.stringify([
    phone.name, phone.view === null,
    chips.map((chip) => [chip, chipLabel(chip)]),
    available.map((chip) => [chip, chipLabel(chip)]),
  ]);
  if (key === card.viewKey) return;
  card.viewKey = key;

  const focus = card.drop.contains(document.activeElement) ? document.activeElement : null;
  const parts = chips.map((chip) => {
    const element = chipElement("span", chip);
    const remove = button("×", "remove-chip", () => {
      phoneAction("remove_chip_from_phone", { id: phone.id, chip },
        `Removed ${chipLabel(chip)} from ${phone.name}.`);
    });
    remove.dataset.chip = chip;
    remove.setAttribute("aria-label", `Remove ${chipLabel(chip)} from ${phone.name}`);
    element.append(remove);
    return element;
  });

  if (phone.view === null || !chips.length) {
    const note = document.createElement("span");
    note.className = "muted view-note";
    note.textContent = phone.view === null ? "Everything" : "Sees nothing · drop chips here";
    parts.unshift(note);
  }

  const controls = document.createElement("form");
  controls.className = "view-controls";
  let select;
  if (available.length) {
    select = document.createElement("select");
    select.required = true;
    select.setAttribute("aria-label", `Add a chip to ${phone.name}`);
    const prompt = new Option("Add…", "");
    prompt.disabled = true;
    select.add(prompt);
    for (const chip of available) select.add(new Option(chipLabel(chip), chip));
    select.value = "";
    const add = document.createElement("button");
    add.type = "submit";
    add.className = "btn";
    add.textContent = "Add";
    controls.append(select, add);
    controls.addEventListener("submit", (event) => {
      event.preventDefault();
      addPhoneChip(card.phone, select.value);
    });
  }
  if (phone.view !== null) {
    const shareAll = button("Share everything", "", () =>
      phoneAction("set_phone_view", { id: phone.id, view: null }, `${phone.name} can see everything.`));
    controls.append(shareAll);
  }
  if (controls.childElementCount) parts.push(controls);
  card.drop.replaceChildren(...parts);

  if (focus) {
    const sameChip = [...card.drop.querySelectorAll(".remove-chip")]
      .find((remove) => remove.dataset.chip === focus.dataset.chip);
    (sameChip ?? select ?? card.drop.querySelector(".view-controls button[type=button]") ?? card.rename).focus();
  }
}

async function refreshPhones() {
  if (phoneBusy) return;
  const request = ++phoneRequest;
  try {
    const phones = await invoke("list_phones");
    if (request !== phoneRequest) return;
    const known = new Set(phones.map(({ id }) => id));
    for (const [id, card] of phoneCards) {
      if (!known.has(id)) {
        if (card.item.contains(document.activeElement)) byId("phones-heading").focus();
        card.item.remove();
        phoneCards.delete(id);
      }
    }
    byId("phones-message").hidden = phones.length > 0;
    for (const phone of phones) {
      let card = phoneCards.get(phone.id);
      if (!card) {
        card = createPhoneCard(phone.id);
        phoneCards.set(phone.id, card);
        phonesList.append(card.item);
      }
      card.phone = phone;
      card.dot.classList.toggle("active", phone.online);
      if (card.name.textContent !== phone.name) card.name.textContent = phone.name;
      const status = phone.online ? "Online" : `Last seen ${lastSeen(phone.last_seen)}`;
      if (card.seen.textContent !== status) card.seen.textContent = status;
      renderPhoneView(card, phone);
    }
  } catch (error) {
    if (request === phoneRequest) setMessage("phones-error", `Could not list phones: ${error}`);
  }
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

const startAtLogin = byId("start-at-login");
let checkingStartAtLogin = false;

async function syncStartAtLogin(enabled) {
  if (checkingStartAtLogin) return;
  checkingStartAtLogin = true;
  startAtLogin.disabled = true;
  let message = null;
  if (enabled !== undefined) {
    try {
      await invoke(enabled ? "plugin:autostart|enable" : "plugin:autostart|disable");
    } catch (error) {
      message = `Could not change Start at login: ${error}`;
    }
  }
  try {
    startAtLogin.checked = await invoke("plugin:autostart|is_enabled");
    startAtLogin.disabled = false;
  } catch (error) {
    message ??= `Could not check Start at login: ${error}`;
  } finally {
    checkingStartAtLogin = false;
    setMessage("start-at-login-error", message);
  }
}

startAtLogin.addEventListener("change", () => syncStartAtLogin(startAtLogin.checked));
window.addEventListener("focus", () => {
  refreshAccessibility();
  syncStartAtLogin();
});
refreshAccessibility();
syncStartAtLogin();

refreshPhones();
setInterval(() => {
  if (!document.hidden) {
    refreshPhones();
    refreshAccessibility();
  }
}, 2000);
loadHotkey();
listen("sharing-changed", ({ payload }) => {
  setMessage("sharing-error", payload);
  refreshSharing();
}).catch((error) => setMessage("sharing-error", `Could not listen for tray updates: ${error}`));
refreshSharing();
setInterval(() => {
  if (!document.hidden && !sharingBusy) refreshSharing();
}, 1000);
