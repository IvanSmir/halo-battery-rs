// Settings window prototype. Nothing is saved: the device list is real, the
// rest only changes what is shown.

const tauri = window.__TAURI__;
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

const COLORS = { green: "#22d36b", amber: "#ffb020", red: "#ff3b4e" };
const RING_COLORS = [
  { id: "auto", c: "#eef2f8", label: "A" },
  { id: "blue", c: "#2f8cff" },
  { id: "violet", c: "#a78bfa" },
  { id: "mint", c: "#2dd4bf" },
  { id: "rose", c: "#f472b6" },
];
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

const state = { threshold: 20, ring: "blue", devices: [] };

function ringColor(level) {
  if (level != null && level <= state.threshold) return COLORS.red;
  if (level != null && level <= state.threshold + 10) return COLORS.amber;
  return "var(--ring)";
}

function ring(level, charging, online) {
  const cls = ["ring", charging && online ? "charging" : "", online ? "" : "asleep"].join(" ");
  return `
    <div class="${cls}" data-level="${level ?? 0}" style="--color:${ringColor(level)}">
      <svg viewBox="0 0 36 36">
        <circle class="track" cx="18" cy="18" r="15.5" pathLength="100" />
        <circle class="arc" cx="18" cy="18" r="15.5" pathLength="100" />
      </svg>
      <div class="label">${level == null ? "–" : level + "%"}</div>
    </div>`;
}

/// Starts rings at zero and lets CSS animate them to their level.
function fill(root) {
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      $$(".ring", root).forEach((r) => r.style.setProperty("--level", r.dataset.level));
    }),
  );
}

