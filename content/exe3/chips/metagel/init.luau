--!strict
-- EXE3 MetaGel1 (0x085), MetaGel2 (0x086), MetaGel3 (0x087):
-- drops aqua gels on the opponent's front column in each row, dealing aqua damage and stealing panels!

local collision = require("../rules/collision")
local common_controller = require("@exelib/metagel/controller")
local common_gel = require("@exelib/metagel/gel")
local sparks = require("../lib/sparks")

local GEL_SPRITE = asset.sprite("grab-shot")
local PUFF_FX = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0, steps = 8 }

-- Cannot steal the opponent's back column (col 6 for side 0, col 1 for side 1).
local function takes(x: number, _y: number, side: number): boolean
    return x ~= bit32.bxor(side, 1) * 5 + 1
end

local GEL = common_gel.make {
    id = "metagel/gel",
    sprite = GEL_SPRITE,
    drop = asset.sound("grab-shot-fall"),
    land = asset.sound("grab-shot-land"),
    spark = sparks.aqua,
    is = collision.thrown,
    hits = collision.hits_navis,
    takes = takes,
    puff = PUFF_FX,
}

local CONTROLLER = common_controller.make {
    id = "metagel/controller",
    gel = GEL.spawn,
    front = common_controller.nearest_other,
}

local metagel1: Chip = {
    codes = { "B", "C", "D", "K", "Y", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 18,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
}

local metagel2: Chip = {
    codes = { "E", "F", "G", "P", "S" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
}

local metagel3: Chip = {
    codes = { "G", "H", "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 170,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
}

return {
    metagel1 = metagel1,
    metagel2 = metagel2,
    metagel3 = metagel3,
}
