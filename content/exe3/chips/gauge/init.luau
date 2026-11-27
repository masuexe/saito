--!strict
-- EXE3 FstGauge (0x09E) and SloGauge (0x09D).

local gauge_speed = require("../lib/gauge_speed")

local fstgauge: Chip = {
    codes = { "B", "E", "J", "R", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = gauge_speed.fast,
    auto_battle = { class = "at_once" },
}

local slogauge: Chip = {
    codes = { "A", "C", "R", "S", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = gauge_speed.slow,
    auto_battle = { class = "at_once" },
}

return { fstgauge = fstgauge, slogauge = slogauge }
