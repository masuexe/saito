--!strict
-- EXE3 Recov10..Recov300 (compat/chips.toml IDs 0x079..0x080).

local recov = require("@self/recov")

local recov10: Chip = {
    codes = { "A", "C", "E", "G", "L", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov10/action", hp = 10 },
}

local recov30: Chip = {
    codes = { "B", "D", "F", "H", "M", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov30/action", hp = 30 },
}

local recov50: Chip = {
    codes = { "C", "E", "G", "I", "N", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov50/action", hp = 50 },
}

local recov80: Chip = {
    codes = { "D", "F", "H", "J", "O", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov80/action", hp = 80 },
}

local recov120: Chip = {
    codes = { "O", "Q", "S", "U", "W", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov120/action", hp = 120 },
}

local recov150: Chip = {
    codes = { "N", "P", "R", "T", "V", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov150/action", hp = 150 },
}

local recov200: Chip = {
    codes = { "M", "N", "U", "V", "W" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 3,
    mb = 65,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov200/action", hp = 200 },
}

local recov300: Chip = {
    codes = { "O", "R", "V", "W", "Z" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 3,
    mb = 80,
    damage = 0,
    hit_param = 0,
    lockout = 15,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov300/action", hp = 300 },
}

return {
    recov10 = recov10,
    recov30 = recov30,
    recov50 = recov50,
    recov80 = recov80,
    recov120 = recov120,
    recov150 = recov150,
    recov200 = recov200,
    recov300 = recov300,
}
