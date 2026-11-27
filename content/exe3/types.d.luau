-- Types shared across the exe3 pack's modules.

-- EXE3 Style Change types (packed in save byte 0x1881 as `(style_type << 3) | element`).
export type StyleType = "normal" | "guts" | "custom" | "team" | "shield" | "ground" | "shadow" | "bug"

-- EXE3 Style Change elements (0=null, 1=elec, 2=heat, 3=aqua, 4=wood).
export type StyleElement = "null" | "elec" | "heat" | "aqua" | "wood"
