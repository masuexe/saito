--!strict
-- EXE3 Program Advances: Z-Canon1 (0x140) and LifeSwrd (0x150).

local attachment = require("../objects/attachment")
local blades = require("./sword")
local cannon_lib = require("../lib/cannon")
local cannons = require("./cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local swords = require("../lib/swords")

local cannon_sprite = asset.sprite("cannon")
local big_slash_sprite = asset.sprite("big-slash")

local Z_CANON_ARM = attachment.look { sprite = cannon_sprite, palette = 0 }
local Z_CANON_SHOT = projectile.variant {
    id = "z-canon1/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 5,
    element = "null",
    spark = sparks.blast,
}

local z_canon1: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 200,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 0, code_run = { chip = cannons.cannon, count = 3 } },
    },
    action = cannon_lib.action { id = "z-canon1/action", arm = Z_CANON_ARM, shot = Z_CANON_SHOT, afterimages = true },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

local BIG_SLASH = new.effect { sprite = big_slash_sprite, anim = 0, palette = 0, steps = 15 }
local SWORD_HIT = { hits = collision.hits_navis, is = collision.slash }

local lifeswrd: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 1, sequence = { blades.sword, blades.wideswrd, blades.longswrd } },
    },
    action = swords.slash {
        id = "lifeswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = SWORD_HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
    auto_battle = { class = "into_reach", range = "two_columns_ahead" },
}

return {
    ["z-canon1"] = z_canon1,
    lifeswrd = lifeswrd,
}
