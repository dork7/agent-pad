const sessionHarnesses = [
  ["claude", "Claude Code"],
  ["codex", "Codex"],
];
const sessionHarnessNames = new Map(sessionHarnesses);
const sessionStatuses = {
  running: "Running",
  blocked: "Blocked",
  done: "Done",
  stalled: "Stalled",
  ended: "Ended",
};

byId("session-settings").addEventListener("click", () => {
  showPadSection("settings-panel");
  byId("session-settings-heading").focus();
  byId("session-settings-heading").scrollIntoView({ block: "center" });
});

function hookDiff({ before, after }) {
  const oldLines = before === null ? [] : before.split("\n");
  const newLines = after.split("\n");
  let first = 0;
  let oldEnd = oldLines.length;
  let newEnd = newLines.length;
  while (first < Math.min(oldEnd, newEnd) && oldLines[first] === newLines[first]) first++;
  while (oldEnd > first && newEnd > first && oldLines[oldEnd - 1] === newLines[newEnd - 1]) {
    oldEnd--;
    newEnd--;
  }
  const diff = el("pre", { className: "hook-diff", tabIndex: 0, ariaLabel: "Configuration diff" });
  function append(lines, prefix, className) {
    for (const line of lines) {
      diff.append(el("span", { className, textContent: `${prefix}${line}\n` }));
    }
  }
  append(oldLines.slice(0, first), "  ", "");
  append(oldLines.slice(first, oldEnd), "− ", "diff-remove");
  append(newLines.slice(first, newEnd), "+ ", "diff-add");
  append(newLines.slice(newEnd), "  ", "");
  return diff;
}

function hookEditor(id, name) {
  let preview;
  let busy = false;
  const heading = el("h4", { textContent: name });
  const details = el("div", { className: "hook-preview", hidden: true });
  const status = el("p", { className: "muted", role: "status", hidden: true });
  const error = el("p", { className: "error", role: "alert", hidden: true });
  const previewButton = button("Preview", "", async () => {
    if (await run("preview", (value) => {
      preview = value;
      const { path, before, after } = value;
      details.replaceChildren(
        el("p", { className: "path" }, el("code", { textContent: path })),
        el("p", { className: "muted", textContent: before === after
          ? "The file already contains the hooks."
          : before === null ? "This will create the configuration file." : "Review the changes before installing." }),
        hookDiff(value),
        el("div", { className: "form-row" }, install, cancel),
      );
      details.hidden = false;
    })) details.querySelector("pre").focus();
  });
  const remove = button("Remove", "danger", () => run("remove", () => {
    status.textContent = `${name} hooks removed.`;
    status.hidden = false;
  }));
  const install = button("Install", "primary", async () => {
    if (await run("apply", (notice) => {
      status.textContent = notice;
      status.hidden = false;
    }, { preview })) previewButton.focus();
    else if (details.hidden) previewButton.focus();
  });
  const cancel = button("Cancel", "", () => {
    preview = null;
    details.hidden = true;
    previewButton.focus();
  });

  async function run(action, onSuccess, args) {
    if (busy) return false;
    busy = true;
    error.hidden = true;
    if (action !== "preview") status.hidden = true;
    for (const control of [previewButton, remove, install, cancel]) control.disabled = true;
    try {
      const result = await invoke(`${action}_${id}_hooks`, args);
      details.hidden = true;
      preview = null;
      onSuccess(result);
      return true;
    } catch (failure) {
      const message = String(failure);
      if (action === "apply" && message.includes("preview the install again")) {
        preview = null;
        details.hidden = true;
        error.textContent = id === "codex"
          ? "Codex config changed. Choose Preview again. If you changed Codex homes, first remove the hooks from the previous home."
          : "Claude Code settings changed. Choose Preview to review the current change.";
      } else {
        error.textContent = message;
      }
      error.hidden = false;
      return false;
    } finally {
      busy = false;
      for (const control of [previewButton, remove, install, cancel]) control.disabled = false;
    }
  }

  return el("div", {}, heading, el("div", { className: "form-row" }, previewButton, remove), details, status, error);
}

for (const [id, name] of sessionHarnesses) {
  byId("hook-settings").append(hookEditor(id, name));
}

