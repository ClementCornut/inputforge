# App Icon Design Spec

## Context

The project ships a real GUI and a system tray icon today, but no branded icon
asset is managed by the codebase. The taskbar, alt-tab, and Start menu show
dx-cli's auto-generated rocket placeholder (left at
`target/dx/inputforge-app/{debug,release}/windows/.winres/icon.ico` after each
build). The tray icon is loaded from
`crates/inputforge-app/assets/icon.rgba` by `tray.rs::load_icon` (defined at
`crates/inputforge-app/src/tray.rs:94`); the `Icon::from_rgba` call sits at
`crates/inputforge-app/src/tray.rs:96`, and that file is a 4096-byte (32x32
RGBA) blob of unknown provenance.

`PRODUCT.md` and `DESIGN.md` define a coherent visual identity (the "Evolved
Glass Cockpit" north star, Hangar Navy `#14172A`, HUD Cyan-Blue `#4FA8FF`,
CRT Phosphor Green `#2EE0A0`). The app icon should sit inside that system,
not adjacent to it.

This spec defines the source SVG, the derived binary outputs, the conversion
script, and the integration paths into both the development build (cargo /
`dx run`) and the future bundled build (`dx bundle`).

## Goals

- Replace the dx-cli rocket placeholder with a branded mark that survives at
  every size Windows actually uses (16, 24, 32, 48, 256).
- Replace the existing `icon.rgba` so the tray carries the same identity.
- Keep one SVG source per visually distinct rendering: a 96 px master, a
  16 px variant for the small-icon taskbar, and a 24 px variant for the
  150% DPI taskbar; each tunes stroke and dot weight for its raster grid.
- Generate all binary derivatives (.ico, .rgba, per-size .pngs) from those
  SVGs through a single committed script, so any future palette tweak in
  `DESIGN.md` propagates by re-running the script.
- Wire the icon into the .exe at compile time (covers `cargo build`, `dx run`,
  `dx serve`) and declare it for `dx bundle` (covers future installer
  creation), since Dioxus separates dev-time from bundle-time.

## Non-Goals

- No macOS `.icns` and no Linux PNG set. `PRODUCT.md` scopes the project to
  Windows 10+; cross-platform icon assets stay out of scope.
- No favicons, marketing artwork, or social-card assets.
- No changes to the in-app icon set under
  `crates/inputforge-gui-dx/src/icons/svg/`. Those follow Phosphor and are
  unrelated to the app's executable identity.
- No code changes to `crates/inputforge-app/src/tray.rs`. Only the byte
  content of `icon.rgba` changes.
- No build-time SVG-to-PNG pipeline. The conversion runs out of band, and
  the binaries are checked in.
- No light-theme variant of the mark. The navy tile is self-contained and
  reads against any background.

## The Mark

### Master SVG (`crates/inputforge-app/assets/icon.svg`)

```svg
<svg viewBox="0 0 96 96" xmlns="http://www.w3.org/2000/svg">
  <rect width="96" height="96" rx="14" fill="#14172A"/>
  <path d="M 14 80 C 30 80, 36 74, 48 48 C 60 22, 66 16, 82 16"
        fill="none" stroke="#4FA8FF" stroke-width="9" stroke-linecap="round"/>
  <circle cx="48" cy="48" r="6" fill="#2EE0A0"/>
</svg>
```

The mark is an S-curve, the signature shape produced by InputForge's curve
editor, with a phosphor-green dot at the inflection point. Three semantic
choices anchor it to the design system:

- The tile color is the `bg` token (Hangar Navy). It does not change with
  theme; the icon carries its own surface.
- The curve color is the `primary` token (HUD Cyan-Blue), the project's One
  Action Color per `DESIGN.md` section 2. The mark contains the only
  permitted action color and nothing else.
- The dot color is the `live` token (CRT Phosphor Green), the project's
  reserved live-signal color. The dot at the inflection reads as "the live
  cursor sitting on the response trace," tying the mark to the
  product's "live data is the contract" principle from `PRODUCT.md`.

This master renders cleanly at 32, 48, and 256 pixels. It does not render
cleanly at 16 pixels because the 9 px stroke and 6 px dot scale down to
sub-pixel widths.

### 16 px variant (`crates/inputforge-app/assets/icon-16.svg`)

```svg
<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect width="16" height="16" rx="2" fill="#14172A"/>
  <path d="M 2 13.5 C 5 13.5, 6 12, 8 8 C 10 4, 11 2.5, 14 2.5"
        fill="none" stroke="#4FA8FF" stroke-width="2.5" stroke-linecap="round"/>
  <circle cx="8" cy="8" r="1.6" fill="#2EE0A0"/>
</svg>
```

