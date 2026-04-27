"""Find a working tint percentage for category badges.
The 14% spec fails AA for control violet (3.71x). Try higher mixes.
"""

import math


def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) / 255.0 for i in (0, 2, 4))


def rgb_to_hex(rgb):
    r, g, b = (max(0, min(255, round(c * 255))) for c in rgb)
    return f"#{r:02X}{g:02X}{b:02X}"


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def relative_luminance(hex_color):
    r, g, b = hex_to_rgb(hex_color)
    r, g, b = srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast_ratio(fg, bg):
    L1, L2 = relative_luminance(fg), relative_luminance(bg)
    lighter, darker = max(L1, L2), min(L1, L2)
    return (lighter + 0.05) / (darker + 0.05)


def color_mix(fg_hex, bg_hex, alpha):
    fr, fg_, fb = hex_to_rgb(fg_hex)
    br, bg_, bb = hex_to_rgb(bg_hex)
    return rgb_to_hex(
        (fr * alpha + br * (1 - alpha),
         fg_ * alpha + bg_ * (1 - alpha),
         fb * alpha + bb * (1 - alpha)),
    )


HEX = {
    "bg":             "#14172A",
    "bg_elevated":    "#1E223A",
    "processing":     "#3FB8B0",
    "output":         "#C99846",
    "control":        "#9A78D6",
}


def explore_on_bg_elevated(category):
    print(f"\n--- {category} ({HEX[category]}) on bg-elevated #1E223A ---")
    for pct in [0.10, 0.12, 0.14, 0.16, 0.18, 0.20, 0.22, 0.24, 0.26, 0.28, 0.30]:
        composited = color_mix(HEX[category], HEX["bg_elevated"], pct)
        ratio = contrast_ratio(HEX[category], composited)
        flag = "PASS" if ratio >= 4.5 else "fail"
        print(f"  {int(pct*100):>3}%: bg={composited}  ratio={ratio:.2f}  {flag}")


def explore_on_bg(category):
    print(f"\n--- {category} ({HEX[category]}) on bg #14172A ---")
    for pct in [0.10, 0.12, 0.14, 0.16, 0.18, 0.20, 0.22, 0.24, 0.26, 0.28, 0.30]:
        composited = color_mix(HEX[category], HEX["bg"], pct)
        ratio = contrast_ratio(HEX[category], composited)
        flag = "PASS" if ratio >= 4.5 else "fail"
        print(f"  {int(pct*100):>3}%: bg={composited}  ratio={ratio:.2f}  {flag}")


# Also try lifting the violet hue itself
def explore_lifted_control():
    print("\n\n=== Lifting control violet itself (keeping H=299, C=0.14) ===")
    # original: oklch(0.645 0.14 299)
    # We need higher L for legibility. Compute matching hex for various L values.
    # Approximate: every +0.05 L on this OKLCH ≈ +20 hex points in each channel.
    # Let's just try some hand-picked hues by trial:
    candidates = [
        "#9A78D6",  # original
        "#A98AE0",  # +12 L
        "#B89BEA",  # +25 L
        "#C7AEF4",  # +35 L
    ]
    for cand in candidates:
        # Test at original 14% mix
        composited = color_mix(cand, HEX["bg_elevated"], 0.14)
        ratio = contrast_ratio(cand, composited)
        flag = "PASS" if ratio >= 4.5 else "fail"
        print(f"  {cand}: bg={composited}  ratio={ratio:.2f}  {flag}")


for cat in ("processing", "output", "control"):
    explore_on_bg_elevated(cat)

print("\n========================================")
for cat in ("processing", "output", "control"):
    explore_on_bg(cat)

explore_lifted_control()
