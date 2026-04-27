# Product

## Register

product

## Users

Sim, flight-sim, and racing-sim enthusiasts running multi-device rigs on Windows 10+. The user is a power-user with a HOTAS / pedals / wheel / button-box setup who is already familiar with category tools (JoystickGremlin, vJoy, ReWASD) and brings strong opinions about deadzones, response curves, mode profiles, and how those should compose. They open the GUI to **author** a profile for a specific game or **tune** one between rounds — not to browse, learn, or be onboarded.

Two session shapes appear in roughly equal weight:

- **Authoring sessions.** The engine is closed or paused. The user is building or revising mappings against a target game's control scheme. Goal: assemble the right pipeline (curve · deadzone · output) for each input that matters.
- **Tuning sessions.** The engine is running, the game is in another window, the user has the GUI open to adjust deadzones / curves / sensitivity and watch the response live. Goal: see what the engine sees, change one knob, feel the difference, save.

The GUI is closed during normal play (the engine keeps running in the tray). Performance during an open GUI is therefore not the constraint; **clarity and editing speed** are.

## Product Purpose

InputForge takes physical input devices, runs each input through an action pipeline (calibration, curves, deadzones, conditionals, mode-scoping), and emits to vJoy virtual devices that games consume. The GUI is the configuration and testing surface for that pipeline.

It exists because every alternative makes a trade the user refuses:

- JoystickGremlin solves the routing problem but its UI is uncared-for.
- vJoyConf does plumbing only.
- ReWASD aims at consumer remapping, not multi-device sim rigs.

InputForge's stake in the ground: **a precision sim-input tool with the editing ergonomics power users expect from modern technical software** — Linear, Stripe Dashboard, Figma in the lane it occupies. Success is the user reaching for InputForge first, on the same rig where they used to keep JoystickGremlin running.

## Brand Personality

**Sharp · Calm · Technical.**

The interface is a precision instrument, not a brand experience. It carries itself with the disciplined restraint of Linear or Stripe Dashboard: hierarchy and contrast do the heavy lifting, color and motion are used like seasoning. Voice is technical — accurate names, exact values, no marketing softeners ("Cool!", "Awesome!", "We've got you covered"). Confidence shows up as defaults that don't need explanation and forms that don't apologize for asking for numbers.

Emotional target: **focus**. The user should feel the same way using InputForge as they feel using a well-tuned DAW or color grading tool — that the software is getting out of the way of the work, that the values on screen are trustworthy, that small adjustments produce predictable results.

## Anti-references

Four explicit anti-references. Every visual decision should be testable against this list — if the answer drifts toward any of these, the answer is wrong.

- **JoystickGremlin / 2003-era Tk.** Cramped grey forms, system-default fonts, dialog soup, modal-after-modal flows. The whole reason for the rewrite. The cure is not just better visuals — it is editing surfaces that don't make the user click through nested windows to change one number.
- **Generic SaaS dashboard.** Big number cards, soft pastel shadows, friendly empty-state illustrations, marketing-style stat tiles, "Welcome back!" greetings. InputForge is a workshop, not a metrics product. There are no KPIs to celebrate.
- **Gaming RGB / neon-on-black.** Razer Synapse, Logitech G Hub, Corsair iCUE. Backlit edges, cyan glows, gradient angles, "esports" type. Misreads InputForge as consumer kit. The user is configuring sim hardware, not an LED strip.
- **Apple glassmorphism / blurred translucency.** Frosted panels, system-tinted blurs, oversoft hairlines, "vibrancy." Wrong register for a precision tool — the imprecision is the whole problem.

These four are not just "things to avoid." They are the four directions a careless redesign of this tool would slide in. The Sharp · Calm · Technical personality is defined by what it refuses to be.

## Design Principles

Five strategic principles. These guide structural and behavioral decisions; visual rules belong in DESIGN.md.

1. **Live data is the contract.** What the engine sees, the user sees — without a save-and-check round trip. Curves, deadzones, and calibration show their effect on the live signal as the user tweaks. If a tuning session ever forces "edit, save, switch to game, feel it, switch back, adjust," the surface has failed.
2. **The hardware is the protagonist.** The UI exists to make hardware behavior legible. Real device names, real input values, real output verification. Never abstract a stick or pedal into a generic control. Identifiers and addresses are first-class citizens, not implementation details hidden in tooltips.
3. **Power-user defaults, no apologies.** Density over whitespace. Numeric inputs over sliders when precision matters. Keyboard speed over click-to-discover. Onboarding is minimal — assume the user knows what a deadzone is. Users who don't are not the target; pretending otherwise costs the target audience.
4. **One job per surface.** Each visible region has a single, sharp purpose. The left rail is for navigation. The center is for editing the selected thing. Status chrome reports state, never tries to be a tool. If two regions compete for the same job, one of them is wrong; cut it.
5. **Restraint over spectacle.** Hierarchy, contrast, and rhythm carry the design — not color, not shadows, not motion. Color appears where it makes a value mechanically more legible (live indicators, dirty state, error). Motion appears where it confirms causality (a state change the user requested). Neither appears as decoration.

## Accessibility & Inclusion

Target: **WCAG 2.2 AA** for color contrast on text and meaningful UI. Defaults are dark theme; if a light theme is added later, both must clear AA at the same components.

Respect `prefers-reduced-motion` system-wide: motion under it collapses to instantaneous state changes, never decorative animation, never parallax.

Color is never the sole channel for state. Live / paused / stopped engine status carries a shape (filled vs. ring dot) and a label, not just a hue. Mapped vs. unmapped inputs carry an icon or a position, not only a color.

Keyboard-only navigation must be possible end-to-end: every action surface (mapping editor, calibration, profile management, mode switching) reachable without a mouse, with visible focus rings that survive on dark backgrounds. A keyboard-coverage audit milestone in the Dioxus rewrite enforces this; the deliverable is "every action surface reachable, every focus ring visible on the dark theme".

i18n / localization is out of scope for now and not implied by this document. Strings are English-only; this is acceptable given the audience.