Hand-tuned for the 16x16 raster: corner radius dropped to 2, stroke widened
proportionally to 2.5, dot held at r=1.6 so it antialiases to a recognisable
~2 px round shape rather than a single pixel. Used only at 16 px.

### 24 px variant (`crates/inputforge-app/assets/icon-24.svg`)

```svg
<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <rect width="24" height="24" rx="3" fill="#14172A"/>
  <path d="M 3 20 C 7.5 20, 9 18, 12 12 C 15 6, 16.5 4, 21 4"
        fill="none" stroke="#4FA8FF" stroke-width="3.5" stroke-linecap="round"/>
  <circle cx="12" cy="12" r="2.2" fill="#2EE0A0"/>
</svg>
```

Hand-tuned for the 24x24 raster: corner radius 3 (between the 16 px
variant's 2 and the 96 px master's 14, scaled by side length); stroke 3.5
(14.6% of side, matching the 16 px variant's 15.6% so the curve still reads
at small scale, where the master's 9.4% would feel anaemic); dot r=2.2
antialiases to roughly a 4 px round shape, giving the live-cursor
inflection point real visual weight. Path coordinates are the 16 px variant
scaled 1.5x and rounded for the 24 px raster grid. Used only at 24 px,
which is the size Windows taskbar uses at 150% DPI scaling.

### Brand-rule check

The mark passes every rule in `DESIGN.md` section 8:

- No side-stripe borders, no gradient text, no `backdrop-filter`, no soft
  drop shadow on a card-style surface.
- Single solid colors, all drawn from the documented token system. No new
  hues introduced.
- The tile is the navy surface, not a `#000` rectangle. The dot and curve
  are the canonical token values.
- No glow or RGB lighting. No "esports" type. No skeumorphic chamfer.

## File Layout

All assets live under `crates/inputforge-app/assets/` so the convention "all
icon assets live next to `tray.rs::include_bytes!`'s target" is preserved:

```
crates/inputforge-app/assets/
  icon.svg          source     master mark, viewBox 0 0 96 96
  icon-16.svg       source     16 px variant, viewBox 0 0 16 16
  icon-24.svg       source     24 px variant, viewBox 0 0 24 24
  icon-16.png       generated  for [bundle].icon
  icon-24.png       generated  for [bundle].icon
  icon-32.png       generated  for [bundle].icon
  icon-64.png       generated  for [bundle].icon
  icon-256.png      generated  for [bundle].icon
  icon.ico          generated  16 (from icon-16.svg) + 32, 48, 256 (from icon.svg)
  icon.rgba         generated  32x32 raw RGBA for the tray (replaces existing file)
  README.md         doc        when to regenerate; one-line invocation; input/output table
```

PNGs are emitted at 16, 24, 32, 64, and 256 (not 48) because those are the
sizes the Dioxus bundle config accepts. The `.ico` carries 16, 32, 48, and
256 because Windows is liberal at that layer and 48 is the size the Start
menu picks up at standard DPI; standard `.ico` files do not carry a 24
frame.

The three `.svg` files and the README are the human-edited surface. The
five `.png` files, the `.ico`, and the `.rgba` are regenerated by the
script and checked in alongside. The committed binary outputs are the
contract; a fresh clone builds without bun installed. Bun is only required
when the SVGs change.

## Generation Script

### Location and invocation

`scripts/build-app-icon.mjs` at the workspace root. Invoked as:

```
bun scripts/build-app-icon.mjs
```

Bun is the project's standard JavaScript runner per existing convention
(`.mcp.json` entries use `bunx`). The script is idempotent: same SVGs in,
identical bytes out, regardless of run order or working directory.

### Dependencies

The script depends on two npm packages, declared in a workspace-root
`package.json` (the project does not have one yet; this spec adds it):

- name: `inputforge-icon-pipeline`
- version: `0.0.0`
- private: `true`
- dependencies:
  - `sharp` (^0.33) for SVG-to-PNG rasterisation. Sharp also exposes raw
    RGBA output, which the script uses directly for `icon.rgba`.
  - `to-ico` (^1.1) to pack a list of PNG buffers into a Windows
    multi-resolution `.ico` file.

Both are mature, single-purpose libraries with no native build steps
required on Windows.

Commit `bun.lockb` for reproducible installs. Add `node_modules/` to the
workspace `.gitignore`. No `scripts` block in `package.json`; the pipeline
is invoked directly as `bun scripts/build-app-icon.mjs`.

### Pipeline

1. Resolve repo root from `import.meta.url`.
2. Define the input map:
   - `icon.svg`     -> render at 32, 48, 64, 256 (PNG)
   - `icon-16.svg`  -> render at 16 (PNG)
   - `icon-24.svg`  -> render at 24 (PNG)
3. For each entry, render the SVG to a PNG buffer at the target size using
   `sharp(svgBuffer).resize(size, size).png()`.
4. Write the five bundle-PNG files (`icon-16.png`, `icon-24.png`,
   `icon-32.png`, `icon-64.png`, `icon-256.png`) to
   `crates/inputforge-app/assets/`. The 48 px buffer rendered in step 2 is
   consumed only by `to-ico` in step 5 and is not written as a standalone
   PNG; bundle PNGs are 16/24/32/64/256.
5. Build `icon.ico` by passing the 16, 32, 48, 256 PNG buffers to `to-ico`
   and writing the result.
6. Build `icon.rgba` by re-rendering `icon.svg` at 32x32 with
   `sharp(svgBuffer).resize(32, 32).ensureAlpha().raw().toBuffer()`. Sharp's
   `.raw()` returns row-major 8-bit-per-channel RGBA;
   `.ensureAlpha()` guarantees the alpha channel is present. The existing
   4096-byte `icon.rgba` already feeds `tray_icon::Icon::from_rgba`
   successfully, so the format produced here matches by construction.
   Build-time verification: read the dot pixel at (16, 16) of the new
   `icon.rgba` and confirm it reads as `(46, 224, 160, 255)` (CRT Phosphor
   Green at full alpha), not a premultiplied variant.
7. Print a summary of what was written and the byte sizes.

### Determinism

The script seeds nothing random. `sharp` produces deterministic PNG output
given identical SVG input and resize parameters. `to-ico` packs entries in
the order they are given. Re-running the script with no SVG changes
overwrites the binary outputs with byte-identical content, so a `git diff`
on a clean tree after re-running shows nothing.

### README

`crates/inputforge-app/assets/README.md` documents:

- The three SVGs are the source of truth.
- The seven binary outputs (five PNGs, one ICO, one RGBA) are generated by
  `bun scripts/build-app-icon.mjs` from the workspace root.
- An input/output table mirroring the file-layout block above.
- The one rule: do not edit the generated files by hand; edit the SVGs and
  re-run the script.

## Embedding into the .exe (winres)

The `winres` crate embeds Windows resources (icons, version info, manifest)
into the linked binary at compile time by emitting linker flags from
`build.rs`. The `inputforge-app` crate already has a Windows-aware `build.rs`:
`fn main()` opens at `crates/inputforge-app/build.rs:17` and early-returns on
non-Windows hosts at lines 20-22 via
`if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }`.
The icon embedding lives in the same function body, after that gate, so no
extra `#[cfg(target_os = "windows")]` wrapper is needed on the new code.

### Cargo.toml changes

The project uses workspace-level version pinning. `winres` follows the
same pattern.

Add to the workspace `Cargo.toml` `[workspace.dependencies]` block (next to
the other Windows-only entries like `windows` and `tray-icon`):

```toml
winres = "0.1"
```

Add to `crates/inputforge-app/Cargo.toml`, creating the section (the crate
does not yet have a `[build-dependencies]` section):

```toml
[target.'cfg(target_os = "windows")'.build-dependencies]
winres = { workspace = true }
```

The `cfg(target_os = "windows")` target predicate keeps `winres` out of the
dependency graph on non-Windows hosts, where it is unused.

### build.rs changes

Add the following block in `crates/inputforge-app/build.rs` after the
non-Windows early-return (after line 22) and before the SDL3.dll copy block:

```rust
let mut res = winres::WindowsResource::new();
res.set_icon("assets/icon.ico");
if let Err(e) = res.compile() {
    println!("cargo:warning=winres failed to embed icon.ico: {e}");
}
println!("cargo:rerun-if-changed=assets/icon.ico");
```

Reasons for the soft failure (warning rather than panic):

- The existing `build.rs` uses the same `cargo:warning=...` convention for
  SDL3.dll absence (lines 47, 65, 81, 116, 129).
- A failed icon embed should not block the build; the binary is still usable,
  just without a custom icon.
- If `assets/icon.ico` is somehow missing, the contract is "the script was
  not run"; the warning surfaces that loudly enough to fix.

The trailing `cargo:rerun-if-changed=assets/icon.ico` line in the block above
ensures cargo re-runs `build.rs` (and re-embeds) when the icon changes.
Build.rs does NOT track the source SVGs: the SVG-to-ICO step is the bun
script, not cargo, so cargo's only icon-related trigger is the .ico file.

### Effect

The embedded icon resource is what Windows reads for taskbar, alt-tab,
Explorer thumbnails, the executable's file icon, and shortcuts to it. dx-cli's
auto-generated `target/dx/.../.winres/icon.ico` becomes irrelevant: the
embedded resource takes precedence at every surface that consults the
binary's icon.

## Bundle Config (Dioxus.toml)

The project does not have a `Dioxus.toml`. This spec creates one at the
workspace root with the bundle config below. It is a no-op until someone
runs `dx bundle`; on that day, the icon is already wired up.

```toml
[application]
name = "InputForge"

[bundle]
identifier = "io.inputforge.app"
icon = [
  "crates/inputforge-app/assets/icon-16.png",
  "crates/inputforge-app/assets/icon-24.png",
  "crates/inputforge-app/assets/icon-32.png",
  "crates/inputforge-app/assets/icon-64.png",
  "crates/inputforge-app/assets/icon-256.png",
  "crates/inputforge-app/assets/icon.ico",
]
```

`identifier` follows reverse-DNS convention; `io.inputforge.app` is a
plausible default that does not collide with any registered domain. The
identifier can be changed later without affecting the icon. Treat it as a
placeholder until a domain or store identity is claimed; changing it later
is mechanical but breaks any installer update channel that has already
shipped.

The Dioxus bundle docs at
https://dioxuslabs.com/learn/0.7/guides/tools/configure/#bundle accept five
PNG sizes (16, 24, 32, 64, 256). We ship all five; 24 px targets the Windows
taskbar at 150% DPI scaling and gets its own hand-tuned source so it does
not degrade to a 16-to-24 upscale. Including the `.ico` alongside the PNGs
lets tauri-bundler pick the right asset for each Windows artifact it
produces.

`[bundle.windows].icon_path` is intentionally omitted: it controls
tauri-bundler's idea of an installer-side tray icon, not our runtime tray.
Our tray is owned by the `tray-icon` crate via `icon.rgba` and is
unaffected by the bundle config.

## Tray Integration

`crates/inputforge-app/src/tray.rs:96` already calls `Icon::from_rgba` on
the bytes pulled in by `include_bytes!("../assets/icon.rgba")` at line 95.
The byte format expected by `Icon::from_rgba` is row-major 8-bit RGBA at
32x32, which is exactly what the script produces.

No code change is needed in `tray.rs`. The tray begins displaying the new
mark on the next build after the script overwrites `icon.rgba`.

## Verification

The implementation plan that follows this spec ends with these checks. They
are recorded here so the plan does not need to invent them:

0. Before adding any winres plumbing, build the current binary, run the bun
   script once to produce `assets/icon.ico`, then re-link `inputforge-app`
   with a stand-in winres step and confirm the embedded PE icon survives a
   `dx run` rebuild. PowerShell check:

   ```
   Add-Type -AssemblyName System.Drawing
   $exe = "target/debug/inputforge-app.exe"
   [System.Drawing.Icon]::ExtractAssociatedIcon($exe).ToBitmap().Size
   ```

   If dx-cli's auto-embedded rocket overwrites the resource, abandon the
   winres path and instead wire `assets/icon.ico` through `[application].icon`
   in `Dioxus.toml`. The rest of this spec (asset layout, generation script,
   tray integration, bundle config) is unchanged in either path.

1. `bun scripts/build-app-icon.mjs` from a clean tree produces seven files
   under `crates/inputforge-app/assets/` (five PNGs, one ICO, one RGBA) with
   non-zero sizes; re-running it leaves `git status` clean.
2. `cargo build -p inputforge-app` succeeds without new warnings beyond the
   existing SDL3.dll-absence pattern. Inspecting
   `target/debug/inputforge-app.exe` in Explorer's Properties dialog shows
   the new mark in the icon panel.
3. `dx run -p inputforge-app` shows the new mark in the taskbar and in
   alt-tab. The system tray icon shows the new mark.
4. The `.ico` carries all four sizes. Confirm with PowerShell (the project
   is Windows-only):

   ```
   $bytes = [IO.File]::ReadAllBytes("crates/inputforge-app/assets/icon.ico")
   [BitConverter]::ToUInt16($bytes, 4)  # ICONDIR.idCount; expect 4
   for ($i = 0; $i -lt 4; $i++) {
       $entry = 6 + $i * 16
       $w = if ($bytes[$entry]   -eq 0) { 256 } else { $bytes[$entry] }
       $h = if ($bytes[$entry+1] -eq 0) { 256 } else { $bytes[$entry+1] }
       "$w x $h"
   }
   ```

   Output expected: `4` followed by `16 x 16`, `32 x 32`, `48 x 48`,
   `256 x 256` in some order.
5. The 16 px taskbar rendering carries the dot legibly (per the brainstorm
   selection that retained dot r=1.6 at the small size), and the 24 px PNG
   (visible if the user runs Windows at 150% DPI scaling) carries the dot
   at roughly 4 px diameter, not blurred to a smear.

Failure of any check is a defect in the implementation plan, not the design.
