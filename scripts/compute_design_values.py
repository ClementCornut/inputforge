"""Compute OKLCH from hex, WCAG contrast ratios, and find new text-subtle.

Self-contained: no third-party deps. Outputs a JSON blob to stdout
that the design-doc edits consume.
"""

import json
import math


# --- sRGB <-> OKLCH ---------------------------------------------------------

def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) / 255.0 for i in (0, 2, 4))


def rgb_to_hex(rgb):
    r, g, b = (max(0, min(255, round(c * 255))) for c in rgb)
    return f"#{r:02X}{g:02X}{b:02X}"


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def linear_to_srgb(c):
    return 12.92 * c if c <= 0.0031308 else 1.055 * (c ** (1 / 2.4)) - 0.055


def linear_rgb_to_oklab(r, g, b):
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l_ = l ** (1 / 3)
    m_ = m ** (1 / 3)
    s_ = s ** (1 / 3)
    return (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )


def oklab_to_linear_rgb(L, a, b):
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l = l_ ** 3
    m = m_ ** 3
    s = s_ ** 3
    return (
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )


def hex_to_oklch(h):
    r, g, b = (srgb_to_linear(c) for c in hex_to_rgb(h))
    L, a, b_ = linear_rgb_to_oklab(r, g, b)
    C = math.sqrt(a * a + b_ * b_)
    H = math.degrees(math.atan2(b_, a))
    if H < 0:
        H += 360
    return L, C, H


def oklch_to_hex(L, C, H):
    a = C * math.cos(math.radians(H))
    b = C * math.sin(math.radians(H))
    r, g, b_ = oklab_to_linear_rgb(L, a, b)
    return rgb_to_hex((linear_to_srgb(max(0, min(1, r))),
                       linear_to_srgb(max(0, min(1, g))),
                       linear_to_srgb(max(0, min(1, b_)))))


# --- WCAG contrast ----------------------------------------------------------

def relative_luminance(hex_color):
    r, g, b = hex_to_rgb(hex_color)
    r, g, b = srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast_ratio(fg, bg):
    L1, L2 = relative_luminance(fg), relative_luminance(bg)
    lighter, darker = max(L1, L2), min(L1, L2)
    return (lighter + 0.05) / (darker + 0.05)


def color_mix(fg_hex, bg_hex, alpha):
    """Composite fg over bg at given alpha; returns sRGB hex.
    Mirrors CSS color-mix(in srgb, fg <alpha%>, bg <(1-alpha)%>).
    """
    fr, fg_, fb = hex_to_rgb(fg_hex)
    br, bg_, bb = hex_to_rgb(bg_hex)
    return rgb_to_hex(
        (fr * alpha + br * (1 - alpha),
         fg_ * alpha + bg_ * (1 - alpha),
         fb * alpha + bb * (1 - alpha)),
    )


# --- Inputs from DESIGN.md frontmatter --------------------------------------

HEX = {
    "bg":             "#14172A",
    "bg_elevated":    "#1E223A",
    "bg_sunken":      "#0E1020",
    "text":           "#E4E6F0",
    "text_muted":     "#9CA0BC",
    "text_subtle":    "#686C88",
    "border":         "#2A2E48",
    "border_strong":  "#424766",
    "border_focus":   "#5AB0FF",
    "primary":        "#4FA8FF",
    "primary_hover":  "#6FBAFF",
    "primary_active": "#2D7FD9",
    "live":           "#2EE0A0",
    "warning":        "#FFB347",
    "error":          "#F25555",
    "error_hover":    "#FF6F6F",
    "error_active":   "#D43F3F",
    "processing":     "#3FB8B0",
    "output":         "#C99846",
    "control":        "#9A78D6",
}


