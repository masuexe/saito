--!strict
-- EXE3 AreaGrab and PanlGrab (compat/chips.toml IDs 0x081..0x082).

local grab = require("../lib/grab")

local panlgrab: Chip = {
    codes = { "A", "H", "L", "S", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = grab.panel,
}

local areagrab: Chip = {
    codes = { "E", "L", "R", "S", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 15,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = grab.area,
}

return { panlgrab = panlgrab, areagrab = areagrab }
