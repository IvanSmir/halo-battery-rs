// The settings window. Every change is saved at once to config.json, which
// the tray applies; the device list comes from the tray's devices.json and
// updates live.

const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

const COLORS = { green: "#22d36b", amber: "#ffb020", red: "#ff3b4e" };
// the same colours the tray uses (icon::palette::ring_color)
const RING_COLORS = [
  { id: "auto", c: "#eef2f8", label: "A" },
  { id: "blue", c: "#2f8cff" },
  { id: "violet", c: "#a78bfa" },
  { id: "mint", c: "#2dd4bf" },
  { id: "rose", c: "#f472b6" },
];
const INTERVALS = [15, 30, 60, 120, 300];
const KIND_LABEL = { mouse: "Ratón", keyboard: "Teclado", headset: "Auriculares", gamepad: "Mando" };

// small line icons
const ICON = {
  mouse: '<rect x="7" y="3" width="10" height="18" rx="5"/><path d="M12 3v6"/>',
  keyboard: '<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8"/>',
  headset: '<path d="M4 15v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="14" width="4" height="6" rx="1.5"/><rect x="17" y="14" width="4" height="6" rx="1.5"/>',
  gamepad: '<path d="M6 8h12a4 4 0 0 1 3.9 4.9l-1 4a2.5 2.5 0 0 1-4.3 1L15 16H9l-1.6 1.9a2.5 2.5 0 0 1-4.3-1l-1-4A4 4 0 0 1 6 8z"/><path d="M8 11v3M6.5 12.5h3"/><circle cx="16" cy="12" r=".6"/>',
  pencil: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>',
  bolt: '<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>',
  dots: '<circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/>',
};
const icon = (name, cls = "") => `<svg viewBox="0 0 24 24" class="${cls}">${ICON[name]}</svg>`;
const escape = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

/** The app state: the saved config and the tray's last reading. */
const state = { config: null, devices: [] };

// ---------------------------------------------------------------- config

const DEVICE_DEFAULTS = { alias: "", visible: true, notify: true };

function deviceSettings(key) {
  return { ...DEVICE_DEFAULTS, ...(state.config.devices[key] ?? {}) };
}

function setDevice(key, patch) {
  state.config.devices[key] = { ...deviceSettings(key), ...patch };
}

let saveTimer = null;

/** Saves the config; `delay` batches keystrokes while a name is typed. */
function save(delay = 0) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    try {
      await invoke("save_config", { config: state.config });
    } catch (e) {
      console.error("could not save the settings", e);
    }
  }, delay);
}

// ---------------------------------------------------------------- rings

function ringColor(level, charging) {
  const low = state.config.low_threshold;
  if (charging) return COLORS.green;
  if (level != null && level <= Math.max(low, 10)) return COLORS.red;
  if (level != null && level <= Math.max(low, 10) + 10) return COLORS.amber;
  return "var(--ring)";
}

function ring(level, charging, online) {
  const cls = ["ring", charging && online ? "charging" : "", online ? "" : "asleep"].join(" ");
  return `
    <div class="${cls}" data-level="${level ?? 0}" data-charging="${charging}"
         style="--color:${ringColor(level, charging)}">
      <svg viewBox="0 0 36 36">
        <circle class="track" cx="18" cy="18" r="15.5" pathLength="100" />
        <circle class="arc" cx="18" cy="18" r="15.5" pathLength="100" />
      </svg>
      <div class="label">${level == null ? "–" : level + "%"}</div>
    </div>`;
}

/** Starts rings at zero (or where they were) and lets CSS animate them to their level. */
function fill(root) {
  requestAnimationFrame(() =>
    requestAnimationFrame(() => $$(".ring", root).forEach((r) => r.style.setProperty("--level", r.dataset.level))),
  );
}

function recolorRings() {
  $$(".ring").forEach((r) => r.style.setProperty("--color", ringColor(+r.dataset.level, r.dataset.charging === "true")));
}

function applyRingColor() {
  const c = RING_COLORS.find((s) => s.id === state.config.appearance.ring_color) ?? RING_COLORS[0];
  document.documentElement.style.setProperty("--ring", c.c);
}

// ---------------------------------------------------------------- navigation

$$(".nav").forEach((btn) =>
  btn.addEventListener("click", () => {
    $$(".nav").forEach((b) => b.classList.toggle("active", b === btn));
    $$(".page").forEach((p) => p.classList.toggle("active", p.id === "page-" + btn.dataset.page));
    const page = $("#page-" + btn.dataset.page);
    $$(".ring", page).forEach((r) => r.style.setProperty("--level", 0));
    fill(page);
  }),
);

// ---------------------------------------------------------------- window controls