def make_ramp(L_canonical, C_canonical, H_canonical):
    """Generate an 8-stop ramp anchored at the canonical L value.
    L stops follow a fixed cadence; chroma falls off near both lightness extremes.
    The canonical step is the one closest to L_canonical in the ramp grid.
    """
    L_grid = [0.18, 0.28, 0.40, 0.52, 0.63, 0.72, 0.82, 0.92]
    canonical_idx = min(range(len(L_grid)), key=lambda i: abs(L_grid[i] - L_canonical))
    L_grid = list(L_grid)
    L_grid[canonical_idx] = round(L_canonical, 3)

    ramp = []
    for L in L_grid:
        # Falloff: chroma scales by lightness contour (peaks mid, drops near extremes).
        falloff = 1 - ((L - 0.55) / 0.55) ** 2
        falloff = max(0.25, min(1.0, falloff))
        C = round(C_canonical * falloff, 3)
        ramp.append(f"oklch({round(L, 3)} {C} {round(H_canonical, 1)})")
    return ramp


def fmt_oklch(L, C, H):
    return f"oklch({round(L, 3)} {round(C, 3)} {round(H, 1)})"


# --- Compute everything -----------------------------------------------------

result = {
    "tokens": {},
    "ramps": {},
    "contrast": {},
    "category_badges": {},
    "text_subtle_search": [],
}

for name, h in HEX.items():
    L, C, H = hex_to_oklch(h)
    result["tokens"][name] = {
        "hex": h,
        "oklch": fmt_oklch(L, C, H),
        "L": round(L, 3),
        "C": round(C, 3),
        "H": round(H, 1),
    }
    result["ramps"][name] = make_ramp(L, C, H)


# Status text contrast on bg and bg-elevated
status_pairs = [
    ("primary", "bg"),
    ("primary", "bg_elevated"),
    ("primary_active", "bg"),
    ("primary_active", "bg_elevated"),
    ("live", "bg"),
    ("live", "bg_elevated"),
    ("warning", "bg"),
    ("warning", "bg_elevated"),
    ("error", "bg"),
    ("error", "bg_elevated"),
    ("text", "bg"),
    ("text", "bg_elevated"),
    ("text_muted", "bg"),
    ("text_muted", "bg_elevated"),
    ("text_subtle", "bg"),
    ("text_subtle", "bg_elevated"),
]
for fg_name, bg_name in status_pairs:
    ratio = contrast_ratio(HEX[fg_name], HEX[bg_name])
    result["contrast"][f"{fg_name} on {bg_name}"] = round(ratio, 2)


# Category badge contrast: hue itself on 14% tint of hue mixed into bg-elevated
for cat in ("processing", "output", "control"):
    composited = color_mix(HEX[cat], HEX["bg_elevated"], 0.14)
    ratio = contrast_ratio(HEX[cat], composited)
    result["category_badges"][cat] = {
        "hue": HEX[cat],
        "composited_bg": composited,
        "ratio": round(ratio, 2),
        "passes_AA": ratio >= 4.5,
    }


# Find a new text-subtle that clears 4.5x on bg-elevated
# Walk lightness up from current #686C88, keeping chroma and hue fixed
L_subtle, C_subtle, H_subtle = hex_to_oklch(HEX["text_subtle"])
for delta in [round(0.005 * i, 3) for i in range(0, 60)]:
    candidate_hex = oklch_to_hex(L_subtle + delta, C_subtle, H_subtle)
    ratio_elev = contrast_ratio(candidate_hex, HEX["bg_elevated"])
    ratio_bg = contrast_ratio(candidate_hex, HEX["bg"])
    result["text_subtle_search"].append({
        "L_delta": delta,
        "hex": candidate_hex,
        "ratio_on_bg_elevated": round(ratio_elev, 2),
        "ratio_on_bg": round(ratio_bg, 2),
        "passes_AA_elev": ratio_elev >= 4.5,
    })
    if ratio_elev >= 4.5:
        result["text_subtle_recommendation"] = {
            "hex": candidate_hex,
            "oklch": fmt_oklch(L_subtle + delta, C_subtle, H_subtle),
            "ratio_on_bg_elevated": round(ratio_elev, 2),
            "ratio_on_bg": round(ratio_bg, 2),
            "L_delta_from_original": delta,
        }
        break


print(json.dumps(result, indent=2))
