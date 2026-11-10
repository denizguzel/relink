// Builds the README's split hero: the same screen in the dark and light
// themes, joined by a soft S-curve. The light render is clipped to the left
// side and laid over the dark one, so both themes show in one image.
//
//   cd scripts/screenshots && npm install
//   node split.mjs <dark.png> <light.png> <out.png>
//
// The inputs are the headless renders from `make snaps` (`stuck-en.png` and
// `stuck-light.png`), so the hero always matches the current build.
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync } from "node:fs";

const [darkPath, lightPath, outPath] = process.argv.slice(2);
if (!darkPath || !lightPath || !outPath) {
  console.error("usage: node split.mjs <dark.png> <light.png> <out.png>");
  process.exit(1);
}

/** Width and height from the PNG's IHDR chunk. */
function size(png) {
  const signature = "89504e470d0a1a0a";
  if (png.subarray(0, 8).toString("hex") !== signature) throw new Error("not a PNG");
  return { width: png.readUInt32BE(16), height: png.readUInt32BE(20) };
}

const dark = readFileSync(darkPath);
const light = readFileSync(lightPath);
const { width: w, height: h } = size(dark);
const other = size(light);
if (other.width !== w || other.height !== h) {
  throw new Error(`the renders differ: ${w}x${h} vs ${other.width}x${other.height}`);
}

// The seam enters near the top at 60% and leaves near the bottom at 53%,
// bulging to ~64% in the middle: the same shape as a printed split.
const top = w * 0.6;
const bottom = w * 0.53;
const bulge = w * 0.66;
const left = [
  `M0 0`,
  `H${top.toFixed(1)}`,
  `C${bulge.toFixed(1)} ${(h * 0.32).toFixed(1)}`,
  `${bulge.toFixed(1)} ${(h * 0.68).toFixed(1)}`,
  `${bottom.toFixed(1)} ${h}`,
  `H0`,
  `Z`,
].join(" ");

const uri = (png) => `data:image/png;base64,${png.toString("base64")}`;
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
  <defs><clipPath id="seam"><path d="${left}"/></clipPath></defs>
  <image href="${uri(dark)}" width="${w}" height="${h}"/>
  <image href="${uri(light)}" width="${w}" height="${h}" clip-path="url(#seam)"/>
</svg>`;

const png = new Resvg(svg, { fitTo: { mode: "width", value: w } }).render().asPng();
writeFileSync(outPath, png);
console.log(`${outPath} (${w}x${h})`);
