// Renders every brand asset in assets/brand from the description below:
//   cd scripts/brand && npm install && npm run build
// The mark is a Lucide glyph drawn with a gradient stroke (width 1.75) on a
// quiet bordered tile; the wordmark is Inter semibold with tight tracking.
import { Resvg } from "@resvg/resvg-js";
import { mkdirSync, readdirSync, writeFileSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "assets", "brand");
mkdirSync(OUT, { recursive: true });

// --- the brand -------------------------------------------------------------
// Lucide "link" (ISC), 24x24 grid.
const LINK =
  '<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>' +
  '<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>';
const GRADIENT = [
  ["0", "#4F46E5"], // indigo
  ["0.55", "#2F6FD6"], // blue (the app's accent)
  ["1", "#22B8CF"], // cyan
];
const INK = "#0F1115"; // the dark surface
const TILE_BORDER = "#2A2E37";
// Status colours, as in src/brand.rs.
const STATUS = {
  ok: "#2E9E5B",
  stuck: "#E09B1A",
  missing: "#D13A3A",
  "vpn-down": "#7A8088",
  settling: "#2F6FD6",
  idle: "#2F6FD6",
};

const gradient = (id) =>
  `<linearGradient id="${id}" x1="4" y1="3" x2="20" y2="19" gradientUnits="userSpaceOnUse">` +
  GRADIENT.map(([o, c]) => `<stop offset="${o}" stop-color="${c}"/>`).join("") +
  `</linearGradient>`;

/** A Lucide glyph centred in a 256 box, `size` px tall, stroked with `stroke`. */
const glyph = (size, stroke, paint, body = LINK) => {
  const k = size / 24;
  return `<g transform="translate(${128 - 12 * k} ${128 - 12 * k}) scale(${k})" fill="none" stroke="${paint}" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${body}</g>`;
};

/** The app icon: dark bordered tile, gradient link. Thicker lines when tiny. */
const appIcon = ({ small = false } = {}) => `
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
    <defs>${gradient("g")}</defs>
    <rect x="8" y="8" width="240" height="240" rx="56" fill="${INK}" stroke="${TILE_BORDER}" stroke-width="${small ? 0 : 3}"/>
    ${glyph(small ? 164 : 150, small ? 3.1 : 2.1, "url(#g)")}
  </svg>`;

/** The bare glyph for favicons and docs: gradient stroke on transparent. */
const bareGlyph = () => `
  <svg xmlns="http://www.w3.org/2000/svg" width="192" height="192" viewBox="0 0 24 24" fill="none">
    <defs>${gradient("g")}</defs>
    <g stroke="url(#g)" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">${LINK}</g>
  </svg>`;

/** A status tile for the tray: status colour, white glyph. The problem states
 * use their own Lucide glyph, so the icon differs by shape, not only colour. */
const TRAY_GLYPHS = {
  link: LINK,
  alert:
    '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/>' +
    '<path d="M12 9v4"/><path d="M12 17h.01"/>',
  "link-off":
    '<path d="M9 17H7A5 5 0 0 1 7 7"/><path d="M15 7h2a5 5 0 0 1 4 8"/>' +
    '<line x1="8" x2="12" y1="12" y2="12"/><line x1="2" x2="22" y1="2" y2="22"/>',
  unplug:
    '<path d="m19 5 3-3"/><path d="m2 22 3-3"/>' +
    '<path d="M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z"/>' +
    '<path d="M7.5 13.5 10 11"/><path d="M10.5 16.5 13 14"/>' +
    '<path d="m12 6 6 6 2.3-2.3a2.4 2.4 0 0 0 0-3.4l-2.6-2.6a2.4 2.4 0 0 0-3.4 0Z"/>',
};
const TRAY_SHAPES = {
  ok: "link",
  stuck: "alert",
  missing: "link-off",
  "vpn-down": "unplug",
  settling: "link",
  idle: "link",
};
const trayIcon = (color, shape) => `
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
    <rect x="6" y="6" width="244" height="244" rx="60" fill="${color}"/>
    ${glyph(176, 3.3, "#FFFFFF", TRAY_GLYPHS[shape])}
  </svg>`;

// --- rendering helpers -------------------------------------------------------
const fonts = () => {
  const root = join(homedir(), ".cargo", "git", "checkouts");
  for (const d of existsSync(root) ? readdirSync(root) : []) {
    if (!d.startsWith("rune-")) continue;
    for (const rev of readdirSync(join(root, d))) {
      const dir = join(root, d, rev, "crates", "rune", "assets");
      if (existsSync(join(dir, "Inter-SemiBold.ttf")))
        return ["Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-SemiBold.ttf"].map((f) => join(dir, f));
    }
  }
  throw new Error("Inter fonts not found (build the app once so cargo fetches Rune)");
};
const FONT_FILES = fonts();
const render = (svg, width) =>
  new Resvg(svg, {
    fitTo: { mode: "width", value: width },
    font: { fontFiles: FONT_FILES, loadSystemFonts: false, defaultFontFamily: "Inter" },
  }).render();
