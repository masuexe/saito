--!strict
-- EXE3 PanlOut1 and PanlOut3 (compat/chips.toml IDs 0x077..0x078).

local crack = require("../lib/crack")

local panlout1: Chip = {
    codes = { "A", "B", "D", "L", "S", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    action = crack.action { id = "panlout1/action", panels = crack.ahead },
}

local panlout3: Chip = {
    codes = { "C", "E", "N", "R", "Y", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    action = crack.action { id = "panlout3/action", panels = crack.column },
}

return { panlout1 = panlout1, panlout3 = panlout3 }
