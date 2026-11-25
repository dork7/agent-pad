const directoryForm = byId("directory-form");
const directoryHarness = byId("directory-harness");
const pickDirectory = byId("pick-directory");
const customForm = byId("custom-form");
const networkForm = byId("network-form");
const networkAddress = byId("network-address");
const templateEditors = new Map();

let inventoryRequest = 0;
let inventoryBusy = false;
let inventoryKey;
let pickedDirectory = null;
let networkRequest = 0;
let networkBusy = false;

function folderName(path) {
  const separator = path.startsWith("/") ? /\/+/ : /[\\/]+/;
  return path.split(separator).filter(Boolean).at(-1) ?? path;
}

function listSetting(name, detail, action) {
  const row = document.createElement("li");
  const description = document.createElement("div");
  const heading = document.createElement("b");
  heading.textContent = name;
  const text = document.createElement("div");
  text.className = "muted path";
  text.textContent = detail;
  description.append(heading, text);
  row.append(description, action);
  return row;
}

function renderDirectories(directories, names, templates) {
  byId("directories-empty").hidden = directories.length > 0;
  byId("directories").replaceChildren(...directories.map(({ id, path, harness, skill_count }) => {
    const name = folderName(path);
    const count = `${skill_count} ${skill_count === 1 ? "skill" : "skills"}`;
    const remove = button("Remove", "danger", () =>
      changeInventory("remove_directory", { id }, byId("directories-error"), () => pickDirectory.focus()));
    remove.setAttribute("aria-label", `Remove skill directory ${path}`);
    return listSetting(name, `${names.get(harness) ?? harness} · ${count} · ${path}`, remove);
  }));

  const previous = directoryHarness.value;
  const options = [new Option("Choose a harness…", ""), ...templates.map(([id, name]) => new Option(name, id))];
  options[0].disabled = true;
  directoryHarness.replaceChildren(...options);
  directoryHarness.value = names.has(previous) ? previous : "";
}

function renderCustomHarnesses(customHarnesses) {
  byId("custom-empty").hidden = customHarnesses.length > 0;
  byId("custom-harnesses").replaceChildren(...customHarnesses.map(({ id, name, template }) => {
    const remove = button("Remove", "danger", () =>
      changeInventory("remove_custom_harness", { id }, byId("custom-error"), () => byId("custom-name").focus()));
    remove.setAttribute("aria-label", `Remove harness ${name}`);
    return listSetting(name, template, remove);
  }));
}

function createTemplateEditor(id, name, isCustom) {
  const form = document.createElement("form");
  const row = document.createElement("div");
  row.className = "form-row";
  const label = document.createElement("label");
  label.className = "field";
  label.textContent = name;
  const input = document.createElement("input");
  input.type = "text";
  input.required = true;
  input.autocomplete = "off";
  input.spellcheck = false;
  const error = document.createElement("p");
  error.id = `template-error-${id}`;
  error.className = "error";
  error.setAttribute("role", "alert");
  error.hidden = true;
  input.setAttribute("aria-describedby", `template-hint ${error.id}`);
  label.append(input);
  const save = document.createElement("button");
  save.className = "btn";
  save.type = "submit";
  save.textContent = "Save";
  save.setAttribute("aria-label", `Save ${name} template`);
  row.append(label, save);
  const editor = { form, input, saved: null };

  async function changeTemplate(command, args) {
    await changeInventory(command, args, error, () => {
      editor.saved = null;
    });
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    changeTemplate("set_template", { harness: id, template: input.value });
  });
  if (!isCustom) {
    const reset = button("Reset", "", () => changeTemplate("reset_template", { harness: id }));
    reset.setAttribute("aria-label", `Reset ${name} template`);
    row.append(reset);
  }
  form.append(row, error);
  byId("templates").append(form);
  return editor;
}

function renderTemplates(templates, customHarnesses) {
  const custom = new Set(customHarnesses.map(({ id }) => id));
  const ids = new Set(templates.map(([id]) => id));
  for (const [id, editor] of templateEditors) {
    if (!ids.has(id)) {
      editor.form.remove();
      templateEditors.delete(id);
    }
  }
  for (const [id, name, template] of templates) {
    let editor = templateEditors.get(id);
    if (!editor) {
      editor = createTemplateEditor(id, name, custom.has(id));
      templateEditors.set(id, editor);
    }
    if (editor.saved === null || editor.input.value === editor.saved) editor.input.value = template;
    editor.saved = template;
  }
}

