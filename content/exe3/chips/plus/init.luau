--!strict
-- EXE3 Atk+10 (0x0C3) and Atk+30 (0x0CD).

local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local atk_10: Chip = {
    modifier = "attack_plus",
    action = spawners.action { id = "atk-10/action", spawn = plus.spawner(10) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local atk_30: Chip = {
    modifier = "attack_plus",
    action = spawners.action { id = "atk-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    auto_battle = { class = "at_once" },
}

return { ["atk-10"] = atk_10, ["atk-30"] = atk_30 }