const png = (svg, w) => Buffer.from(render(svg, w).asPng());

/** Straight-alpha RGBA bytes (resvg gives premultiplied). */
const rgba = (svg, w) => {
  const p = render(svg, w).pixels;
  const out = Buffer.alloc(p.length);
  for (let i = 0; i < p.length; i += 4) {
    const a = p[i + 3];
    for (let k = 0; k < 3; k++) out[i + k] = a ? Math.min(255, Math.round((p[i + k] * 255) / a)) : 0;
    out[i + 3] = a;
  }
  return out;
};

const ico = (frames) => {
  const head = Buffer.alloc(6 + 16 * frames.length);
  head.writeUInt16LE(1, 2);
  head.writeUInt16LE(frames.length, 4);
  let offset = head.length;
  frames.forEach(({ size, data }, i) => {
    const e = 6 + 16 * i;
    head[e] = size >= 256 ? 0 : size;
    head[e + 1] = size >= 256 ? 0 : size;
    head.writeUInt16LE(1, e + 4);
    head.writeUInt16LE(32, e + 6);
    head.writeUInt32LE(data.length, e + 8);
    head.writeUInt32LE(offset, e + 12);
    offset += data.length;
  });
  return Buffer.concat([head, ...frames.map((f) => f.data)]);
};

/** A 24-bit BMP of an opaque render (the installer wizard wants BMPs). */
const bmp = (svg, w, h) => {
  const px = render(svg, w).pixels;
  const row = Math.ceil((w * 3) / 4) * 4;
  const out = Buffer.alloc(54 + row * h);
  out.write("BM");
  out.writeUInt32LE(out.length, 2);
  out.writeUInt32LE(54, 10);
  out.writeUInt32LE(40, 14);
  out.writeInt32LE(w, 18);
  out.writeInt32LE(h, 22);
  out.writeUInt16LE(1, 26);
  out.writeUInt16LE(24, 28);
  out.writeUInt32LE(row * h, 34);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      const o = 54 + (h - 1 - y) * row + x * 3;
      out[o] = px[i + 2];
      out[o + 1] = px[i + 1];
      out[o + 2] = px[i];
    }
  return out;
};

const write = (name, data) => {
  writeFileSync(join(OUT, name), data);
  console.log(name, data.length);
};

// --- outputs -----------------------------------------------------------------
write("glyph.svg", bareGlyph());
write("icon.svg", appIcon());
write("icon-256.png", png(appIcon(), 256));
write("icon-512.png", png(appIcon(), 512));
write(
  "relink.ico",
  ico([16, 24, 32, 48, 64, 128, 256].map((size) => ({ size, data: png(appIcon({ small: size <= 48 }), size) }))),
);
for (const [name, color] of Object.entries(STATUS))
  write(`tray-${name}.rgba`, rgba(trayIcon(color, TRAY_SHAPES[name]), 32));

// README header and GitHub social preview (1280x640).
const wordmark = (x, y, size, tag) => `
  <text x="${x}" y="${y}" font-family="Inter" font-weight="600" font-size="${size}" letter-spacing="${-0.02 * size}" fill="#F4F5F7">Re-Link</text>
  ${tag ? `<text x="${x}" y="${y + size * 0.62}" font-family="Inter" font-weight="400" font-size="${size * 0.3}" fill="#9AA1AC">${tag}</text>` : ""}`;
const banner = (w, h) => `
  <svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
    <rect width="${w}" height="${h}" fill="${INK}"/>
    <svg x="${w / 2 - h * 0.55}" y="${h * 0.2}" width="${h * 0.36}" height="${h * 0.36}" viewBox="0 0 256 256">${appIcon().replace(/<\/?svg[^>]*>/g, "")}</svg>
    ${wordmark(w / 2 - h * 0.15, h * 0.46, h * 0.19, "")}
    <text x="${w / 2}" y="${h * 0.7}" text-anchor="middle" font-family="Inter" font-weight="400" font-size="${h * 0.052}" fill="#9AA1AC">Keeps your browsers connected while the VPN split tunnel changes.</text>
  </svg>`;
write("social-preview.png", png(banner(1280, 640), 1280));

// Installer wizard images (Inno Setup wants BMPs).
const wizard = (w, h) => `
  <svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
    <rect width="${w}" height="${h}" fill="${INK}"/>
    <svg x="${w / 2 - 48}" y="${h * 0.22}" width="96" height="96" viewBox="0 0 256 256">${appIcon().replace(/<\/?svg[^>]*>/g, "")}</svg>
    <text x="${w / 2}" y="${h * 0.22 + 138}" text-anchor="middle" font-family="Inter" font-weight="600" font-size="24" letter-spacing="-0.5" fill="#F4F5F7">Re-Link</text>
  </svg>`;
write("installer-wizard.bmp", bmp(wizard(164, 314), 164, 314));
write("installer-small.bmp", bmp(appIcon({ small: true }), 55, 55));