function sessionPreference(label, input, read, write, reset = null) {
  const status = el("p", { className: "muted", role: "status", hidden: true });
  const error = el("p", { className: "error", role: "alert", hidden: true });
  const save = el("button", { type: "submit", className: "btn", textContent: "Save", ariaLabel: `Save ${label}` });
  const row = el("div", { className: "form-row" },
    el("label", { className: "field" }, label, input), save);
  const resetButton = reset && button("Reset", "", () => change(reset, "Reset to default."));
  if (resetButton) {
    resetButton.setAttribute("aria-label", `Reset ${label}`);
    row.append(resetButton);
  }
  const fields = el("fieldset", { disabled: true }, row);
  const form = el("form", {}, fields, status, error);

  async function change(action, message) {
    if (fields.disabled) return;
    const focused = document.activeElement;
    fields.disabled = true;
    error.hidden = status.hidden = true;
    try {
      await action();
      input.value = await read();
      status.textContent = message;
      status.hidden = false;
    } catch (failure) {
      error.textContent = failure;
      error.hidden = false;
    } finally {
      fields.disabled = false;
      if (document.activeElement === document.body && form.contains(focused)) focused.focus();
    }
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    change(() => write(input.value), "Saved.");
  });
  read().then((value) => {
    input.value = value;
    fields.disabled = false;
  }).catch((failure) => {
    error.textContent = failure;
    error.hidden = false;
  });
  return form;
}

byId("threshold-setting").append(sessionPreference(
  "Mark stalled after (seconds)",
  el("input", { type: "number", min: 1, max: 4294967295, required: true }),
  () => invoke("stuck_threshold"),
  (seconds) => invoke("set_stuck_threshold", { seconds: Number(seconds) }),
));

const defaultTicketPattern = ".scratch/*/issues/*.md";
byId("ticket-pattern-setting").append(sessionPreference(
  "Ticket folder pattern",
  el("input", { required: true, spellcheck: false, placeholder: defaultTicketPattern }),
  async () => (await invoke("ticket_pattern")) ?? "",
  (pattern) => invoke("set_ticket_pattern", { pattern }),
  () => invoke("set_ticket_pattern", { pattern: null }),
));

for (const [harness, name] of sessionHarnesses) {
  byId("implement-settings").append(sessionPreference(
    name,
    el("textarea", { rows: 2, required: true, spellcheck: false }),
    () => invoke("implement_template", { harness }),
    (template) => invoke("set_implement_template", { harness, template }),
    () => invoke("set_implement_template", { harness, template: null }),
  ));
}

const sessionCards = new Map();
let pollingSessions = false;

function elapsedTime(seconds) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds % 3600 / 60);
  const remainder = String(seconds % 60).padStart(2, "0");
  return hours ? `${hours}:${String(minutes).padStart(2, "0")}:${remainder}` : `${minutes}:${remainder}`;
}

function sessionIcon(session) {
  const icon = el("span", { className: "harness-icon", ariaHidden: "true" },
    el("span", { className: "glyph", textContent: session.harness === "claude" ? "✳" : session.harness === "codex" ? "▌" : "◆" }));
  icon.dataset.harness = session.harness;
  icon.dataset.status = session.status;
  return icon;
}

function sessionCard(session) {
  const name = el("b", { className: "path" });
  const harness = el("span", { className: "muted" });
  const status = el("span", { className: "session-status" });
  const statusText = document.createTextNode("");
  status.append(statusText);
  const elapsed = el("span", { className: "muted numerical" });
  const tools = el("span", { className: "muted numerical" });
  const card = { session, name, harness, status, statusText, elapsed, tools };
  const action = button("Pending tickets", "", () => openTickets(card.session, action));
  card.action = action;
  card.item = el("li", { className: "session-card" },
    el("div", { className: "session-main" },
      el("div", { className: "session-project" }, sessionIcon(session), name, harness),
      el("div", { className: "session-meta" }, status, elapsed, tools)),
    action);
  return card;
}

function sessionKey(session) {
  return JSON.stringify([session.harness, session.session_id]);
}

