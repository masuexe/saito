# saito UI Themes & `.saitotheme` Plugin Specification

**Status: v1 design (2026-11-28).**

This document specifies saito's built-in **HD Vector PET (`pet-hd`)** theme, the preserved **Modern Terminal (`terminal`)** built-in theme, and the **`.saitotheme` UI theme plugin format** (schema v1).

---

## 1. Design Principles

1. **Presentation-only isolation.** Themes and plugins alter colors, surface geometry parameters, backdrop patterns/images, and optional fonts. Nothing in a theme touches `saito-battle`, rollback simulation, netplay protocol, or replay digests.
2. **Zero external ROM dependency.** Both built-in themes (`pet-hd` and `terminal`) are 100% vector-drawn in Slint so desktop, mobile, web, and first-launch screens render identically without requiring any Legacy Collection archive.
3. **Layout stability.** Themes do not change fixed control dimensions, row heights, touch target bounds (`Reach` 44px minimum), or focus geometry. Keyboard/gamepad navigation and CJK text-clipping audits remain intact across all themes.
4. **Slint build-cost discipline.** Multi-style rendering uses shared static layers driven by theme colors and numeric parameters rather than duplicating component subtrees behind `if` conditionals (`docs/app.md §14`).

---

## 2. Built-in Themes

| Theme ID | Display Name (zh-Hans / ja / ko) | Description |
| :--- | :--- | :--- |
| `pet-hd` (default) | 高清 PET / HD PET / HD PET | HD vector PET aesthetic inspired by Battle Network Legacy Collection: dual-tone cyan/royal-blue geometric frames, chamfered 3-state buttons with hard drop-offset shadows, vibrant orange-gold active highlight, slanted progress/slider bars, and per-game EXE4/5/6 palettes. |
| `terminal` | 现代终端 / モダン端末 / 모던 터미널 | The original dark network terminal look driven by `content/<game>/look.toml`. |

### `pet-hd` Per-Game Palettes

| Role | EXE4 (Violet PET) | EXE5 (Emerald PET) | EXE6 (Cyber-Blue PET) |
| :--- | :--- | :--- | :--- |
| `ground_top` / `ground_bottom` | `#321D78` / `#120933` | `#0B4A5E` / `#06212C` | `#0B3C91` / `#061B47` |
| `frame_cyan` (primary cursor/frame) | `#C49BFF` | `#59F7D6` | `#5AF4FF` |
| `frame_stroke` (outer outline) | `#4B249E` | `#0A6368` | `#0A44B8` |
| `panel` / `inset` | `#1C1144` / `#10092C` | `#0A2C38` / `#061A23` | `#09244A` / `#051630` |
| `raised` (button normal fill) | `#221459` | `#0B3A4A` | `#092E59` |
| `btn_shadow` (hard drop shadow) | `#0E072B` | `#041C24` | `#051B38` |
| `slot_bg` / `slot_border` (checkboxes & chip slots) | `#3B2280` / `#D2B4FF` | `#125C63` / `#7EFCE2` | `#15458A` / `#7AE9FF` |
| `bar_start` / `bar_end` (progress & slider fill) | `#8F5CFF` / `#D4B8FF` | `#12B89A` / `#6AFCE0` | `#1A9BFF` / `#5AF4FF` |
| `active_top` / `active_bot` (selected highlight) | `#FFA408` / `#FF7800` | `#FFA408` / `#FF7800` | `#FFA408` / `#FF7800` |
| `active_border` / `active_shadow` | `#FFF4D6` / `#D98200` | `#FFF4D6` / `#D98200` | `#FFF4D6` / `#D98200` |
| `text` / `muted` / `dim` | `#F2F9FF` / `#C4B5EA` / `#7B69AB` | `#F2F9FF` / `#9ADBD0` / `#558C85` | `#F2F9FF` / `#9BC4EE` / `#5C83B0` |
| `pattern.kind` | `pet_grid` | `pet_grid` | `pet_grid` |

---

