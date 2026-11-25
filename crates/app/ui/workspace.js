function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

const sectionButtons = [...document.querySelectorAll(".pad-nav button")];

function showPadSection(panel) {
  for (const tab of sectionButtons) {
    const selected = tab.getAttribute("aria-controls") === panel;
    tab.setAttribute("aria-pressed", String(selected));
    byId(tab.getAttribute("aria-controls")).hidden = !selected;
  }
  window.scrollTo(0, 0);
  document.dispatchEvent(new CustomEvent("pad-section", { detail: panel }));
}

for (const tab of sectionButtons) {
  tab.addEventListener("click", () => showPadSection(tab.getAttribute("aria-controls")));
}

const profiles = byId("profiles");
const profileMenu = byId("profile-menu");
const profileForm = byId("profile-form");
const profileName = byId("profile-name");
const profileSubmit = byId("profile-submit");
let profileNames = [];
let activeProfile = "";
let profileMode = null;
let profileBusy = false;
let profileReturnFocus;

function syncProfileControls() {
  profiles.disabled = profileBusy || !profileNames.length || !!profileMode;
  byId("save-profile").disabled = profileBusy || !!profileMode;
  profileMenu.hidden = !activeProfile || !!profileMode || profileBusy;
  profileName.readOnly = profileBusy;
  profileSubmit.disabled = byId("profile-cancel").disabled = profileBusy;
}

async function loadProfiles() {
  const [saved, active] = await invoke("list_profiles");
  profileNames = saved.map(({ name }) => name);
  activeProfile = active ?? "";
  const placeholder = new Option(profileNames.length ? "Choose profile" : "No profiles", "");
  placeholder.disabled = true;
  profiles.replaceChildren(placeholder, ...profileNames.map((name) => new Option(name, name)));
  profiles.value = activeProfile;
  profiles.title = activeProfile;
}

async function changeProfile(command, args, message) {
  if (profileBusy) return false;
  profileBusy = true;
  syncProfileControls();
  setMessage("profile-error", null);
  try {
    await invoke(command, args);
    await loadProfiles();
    byId("profile-status").textContent = message;
    if (command === "switch_profile") {
      await refreshPhones();
    }
    return true;
  } catch (error) {
    profiles.value = activeProfile;
    setMessage("profile-error", error);
    return false;
  } finally {
    profileBusy = false;
    syncProfileControls();
  }
}

profiles.addEventListener("change", async () => {
  const name = profiles.value;
  await changeProfile("switch_profile", { name }, `Switched to ${name}.`);
  if (document.activeElement === document.body) profiles.focus();
});

function updateProfileForm() {
  const deleting = profileMode === "delete";
  const replacing = profileMode === "save" && profileNames.includes(profileName.value.trim());
  byId("profile-name-field").hidden = deleting;
  profileName.required = !deleting;
  setMessage("profile-confirm", deleting ? `Delete “${activeProfile}”?`
    : replacing ? `Replace “${profileName.value.trim()}” with the current phone Views?` : null);
  profileSubmit.textContent = deleting ? "Delete profile" : replacing ? "Replace profile"
    : profileMode === "rename" ? "Rename profile" : "Save profile";
  profileSubmit.classList.toggle("danger", deleting);
}

function editProfile(mode) {
  profileMode = mode;
  profileReturnFocus = mode === "save" ? byId("save-profile") : profileMenu.querySelector("summary");
  profileMenu.open = false;
  profileName.value = mode === "rename" ? activeProfile : "";
  updateProfileForm();
  setMessage("profile-error", null);
  profileForm.hidden = false;
  syncProfileControls();
  (mode === "delete" ? byId("profile-cancel") : profileName).focus();
  if (mode !== "delete") profileName.select();
}

function closeProfileForm() {
  profileMode = null;
  profileForm.hidden = true;
  syncProfileControls();
  (profileReturnFocus.closest("[hidden]") ? byId("save-profile") : profileReturnFocus).focus();
}

for (const mode of ["save", "rename", "delete"]) {
  byId(`${mode}-profile`).addEventListener("click", () => editProfile(mode));
}
profileName.addEventListener("input", updateProfileForm);
byId("profile-cancel").addEventListener("click", closeProfileForm);
profileForm.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !profileBusy) {
    event.preventDefault();
    closeProfileForm();
  }
});
profileForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const mode = profileMode;
  const input = profileName.value.trim();
  const args = mode === "save" ? { name: input } : { name: activeProfile };
  if (mode === "rename") args.newName = input;
  const name = mode === "delete" ? activeProfile : input;
  if (await changeProfile(`${mode}_profile`, args, `Profile “${name}” ${mode}d.`)) closeProfileForm();
  else if (document.activeElement === document.body) profileSubmit.focus();
});

loadProfiles().then(syncProfileControls).catch((error) => setMessage("profile-error", error));