async function refreshSessions() {
  if (pollingSessions) return;
  pollingSessions = true;
  try {
    const sessions = await invoke("list_sessions");
    const ids = new Set(sessions.map(sessionKey));
    for (const [id, card] of sessionCards) {
      if (ids.has(id)) continue;
      if (card.item.contains(document.activeElement)) byId("agents-heading").focus();
      card.item.remove();
      sessionCards.delete(id);
    }
    for (const session of sessions) {
      const key = sessionKey(session);
      let card = sessionCards.get(key);
      if (!card) {
        card = sessionCard(session);
        sessionCards.set(key, card);
        byId("sessions").append(card.item);
      }
      card.session = session;
      card.item.querySelector(".harness-icon").dataset.status = session.status;
      const project = folderName(session.project_folder);
      card.name.textContent = project;
      card.name.title = session.project_folder;
      card.harness.textContent = sessionHarnessNames.get(session.harness) ?? session.harness;
      card.status.dataset.status = session.status;
      card.statusText.data = sessionStatuses[session.status] ?? session.status;
      card.elapsed.textContent = elapsedTime(session.elapsed_seconds);
      card.elapsed.setAttribute("aria-label", `Elapsed: ${card.elapsed.textContent}`);
      card.tools.textContent = `${session.tool_count} ${session.tool_count === 1 ? "tool" : "tools"}`;
      card.action.setAttribute("aria-label", `Pending tickets for ${project} (${card.harness.textContent})`);
    }
    setMessage("sessions-message", sessions.length ? null : "No sessions detected. Install hooks in Session settings, then start or resume a session.");
    setMessage("sessions-error", null);
  } catch (error) {
    setMessage("sessions-message", null);
    setMessage("sessions-error", `Could not load sessions: ${error}`);
  } finally {
    pollingSessions = false;
  }
}

document.addEventListener("pad-section", ({ detail }) => {
  if (detail === "agents-panel") refreshSessions();
});
setInterval(() => {
  if (!document.hidden && !byId("agents-panel").hidden) refreshSessions();
}, 2000);

const ticketsDialog = byId("tickets-dialog");
const ticketsContent = byId("tickets-content");
const ticketsTitle = byId("tickets-title");
const ticketWarning = byId("implement-warning");
const ticketBack = byId("tickets-back");
const ticketRefresh = byId("tickets-refresh");
let ticketSession;
let ticketOpener;
let ticketsRequest = 0;
let proposedImplement;
let implementing = false;
let warningAcknowledged = false;
try {
  warningAcknowledged = localStorage.getItem("implement-warning-acknowledged") === "true";
} catch {
  // The notice can still be acknowledged for this window if storage is unavailable.
}

function openTickets(session, opener) {
  ticketSession = session;
  ticketOpener = opener;
  byId("tickets-project").textContent = session.project_folder;
  ticketsDialog.showModal();
  loadTicket();
}

function dismissImplementWarning(restoreFocus) {
  ticketWarning.hidden = true;
  ticketsContent.inert = byId("ticket-nav").inert = false;
  if (restoreFocus) proposedImplement?.trigger.focus();
  proposedImplement = null;
}

ticketsDialog.addEventListener("close", () => {
  // Ignore an earlier close event if the dialog has already been reopened.
  if (ticketsDialog.open) return;
  ++ticketsRequest;
  dismissImplementWarning(false);
  (ticketOpener?.isConnected ? ticketOpener : byId("agents-heading")).focus();
});

ticketsDialog.addEventListener("cancel", (event) => {
  if (ticketWarning.hidden) return;
  event.preventDefault();
  dismissImplementWarning(true);
});

function blockedReason(ticket) {
  const dependencies = ticket.blocked_by.ticket_numbers.map((number) => `#${number}`).join(", ");
  return `Blocked by ${dependencies || ticket.blocked_by.raw || "a pending ticket"}`;
}

function implementButton(ticket) {
  const trigger = button("Implement", "primary", () => {
    if (implementing) return;
    const args = { sessionId: ticketSession.session_id, ticketId: ticket.id };
    if (warningAcknowledged) sendImplement(args, trigger, ticket.number);
    else {
      proposedImplement = { args, trigger, number: ticket.number };
      ticketWarning.hidden = false;
      ticketsContent.inert = byId("ticket-nav").inert = true;
      byId("implement-cancel").focus();
    }
  });
  trigger.setAttribute("aria-label", `Implement ticket #${ticket.number}`);
  if (ticket.blocked) {
    trigger.disabled = true;
    trigger.title = blockedReason(ticket);
  }
  return trigger;
}