const win = tauri.window.getCurrentWindow();
$("#win-min").onclick = () => win.minimize();
$("#win-max").onclick = () => win.toggleMaximize();
$("#win-close").onclick = () => win.close();

// ---------------------------------------------------------------- devices

function statusPill(d) {
  if (!d.online) return `<span class="pill" style="--c:#8f9bb0"><span class="led"></span>Dormido</span>`;
  if (d.charging) return `<span class="pill" style="--c:${COLORS.amber}">${icon("bolt")}Cargando</span>`;
  return `<span class="pill" style="--c:${COLORS.green}"><span class="led"></span>Conectado</span>`;
}

function deviceCard(d, i) {
  const s = deviceSettings(d.key);
  return `
    <div class="glass card device ${s.visible ? "" : "hidden-in-tray"}" data-key="${escape(d.key)}"
         style="animation-delay:${i * 80}ms">
      ${ring(d.level, d.charging, d.online)}
      <div class="info">
        <div class="name-row">
          <input class="name" placeholder="${escape(d.name)}" value="${escape(s.alias)}" maxlength="40" spellcheck="false" />
          <button class="edit" title="Renombrar">${icon("pencil")}</button>
        </div>
        <div class="meta">${icon(d.kind)} ${KIND_LABEL[d.kind] ?? ""} · detectado como «${escape(d.name)}»</div>
        ${statusPill(d)}
      </div>
      <div class="vline"></div>
      <div class="toggles">
        <label>Bandeja <input type="checkbox" class="switch" data-field="visible" ${s.visible ? "checked" : ""} /></label>
        <label>Avisos <input type="checkbox" class="switch" data-field="notify" ${s.notify ? "checked" : ""} /></label>
      </div>
      <div class="vline"></div>
      <button class="more" title="Más opciones"><svg viewBox="0 0 24 24">${ICON.dots}</svg></button>
    </div>`;
}

function bindCard(card) {
  const key = card.dataset.key;
  const name = $(".name", card);
  name.addEventListener("input", () => {
    setDevice(key, { alias: name.value });
    save(400);
  });
  name.addEventListener("keydown", (e) => e.key === "Enter" && name.blur());
  $(".edit", card).addEventListener("click", () => {
    name.focus();
    name.select();
  });
  $$(".switch", card).forEach((sw) =>
    sw.addEventListener("change", () => {
      setDevice(key, { [sw.dataset.field]: sw.checked });
      card.classList.toggle("hidden-in-tray", !deviceSettings(key).visible);
      save();
    }),
  );
  $(".more", card).addEventListener("click", (e) => openMenu(e, card));
}

/** Patches the cards in place when the same devices are shown (so a name
 *  being typed survives), rebuilds the list when devices come or go. */
function renderDevices(devices) {
  const list = $("#device-list");
  const same = devices.map((d) => d.key).join() === state.devices.map((d) => d.key).join() && list.children.length;
  state.devices = devices;
  $("#device-count").textContent = devices.length === 1 ? "1 dispositivo" : `${devices.length} dispositivos`;

  if (!devices.length) {
    list.innerHTML = `
      <div class="glass card empty">
        <div class="title">No se ha encontrado ningún dispositivo</div>
        <div class="desc">Enciende tu ratón o tu mando; aparecerán aquí en cuanto la bandeja los detecte.</div>
      </div>`;
    return;
  }

  if (same) {
    devices.forEach((d) => {
      const card = list.querySelector(`[data-key="${CSS.escape(d.key)}"]`);
      const old = $(".ring", card);
      const from = old.dataset.level;
      old.outerHTML = ring(d.level, d.charging, d.online);
      // animate from the previous level, not from empty
      $(".ring", card).style.setProperty("--level", from);
      $(".pill", card).outerHTML = statusPill(d);
    });
    fill(list);
    return;
  }

  list.innerHTML = devices.map(deviceCard).join("");
  $$(".device", list).forEach(bindCard);
  fill(list);
}

// ---------------------------------------------------------------- device menu

const menu = $("#device-menu");
let menuCard = null;

function openMenu(e, card) {
  e.stopPropagation();
  if (menuCard === card) return closeMenu();
  closeMenu();
  menuCard = card;
  const button = $(".more", card);
  button.classList.add("open");
  $('[data-action="toggle-tray"] span', menu).textContent = deviceSettings(card.dataset.key).visible
    ? "Ocultar de la bandeja"
    : "Mostrar en la bandeja";
  const r = button.getBoundingClientRect();
  menu.hidden = false;
  menu.style.top = r.bottom + 8 + "px";
  menu.style.left = r.right - menu.offsetWidth + "px";
}

function closeMenu() {
  menu.hidden = true;
  menuCard && $(".more", menuCard).classList.remove("open");
  menuCard = null;
}

