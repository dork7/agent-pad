function savedTextEditor({ kind, container, errorId, statusId, scope, onChange, reorder = false }) {
  let entries = [];
  let loadRequest = 0;
  let editing = null;
  let busy = false;
  let draggedId = null;
  const plural = `${kind}s`;
  const list = el("ol", { className: "saved-list", ariaLabel: plural });
  const empty = el("p", { className: "muted", textContent: `No ${plural} saved.` });
  const title = reorder && el("input", { required: true, autocomplete: "off" });
  const text = reorder
    ? el("textarea", { required: true, rows: 5 })
    : el("input", { required: true, autocomplete: "off" });
  const fields = el("fieldset", {});
  if (title) fields.append(el("label", { className: "field" }, "Title", title));
  fields.append(el("label", { className: "field" }, reorder ? "Text" : "Preset text", text));
  const submit = el("button", { type: "submit", className: "btn primary" });
  const cancel = button("Cancel", "", () => {
    const previous = editing;
    closeEditor();
    focusEntry(previous, 0);
  });
  fields.append(el("div", { className: "form-row" }, submit, cancel));
  const form = el("form", { className: "saved-form", hidden: true }, fields);
  const add = button(`Add ${kind}…`, "", () => edit(null));
  container.append(list, empty, el("div", { className: "form-row" }, add), form);

  function edit(entry) {
    if (busy) return;
    editing = entry?.id ?? null;
    setMessage(errorId, null);
    setMessage(statusId, null);
    text.value = entry?.text ?? "";
    if (title) title.value = entry?.title ?? "";
    submit.textContent = editing ? "Save changes" : `Add ${kind}`;
    form.hidden = false;
    (title || text).focus();
  }

  function closeEditor() {
    editing = null;
    form.hidden = true;
    form.reset();
  }

  function focusEntry(id, index, action = "edit") {
    const row = [...list.children].find((item) => item.dataset.id === id) ?? list.children[index];
    const control = row?.querySelector(`[data-action="${action}"]:enabled`)
      ?? row?.querySelector('[data-action="edit"]') ?? add;
    control.focus();
  }

  function clearDrop() {
    draggedId = null;
    list.querySelector(".over")?.classList.remove("over");
  }

  function render() {
    clearDrop();
    empty.hidden = entries.length > 0;
    list.replaceChildren(...entries.map((entry, index) => {
      const display = reorder ? entry.title : entry.text;
      const describe = `${kind} ${display}`;
      const editButton = button("Edit", "", () => edit(entry));
      editButton.dataset.action = "edit";
      editButton.setAttribute("aria-label", `Edit ${describe}`);
      const remove = button("Delete", "danger", () => mutate("delete", { id: entry.id },
        `Deleted ${kind}.`, () => {
          if (editing === entry.id) closeEditor();
          focusEntry(null, Math.min(index, list.children.length - 1));
        }));
      remove.setAttribute("aria-label", `Delete ${describe}`);
      const description = el("div", { className: "saved-description" }, el("b", { textContent: display }));
      const actions = el("div", { className: "saved-actions" }, editButton, remove);
      const item = el("li", { className: "saved-row" }, description, actions);
      item.dataset.id = entry.id;
      if (reorder) {
        description.append(el("p", { className: "muted snippet-text", textContent: entry.text }));
        const move = (label, offset) => {
          const control = button(label, "", () => moveEntry(entry.id, index + offset, label));
          control.setAttribute("aria-label", `Move ${display} ${label.toLowerCase()}`);
          control.dataset.action = label;
          control.disabled = index + offset < 0 || index + offset >= entries.length;
          return control;
        };
        actions.prepend(move("Up", -1), move("Down", 1));
        description.draggable = true;
        description.addEventListener("dragstart", (event) => {
          if (busy) {
            event.preventDefault();
            return;
          }
          draggedId = entry.id;
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("text/plain", entry.id);
        });
        description.addEventListener("dragend", clearDrop);
      }
      return item;
    }));
  }

  function moveEntry(id, index, action) {
    return mutate("move", { id, index }, "Snippet moved.", () => {
      if (action) focusEntry(id, index, action);
    });
  }

  if (reorder) {
    list.addEventListener("dragover", (event) => {
      if (!draggedId || busy) return;
      const target = event.target.closest(".saved-row");
      list.querySelector(".over")?.classList.remove("over");
      if (!target || target.dataset.id === draggedId) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      target.classList.add("over");
    });
    list.addEventListener("dragleave", (event) => {
      if (!list.contains(event.relatedTarget)) list.querySelector(".over")?.classList.remove("over");
    });
    list.addEventListener("drop", (event) => {
      const target = event.target.closest(".saved-row");
      if (!target || !draggedId || busy) return;
      event.preventDefault();
      const id = draggedId;
      clearDrop();
      if (id !== target.dataset.id) moveEntry(id, [...list.children].indexOf(target));
    });
  }

  async function load() {
    const request = ++loadRequest;
    const args = scope();
    const data = await invoke(`list_${plural}`, args);
    if (request === loadRequest && scope().skillId === args.skillId) {
      entries = data;
      render();
    }
  }

  async function mutate(action, args, message, onSuccess) {
    if (busy) return;
    busy = true;
    const focused = document.activeElement;
    fields.disabled = true;
    setMessage(errorId, null);
    setMessage(statusId, null);
    onChange?.(true);
    try {
      await invoke(`${action}_${kind}`, args);
      await load();
      setMessage(statusId, message);
      onSuccess();
    } catch (error) {
      setMessage(errorId, error);
    } finally {
      busy = false;
      fields.disabled = false;
      onChange?.(false);
      if (document.activeElement === document.body && focused?.isConnected) focused.focus();
    }
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const args = editing ? { id: editing, text: text.value } : { ...scope(), text: text.value };
    if (title) args.title = title.value;
    const previous = editing;
    mutate(editing ? "update" : "add", args, `Saved ${kind}.`, () => {
      closeEditor();
      focusEntry(previous, list.children.length - 1);
    });
  });
  form.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !busy) {
      event.preventDefault();
      cancel.click();
    }
  });

  return { load, closeEditor };
}

