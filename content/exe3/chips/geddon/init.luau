--!strict
-- EXE3 Geddon1 (0x08A), Geddon2 (0x08B), and Geddon3 (0x08C).

local controller = require("@self/controller")

local quake = controller.quake

local CRACK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.crack(x, y)
    end,
    show = quake.puff,
}

local BREAK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.break_panel(x, y)
    end,
    show = quake.puff,
}

local POISON = quake.change {
    apply = quake.poison,
    show = quake.bubbles,
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

local geddon2: Chip = {
    codes = { "F", "H", "N", "O", "W" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 3,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(BREAK),
}

local geddon3: Chip = {
    codes = { "C", "M", "U", "W", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 90,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(POISON),
}

return {
    geddon1 = geddon1,
    geddon2 = geddon2,
    geddon3 = geddon3,
}