function recolorRings() {
  $$(".ring").forEach((r) => r.style.setProperty("--color", ringColor(+r.dataset.level)));
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

if (tauri) {
  const win = tauri.window.getCurrentWindow();
  $("#win-min").onclick = () => win.minimize();
  $("#win-max").onclick = () => win.toggleMaximize();
  $("#win-close").onclick = () => win.close();
}

// ---------------------------------------------------------------- devices

function statusPill(d) {
  if (!d.online) return `<span class="pill" style="--c:#8f9bb0"><span class="led"></span>Dormido</span>`;
  if (d.charging) return `<span class="pill" style="--c:${COLORS.amber}">${icon("bolt")}Cargando</span>`;
  return `<span class="pill" style="--c:${COLORS.green}"><span class="led"></span>Conectado</span>`;
}

function deviceCard(d, i) {
  const status = statusPill(d);
  return `
    <div class="glass card device" data-i="${i}" style="animation-delay:${220 + i * 80}ms">
      ${ring(d.level, d.charging, d.online)}
      <div class="info">
        <div class="name-row">
          <input class="name" placeholder="${d.name}" spellcheck="false" />
          <button class="edit" title="Renombrar">${icon("pencil")}</button>
        </div>
        <div class="meta">${icon(d.kind)} ${KIND_LABEL[d.kind] ?? ""} · detectado como «${d.name}»</div>
        ${status}
      </div>
      <div class="vline"></div>
      <div class="toggles">
        <label>Bandeja <input type="checkbox" class="switch" checked /></label>
        <label>Avisos <input type="checkbox" class="switch" checked /></label>
      </div>
      <div class="vline"></div>
      <button class="more" title="Más opciones"><svg viewBox="0 0 24 24">${ICON.dots}</svg></button>
    </div>`;
}

// a charging demo device so the animation can be seen
const DEMO = { key: "demo", name: "Auriculares (demo)", kind: "headset", level: 18, charging: true, online: true };

/// Updates the cards in place when the same devices are shown (so names being
/// typed and toggles survive), rebuilds the list when devices come or go.
function renderDevices(real) {
  const devices = [...real, DEMO];
  const list = $("#device-list");
  const sameDevices = devices.map((d) => d.key).join() === state.devices.map((d) => d.key).join();
  state.devices = devices;
  $("#device-count").textContent = `${devices.length} dispositivos`;

  if (sameDevices) {
    devices.forEach((d, i) => {
      const card = list.children[i];
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
  $$(".edit", list).forEach((b) =>
    b.addEventListener("click", () => {
      const input = $(".name", b.closest(".device"));
      input.focus();
      input.select();
    }),
  );
  $$(".more", list).forEach((b) => b.addEventListener("click", (e) => openMenu(e, b)));
  fill($("#page-devices"));
}

async function loadDevices() {
  if (!tauri) return renderDevices([]);
  // pushed by the backend on every change
  tauri.event.listen("devices", (e) => renderDevices(e.payload));
  // the reading may already be there if the window opened late
  const now = await tauri.core.invoke("devices").catch(() => null);
  if (now) renderDevices(now);
}

// ---------------------------------------------------------------- device menu

const menu = $("#device-menu");
let menuFor = null;

function openMenu(e, button) {
  e.stopPropagation();
  if (menuFor === button) return closeMenu();
  closeMenu();
  menuFor = button;
  button.classList.add("open");
  const r = button.getBoundingClientRect();
  menu.hidden = false;
  menu.style.top = r.bottom + 8 + "px";
  menu.style.left = r.right - menu.offsetWidth + "px";
}

function closeMenu() {
  menu.hidden = true;
  menuFor?.classList.remove("open");
  menuFor = null;
}

menu.addEventListener("click", (e) => {
  const action = e.target.closest("button")?.dataset.action;
  const card = menuFor?.closest(".device");
  if (action === "rename" && card) $(".name", card).focus();
  if (action === "hide" && card) $$(".switch", card)[0].checked = false;
  if (action === "forget" && card) {
    card.style.transition = "opacity .3s, transform .3s";
    card.style.opacity = 0;
    card.style.transform = "translateX(20px)";
    setTimeout(() => card.remove(), 300);
  }
  closeMenu();
});
document.addEventListener("click", closeMenu);
window.addEventListener("resize", closeMenu);
$(".content").addEventListener("scroll", closeMenu);

// ---------------------------------------------------------------- appearance

$("#previews").innerHTML = [ring(84, false, true), ring(28, false, true), ring(12, false, true), ring(60, true, true)].join("");

function renderSwatches() {
  $("#swatches").innerHTML = RING_COLORS.map(
    (s) => `<button class="swatch ${state.ring === s.id ? "active" : ""}" style="--c:${s.c}" data-id="${s.id}">${s.label ?? ""}</button>`,
  ).join("");
  $$(".swatch").forEach((b) =>
    b.addEventListener("click", () => {
      state.ring = b.dataset.id;
      document.documentElement.style.setProperty("--ring", RING_COLORS.find((s) => s.id === state.ring).c);
      renderSwatches();
    }),
  );
}
renderSwatches();

$("#animate").addEventListener("change", (e) => document.body.classList.toggle("no-anim", !e.target.checked));

// ---------------------------------------------------------------- notifications

const slider = $("#threshold");
function onThreshold() {
  state.threshold = +slider.value;
  $("#threshold-value").textContent = slider.value + "%";
  slider.style.setProperty("--fill", ((slider.value - slider.min) / (slider.max - slider.min)) * 100 + "%");
  recolorRings();
}
slider.addEventListener("input", onThreshold);
onThreshold();

// ---------------------------------------------------------------- general

const intervals = [15, 30, 60, 120, 300];
let interval = 60;
function renderIntervals() {
  $("#intervals").innerHTML = intervals
    .map((s) => `<button class="${s === interval ? "active" : ""}" data-s="${s}">${s < 60 ? s + " s" : s / 60 + " min"}</button>`)
    .join("");
  $$("#intervals button").forEach((b) =>
    b.addEventListener("click", () => {
      interval = +b.dataset.s;
      renderIntervals();
    }),
  );
}
renderIntervals();

loadDevices();