const skillSearch = byId("skill-search");
const skillSelect = byId("skill-select");
const refreshSkillsButton = byId("refresh-skills");
const presetsPanel = byId("presets");
let skillInventory = [];
let skillId = "";
let skillsLoaded = false;
let skillsBusy = false;
let presetBusy = false;
let presetRequest = 0;

function syncSkillControls() {
  const busy = skillsBusy || presetBusy;
  refreshSkillsButton.disabled = busy;
  skillSearch.readOnly = busy;
  skillSelect.disabled = busy || !skillSelect.options.length;
}

const presets = savedTextEditor({
  kind: "preset",
  container: byId("preset-list"),
  errorId: "skills-error",
  statusId: "skills-status",
  scope: () => ({ skillId }),
  onChange: (busy) => {
    presetBusy = busy;
    syncSkillControls();
  },
});

let snippetsLoaded = false;
const snippets = savedTextEditor({
  kind: "snippet",
  container: byId("snippet-list"),
  errorId: "snippets-error",
  statusId: "snippets-status",
  scope: () => ({}),
  onChange: (busy) => { if (!busy) refreshInventory(); },
  reorder: true,
});

async function selectSkill(next, reload = false) {
  if (!reload && next === skillId) return;
  skillId = next;
  const request = ++presetRequest;
  presetsPanel.hidden = true;
  presets.closeEditor();
  setMessage("skills-error", null);
  setMessage("skills-status", null);
  if (!next) return;
  try {
    await presets.load();
    if (request === presetRequest) presetsPanel.hidden = false;
  } catch (error) {
    if (request === presetRequest) setMessage("skills-error", error);
  }
}

function filterSkills(reload) {
  const query = skillSearch.value.trim().toLowerCase();
  const matches = skillInventory.filter((skill) =>
    [skill.name, skill.invocation, skill.description, skill.label]
      .some((value) => value.toLowerCase().includes(query)));
  const placeholder = new Option(matches.length ? "Choose a skill" : "No matching skills", "");
  placeholder.disabled = true;
  skillSelect.replaceChildren(placeholder, ...matches.map((skill) => new Option(
    `${skill.name} · ${skill.label}${skill.directory ? ` · ${chipLabel(`d:${skill.directory}`)}` : ""}`, skill.id,
  )));
  skillSelect.value = matches.some(({ id }) => id === skillId) ? skillId : "";
  syncSkillControls();
  selectSkill(skillSelect.value, reload);
}

async function loadSkills() {
  if (skillsBusy || presetBusy) return;
  skillsBusy = true;
  syncSkillControls();
  setMessage("skills-error", null);
  try {
    const [skills, templates] = await Promise.all([invoke("list_skills"), invoke("list_templates")]);
    const names = new Map(templates.map(([id, name]) => [id, name]));
    skillInventory = skills.map((skill) => ({ ...skill, label: names.get(skill.harness) ?? skill.harness }));
    skillsLoaded = true;
    filterSkills(true);
  } catch (error) {
    setMessage("skills-error", error);
  } finally {
    skillsBusy = false;
    syncSkillControls();
  }
}

skillSearch.addEventListener("input", () => filterSkills(false));
skillSelect.addEventListener("change", () => selectSkill(skillSelect.value));
refreshSkillsButton.addEventListener("click", async () => {
  await loadSkills();
  if (document.activeElement === document.body) refreshSkillsButton.focus();
});

document.addEventListener("pad-section", ({ detail }) => {
  if (detail !== "saved-text-panel") return;
  if (!skillsLoaded) loadSkills();
  if (!snippetsLoaded) {
    snippetsLoaded = true;
    snippets.load().catch((error) => {
      snippetsLoaded = false;
      setMessage("snippets-error", error);
    });
  }
});
