#!/usr/bin/env bun
// Generate app-icon binary outputs from the SVG sources.
// Source contract: crates/inputforge-app/assets/README.md.
// Idempotent: for a pinned sharp + libvips, same SVGs in produce
// byte-identical files out. Bumping sharp's major version is a regen event.

import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import sharp from "sharp";
import toIco from "to-ico";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, "..");
const assetsDir = join(repoRoot, "crates", "inputforge-app", "assets");

const sources = {
  master: join(assetsDir, "icon.svg"),
  small16: join(assetsDir, "icon-16.svg"),
  medium24: join(assetsDir, "icon-24.svg"),
};

async function renderPng(svgPath, size) {
  const svg = await readFile(svgPath);
  return sharp(svg).resize(size, size).png().toBuffer();
}

async function renderRgba(svgPath, size) {
  const svg = await readFile(svgPath);
  return sharp(svg).resize(size, size).ensureAlpha().raw().toBuffer();
}

async function main() {
  const masterAt32 = await renderPng(sources.master, 32);
  const masterAt48 = await renderPng(sources.master, 48);
  const masterAt64 = await renderPng(sources.master, 64);
  const masterAt256 = await renderPng(sources.master, 256);
  const small16Buf = await renderPng(sources.small16, 16);
  const medium24Buf = await renderPng(sources.medium24, 24);

  const pngOutputs = [
    ["icon-16.png", small16Buf],
    ["icon-24.png", medium24Buf],
    ["icon-32.png", masterAt32],
    ["icon-64.png", masterAt64],
    ["icon-256.png", masterAt256],
  ];
  for (const [name, buf] of pngOutputs) {
    await writeFile(join(assetsDir, name), buf);
  }

  // ICO entry order: 16, 32, 48, 256. to-ico packs in argument order;
  // Windows reads ICONDIRENTRY records by id, not order, so this is
  // ergonomic for inspection rather than functional.
  const icoBuf = await toIco([small16Buf, masterAt32, masterAt48, masterAt256]);
  await writeFile(join(assetsDir, "icon.ico"), icoBuf);

  // 32x32 raw RGBA for tray-icon's Icon::from_rgba. ensureAlpha() guarantees
  // the alpha plane is present even when sharp's RGB-only fast path triggers.
  const rgba = await renderRgba(sources.master, 32);
  if (rgba.length !== 32 * 32 * 4) {
    throw new Error(
      `RGBA buffer is ${rgba.length} bytes; expected ${32 * 32 * 4}`,
    );
  }

  // Dot-pixel sanity check at (16, 16). The dot is r=2 in 32x32 raster,
  // so the centre pixel sits inside the disc and reads as solid CRT
  // Phosphor Green at full alpha.
  const idx = (16 * 32 + 16) * 4;
  const got = [rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]];
  const want = [46, 224, 160, 255];
  const tolerance = 4;
  for (let c = 0; c < 4; c += 1) {
    if (Math.abs(got[c] - want[c]) > tolerance) {
      throw new Error(
        `RGBA dot pixel sanity check failed: got (${got.join(", ")}), expected ~(${want.join(", ")})`,
      );
    }
  }
  await writeFile(join(assetsDir, "icon.rgba"), rgba);

  const summary = [
    ...pngOutputs.map(([name, buf]) => [name, buf.length]),
    ["icon.ico", icoBuf.length],
    ["icon.rgba", rgba.length],
  ];
  console.log("Generated icon assets:");
  for (const [name, size] of summary) {
    console.log(`  ${name}  ${size} bytes`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
