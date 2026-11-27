--!strict
-- EXE3 Cannon, HiCannon, and M-Cannon (compat/chips.toml IDs 0x001..0x003).

local attachment = require("../objects/attachment")
local cannon_lib = require("../lib/cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")
local cannon_sprite = asset.sprite("cannon")

local CANNON_ARM = attachment.look { sprite = cannon_sprite, palette = 0 }
local HICANNON_ARM = attachment.look { sprite = cannon_sprite, palette = 1 }
local M_CANNON_ARM = attachment.look { sprite = cannon_sprite, palette = 2 }

local SHOT = projectile.variant {
    id = "cannon/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

local cannon: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 12,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannon_lib.action { id = "cannon/action", arm = CANNON_ARM, shot = SHOT },
}

local hicannon: Chip = {
    codes = { "H", "I", "J", "K", "L", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannon_lib.action { id = "hicannon/action", arm = HICANNON_ARM, shot = SHOT },
}

local m_cannon: Chip = {
    codes = { "O", "P", "Q", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = cannon_lib.action { id = "m-cannon/action", arm = M_CANNON_ARM, shot = SHOT },
}

return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
