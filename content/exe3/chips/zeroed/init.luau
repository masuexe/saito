--!strict
-- EXE3 zeroed chip (role `chips.zeroed`).

local attachment = require("../objects/attachment")
local cannon_lib = require("../lib/cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")
local cannon_sprite = asset.sprite("cannon")

local ARM = attachment.look { sprite = cannon_sprite, palette = 0 }
local SHOT = projectile.variant {
    id = "zeroed/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

local zeroed: Chip = {
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    action = cannon_lib.action { id = "zeroed/action", arm = ARM, shot = SHOT },
}

return { zeroed = zeroed }