byId("implement-cancel").addEventListener("click", () => dismissImplementWarning(true));
byId("implement-continue").addEventListener("click", () => {
  if (!proposedImplement) return;
  const { args, trigger, number } = proposedImplement;
  warningAcknowledged = true;
  try {
    localStorage.setItem("implement-warning-acknowledged", "true");
  } catch {
    // Keep the acknowledgement for this window.
  }
  dismissImplementWarning(false);
  sendImplement(args, trigger, number);
});

async function sendImplement(args, trigger, number) {
  if (implementing) return;
  implementing = true;
  trigger.disabled = true;
  const request = ticketsRequest;
  setMessage("tickets-error", null);
  setMessage("implement-status", null);
  try {
    await invoke("implement_ticket", args);
    setMessage("implement-status", `Sent ticket #${number} to the focused app.`);
    if (ticketsDialog.open && request === ticketsRequest) ticketsDialog.close();
  } catch (error) {
    if (ticketsDialog.open && request === ticketsRequest) setMessage("tickets-error", error);
    else setMessage("sessions-error", error);
  } finally {
    implementing = false;
    trigger.disabled = false;
    if (ticketsDialog.open && request === ticketsRequest && trigger.isConnected
        && document.activeElement === document.body) trigger.focus();
  }
}

function ticketActions(ticket) {
  const action = implementButton(ticket);
  const controls = el("div", { className: "ticket-actions" }, action);
  if (ticket.blocked) controls.append(el("span", { className: "blocked", textContent: blockedReason(ticket) }));
  return controls;
}

function ticketRow(ticket) {
  const meta = el("span", { className: "ticket-meta" });
  if (ticket.status) {
    const tag = el("span", { className: "ticket-status", textContent: ticket.status });
    tag.dataset.status = ticket.status.toLowerCase();
    meta.append(tag);
  }
  meta.append(el("span", { className: "muted numerical", textContent: `${ticket.done}/${ticket.total}`, ariaLabel: `${ticket.done} of ${ticket.total} checklist items complete` }));
  const open = button("", "ticket-link", () => loadTicket(ticket.id));
  open.dataset.ticketId = ticket.id;
  open.append(el("span", { className: "muted", textContent: `#${ticket.number}` }),
    el("b", { textContent: ticket.title }), meta);
  return el("li", { className: "ticket-row" }, open, ticketActions(ticket));
}

let detailTicket;
async function loadTicket(ticketId, returnTo) {
  const request = ++ticketsRequest;
  const args = { sessionId: ticketSession.session_id, ticketId };
  detailTicket = ticketId;
  ticketBack.hidden = !ticketId;
  ticketRefresh.hidden = !!ticketId;
  ticketsTitle.textContent = ticketId ? "Ticket" : "Pending tickets";
  ticketsContent.replaceChildren();
  setMessage("tickets-error", null);
  setMessage("tickets-message", "Loading…");
  ticketsTitle.focus();
  try {
    if (ticketId) {
      const ticket = await invoke("ticket", args);
      if (!ticketsDialog.open || request !== ticketsRequest) return;
      ticketsTitle.textContent = `Ticket #${ticket.number}`;
      ticketsContent.append(ticketActions(ticket), renderMarkdown(ticket.markdown));
      setMessage("tickets-message", null);
    } else {
      const tickets = await invoke("pending_tickets", args);
      if (!ticketsDialog.open || request !== ticketsRequest) return;
      ticketsContent.append(el("ul", { className: "ticket-list" }, ...tickets.map(ticketRow)));
      setMessage("tickets-message", tickets.length ? null : "No pending tickets in this project.");
      if (returnTo && document.activeElement === ticketsTitle) {
        [...ticketsContent.querySelectorAll(".ticket-link")]
          .find((link) => link.dataset.ticketId === returnTo)?.focus();
      }
    }
  } catch (error) {
    if (!ticketsDialog.open || request !== ticketsRequest) return;
    setMessage("tickets-message", null);
    setMessage("tickets-error", error);
  }
}

ticketRefresh.addEventListener("click", () => loadTicket());
ticketBack.addEventListener("click", () => loadTicket(undefined, detailTicket));