## 3. `.saitotheme` Plugin Format (Schema v1)

### Storage & Discovery
- Themes are discovered from `<app_data>/themes/` (overridable via `$SAITO_THEMES`):
  - **Folder**: `<themes>/<folder>/theme.toml`
  - **Archive**: `<themes>/<name>.saitotheme` (standard ZIP containing `theme.toml` at root or inside a single top-level directory)
- Importing a `.saitotheme` file from Settings validates `theme.toml`, copies the archive into `<app_data>/themes/`, registers it, and applies it immediately.
- Any invalid or unreadable theme logs its reason to `saito.log` and falls back cleanly without breaking startup.

### Regional Availability Gate (Mainland China Only)
- Custom themes and theme switching are restricted to mainland China (`CN`) IPs verified against the fixed official endpoint `https://matchmaking.exessrhub.top/geo` (decoupled from user-configured signaling servers), with automatic fallback to `https://www.cloudflare.com/cdn-cgi/trace` and `https://myip.ipip.net/json` if the primary endpoint is unavailable.
- **Fail-closed default**: First launches without a verified cache lock to `pet-hd` and hide both theme rows in Settings until `CN` is confirmed.
- **30-day offline grace**: Confirmed `CN` verdicts are persisted in an integrity-checked cache (`<app_data>/region.cache`) so mainland users keep their themes offline for up to 30 days.
- **Soft override**: Detecting a non-`CN` IP immediately forces `pet-hd` at runtime and hides the theme UI without overwriting `settings.ui_theme` on disk, so mainland users recover their saved theme after turning off a VPN or returning to China.

### `theme.toml` Schema

All fields under `[style]`, `[colors]`, `[backdrop]`, `[fonts]`, and `[games.<id>]` are **optional** and inherit from `extends` (default `"pet-hd"`). Colors accept either `0xRRGGBB` integers or `"#RRGGBB"` hex strings.

```toml
schema = 1
id = "my-custom-pet"
name = "My Custom PET"
author = "Author Name"
version = "1.0.0"
extends = "pet-hd" # "pet-hd" or "terminal"

[style]
kind = "pet" # "pet" or "terminal"

[colors]
ground_top = "#0B3C91"
ground_bottom = "#061B47"
glow = "#5AF4FF"
glow_strength = 0.18
panel = "#09244A"
raised = "#092E59"
line = "#5AF4FF"
inset = "#051630"
text = "#F2F9FF"
muted = "#9BC4EE"
dim = "#5C83B0"
cursor = "#5AF4FF"
action = "#FF9400"
action_ink = "#FFFFFF"
# PET structural colors (optional; derived automatically if omitted)
frame_stroke = "#0A44B8"
btn_shadow = "#051B38"
active_top = "#FFA408"
active_bot = "#FF7800"
active_border = "#FFF4D6"
active_shadow = "#D98200"
slot_bg = "#15458A"
slot_border = "#7AE9FF"
bar_start = "#1A9BFF"
bar_end = "#5AF4FF"

[backdrop]
kind = "pet_grid" # "pet_grid" | "hexagons" | "isometric_grid" | "starfield"
color = "#60FAFF"
strength = 0.11
# Optional custom background image relative to theme root (PNG/JPEG/SVG)
# image = "assets/bg.png"
# image_opacity = 0.35

[fonts]
# Optional custom fonts relative to theme root (TTF/OTF/WOFF2)
# display = "fonts/MyDisplay.ttf"
# body = "fonts/MyBody.ttf"
# mono = "fonts/MyMono.ttf"

# Optional per-game overrides (inherits from top-level [colors] & [backdrop])
[games.exe4.colors]
ground_top = "#321D78"
ground_bottom = "#120933"
line = "#C49BFF"
cursor = "#C49BFF"
frame_stroke = "#4B249E"

[games.exe5.colors]
ground_top = "#0B4A5E"
ground_bottom = "#06212C"
line = "#59F7D6"
cursor = "#59F7D6"
frame_stroke = "#0A6368"
```