menu.addEventListener("click", (e) => {
  const action = e.target.closest("button")?.dataset.action;
  const card = menuCard;
  closeMenu();
  if (!card) return;
  const key = card.dataset.key;
  if (action === "rename") {
    $(".name", card).focus();
  } else if (action === "toggle-tray") {
    const sw = $('[data-field="visible"]', card);
    sw.checked = !sw.checked;
    sw.dispatchEvent(new Event("change"));
  } else if (action === "reset") {
    delete state.config.devices[key];
    save();
    const d = state.devices.find((x) => x.key === key);
    const fresh = document.createElement("div");
    fresh.innerHTML = deviceCard(d, 0);
    const replacement = fresh.firstElementChild;
    replacement.style.animation = "none";
    card.replaceWith(replacement);
    bindCard(replacement);
    fill(replacement);
  }
});
document.addEventListener("click", closeMenu);
window.addEventListener("resize", closeMenu);
$(".content").addEventListener("scroll", closeMenu);

// ---------------------------------------------------------------- notifications

const slider = $("#threshold");

function showThreshold() {
  $("#threshold-value").textContent = slider.value + "%";
  slider.style.setProperty("--fill", ((slider.value - slider.min) / (slider.max - slider.min)) * 100 + "%");
}

slider.addEventListener("input", () => {
  state.config.low_threshold = +slider.value;
  showThreshold();
  recolorRings();
  save(250);
});

/** Binds a switch to a boolean inside the config. */
function bindSwitch(id, get, set) {
  const sw = $("#" + id);
  sw.checked = get();
  sw.addEventListener("change", () => {
    set(sw.checked);
    save();
  });
}

// ---------------------------------------------------------------- appearance

function renderPreviews() {
  $("#previews").innerHTML = [ring(84, false, true), ring(28, false, true), ring(12, false, true), ring(60, true, true)].join("");
}

function renderSwatches() {
  const current = state.config.appearance.ring_color;
  $("#swatches").innerHTML = RING_COLORS.map(
    (s) =>
      `<button class="swatch ${current === s.id ? "active" : ""}" style="--c:${s.c}" data-id="${s.id}"
               title="${s.id === "auto" ? "Según la barra de tareas" : ""}">${s.label ?? ""}</button>`,
  ).join("");
  $$(".swatch").forEach((b) =>
    b.addEventListener("click", () => {
      state.config.appearance.ring_color = b.dataset.id;
      applyRingColor();
      renderSwatches();
      save();
    }),
  );
}

// ---------------------------------------------------------------- general

function renderIntervals() {
  const current = state.config.interval_secs;
  $("#intervals").innerHTML = INTERVALS.map(
    (s) => `<button class="${s === current ? "active" : ""}" data-s="${s}">${s < 60 ? s + " s" : s / 60 + " min"}</button>`,
  ).join("");
  $$("#intervals button").forEach((b) =>
    b.addEventListener("click", () => {
      state.config.interval_secs = +b.dataset.s;
      renderIntervals();
      save();
    }),
  );
}

// ---------------------------------------------------------------- tray status

function showTray(running) {
  $("#tray-banner").hidden = running;
}

$("#start-tray").addEventListener("click", () => invoke("start_tray").catch(console.error));

// ---------------------------------------------------------------- start

async function start() {
  const initial = await invoke("load");
  state.config = initial.config;
  const cfg = state.config;

  applyRingColor();
  slider.value = cfg.low_threshold;
  showThreshold();
  bindSwitch("notify-enabled", () => cfg.notifications.enabled, (v) => (cfg.notifications.enabled = v));
  bindSwitch("full-charge", () => cfg.notifications.full_charge, (v) => (cfg.notifications.full_charge = v));
  bindSwitch("sound", () => cfg.notifications.sound, (v) => (cfg.notifications.sound = v));
  bindSwitch("pictogram", () => cfg.appearance.pictogram, (v) => (cfg.appearance.pictogram = v));
  bindSwitch("animate", () => cfg.appearance.animate, (v) => {
    cfg.appearance.animate = v;
    document.body.classList.toggle("no-anim", !v);
  });
  document.body.classList.toggle("no-anim", !cfg.appearance.animate);

  const autostart = $("#autostart");
  autostart.checked = initial.autostart;
  autostart.addEventListener("change", async () => {
    autostart.checked = await invoke("set_autostart", { enabled: autostart.checked });
  });

  renderPreviews();
  renderSwatches();
  renderIntervals();
  showTray(initial.tray_running);
  renderDevices(initial.devices);

  tauri.event.listen("devices", (e) => renderDevices(e.payload));
  tauri.event.listen("tray", (e) => showTray(e.payload));
}

start().catch((e) => console.error("could not load the settings", e));