async function refreshInventory() {
  if (inventoryBusy) return;
  const request = ++inventoryRequest;
  try {
    const [chips, directories, templates, settings] = await Promise.all([
      invoke("list_chips"),
      invoke("list_directories"),
      invoke("list_templates"),
      invoke("settings"),
    ]);
    if (request !== inventoryRequest) return;
    const names = new Map(templates.map(([id, name]) => [id, name]));
    const labels = new Map([
      ...templates.map(([id, name]) => [`h:${id}`, name]),
      ...directories.map(({ id, path }) => [`d:${id}`, folderName(path)]),
    ]);
    renderPalette(chips, labels);
    setMessage("chip-message", chips.length ? null : "No chips available.");
    const key = JSON.stringify([directories, templates, settings.custom_harnesses]);
    if (key !== inventoryKey) {
      renderDirectories(directories, names, templates);
      renderCustomHarnesses(settings.custom_harnesses);
      if (inventoryKey === undefined) pickDirectory.disabled = false;
      inventoryKey = key;
    }
    renderTemplates(templates, settings.custom_harnesses);
    customForm.querySelector("fieldset").disabled = false;
    setMessage("settings-error", null);
    await refreshPhones();
  } catch (error) {
    if (request === inventoryRequest) setMessage("settings-error", `Could not load chips and settings: ${error}`);
  }
}

async function changeInventory(command, args, error, onSaved) {
  if (inventoryBusy) return;
  inventoryBusy = true;
  ++inventoryRequest;
  setMessage(error.id, null);
  try {
    await invoke(command, args);
    onSaved();
  } catch (failure) {
    setMessage(error.id, failure);
  } finally {
    inventoryBusy = false;
    await refreshInventory();
  }
}

pickDirectory.addEventListener("click", async () => {
  pickDirectory.disabled = true;
  setMessage("directories-error", null);
  try {
    const path = await invoke("plugin:dialog|open", {
      options: { title: "Choose a Skill Directory", directory: true, multiple: false },
    });
    if (path === null) return;
    pickedDirectory = path;
    byId("directory-path").textContent = path;
    directoryHarness.value = "";
    byId("directory-assignment").hidden = false;
    directoryHarness.focus();
  } catch (error) {
    setMessage("directories-error", error);
  } finally {
    pickDirectory.disabled = false;
  }
});

function closeDirectoryAssignment() {
  pickedDirectory = null;
  byId("directory-assignment").hidden = true;
  pickDirectory.focus();
}

byId("cancel-directory").addEventListener("click", closeDirectoryAssignment);
directoryForm.addEventListener("submit", (event) => {
  event.preventDefault();
  if (!pickedDirectory) return;
  changeInventory("add_directory", { path: pickedDirectory, harness: directoryHarness.value },
    byId("directories-error"), closeDirectoryAssignment);
});

customForm.addEventListener("submit", (event) => {
  event.preventDefault();
  changeInventory("add_custom_harness", {
    name: byId("custom-name").value,
    template: byId("custom-template").value,
  }, byId("custom-error"), () => customForm.reset());
});

async function refreshNetwork(force = false) {
  if (networkBusy) return;
  const request = ++networkRequest;
  try {
    const override = await invoke("sharing_address_override");
    if (request !== networkRequest) return;
    const saved = override ?? "";
    if (force || networkAddress.value === networkAddress.defaultValue) networkAddress.value = saved;
    networkAddress.defaultValue = saved;
    networkForm.querySelector("fieldset").disabled = false;
    try {
      const address = await invoke("sharing_address");
      if (request === networkRequest) {
        byId("network-current").textContent = override
          ? `Sharing address with override: ${address}`
          : `Automatic address: ${address}`;
      }
    } catch (error) {
      if (request === networkRequest) byId("network-current").textContent = `Sharing address unavailable: ${error}`;
    }
  } catch (error) {
    if (request === networkRequest) setMessage("network-error", error);
  }
}

async function saveNetwork(address) {
  if (networkBusy) return;
  networkBusy = true;
  ++networkRequest;
  setMessage("network-error", null);
  let saved = false;
  try {
    await invoke("set_sharing_address_override", { address });
    saved = true;
  } catch (error) {
    setMessage("network-error", error);
  } finally {
    networkBusy = false;
    await refreshNetwork(saved);
  }
}

networkForm.addEventListener("submit", (event) => {
  event.preventDefault();
  saveNetwork(networkAddress.value);
});
byId("clear-network").addEventListener("click", () => saveNetwork(null));

window.addEventListener("focus", () => {
  refreshInventory();
  refreshNetwork();
});
refreshInventory();
refreshNetwork();
