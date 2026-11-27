--!strict
-- EXE3 AirShot1 (compat/chips.toml ID 0x004).

local airshot = require("../lib/airshot")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

local SHOT = projectile.variant {
    id = "airshot1/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 1,
    element = "null",
    spark = sparks.plain,
}

local airshot1: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = airshot.action { id = "airshot1/action", shooter = airshot.shooter, shot = SHOT, recovery = 10 },
}

return { airshot1 = airshot1 }
