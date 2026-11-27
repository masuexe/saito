--!strict
-- EXE3 RockCube (0x08D): places a 200-HP RockCube obstacle on the panel ahead.

local collision = require("../rules/collision")
local common_cube = require("@exelib/rockcube/cube")
local common_debris = require("@exelib/rock/debris")
local common_rock = require("@exelib/rock")
local sparks = require("../lib/sparks")

local CUBE_SPRITE = asset.sprite("grab-shot")
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 16 }
local DUST_FX = new.effect { sprite = asset.sprite("dust"), anim = 0, palette = 0, steps = 16 }

local DEBRIS = common_debris.make_kind("rockcube/debris", CUBE_SPRITE)

local ROCK = common_rock.make {
    id = "rockcube/rock",
    sprite = CUBE_SPRITE,
    reloads_anim = false,
    debris = DEBRIS,
    effects = { puff = PUFF_FX, dust = DUST_FX },
    spark = sparks.breaking,
    is = collision.navi,
    hits = collision.navi_target,
    leaves_by_hp = true,
    crush = "destroys",
}

local CUBE_VARIANT = common_rock.variant {
    anim = 0,
    hp = 200,
    element = "null",
    debris_palette = 0,
    break_sound = asset.sound("panel-crack"),
    identity = nil,
}

local CONTROLLER = common_cube.make {
    id = "rockcube/controller",
    spawn_rock = ROCK.spawn,
    place_sound = asset.sound("grab-shot-land"),
    side_only = true,
}

local rockcube: Chip = {
    codes = { "A", "C", "E", "H", "R", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 9,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.places { variant = CUBE_VARIANT, class = 0, entrance = "rise" },
}

return { rockcube = rockcube }
