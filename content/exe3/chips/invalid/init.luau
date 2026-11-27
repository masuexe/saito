--!strict
-- EXE3 invalid chip placeholder (role `chips.invalid`).

local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local invalid: Chip = {
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = {},
    action = spawners.action { id = "invalid/action", spawn = plus.spawner(0) },
}

return { invalid = invalid }
