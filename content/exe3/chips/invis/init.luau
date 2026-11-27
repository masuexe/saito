--!strict
-- EXE3 Invis (compat/chips.toml ID 0x0A0): turns user invisible for 360 ticks.

local invisible = require("@self/controller")

local chip: Chip = {
    codes = { "B", "E", "F", "R", "S", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 11,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = invisible.hook(360),
    auto_battle = { class = "when_visible" },
}

return { invis = chip }
