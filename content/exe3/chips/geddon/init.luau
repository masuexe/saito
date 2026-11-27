--!strict
-- EXE3 Geddon1 (compat/chips.toml ID 0x08A): cracks all solid panels on the field.

local controller = require("@self/controller")

local quake = controller.quake

local CRACK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.crack(x, y)
    end,
    show = quake.puff,
}

local geddon1: Chip = {
    codes = { "D", "J", "M", "O", "S", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(CRACK),
}

return { geddon1 = geddon1 }
