--!strict
-- EXE3 Swords: Sword, WideSwrd, LongSwrd, FireSwrd, AquaSwrd, ElecSwrd, BambSwrd, StepSwrd.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local SLASH = asset.sprite("sword-slash")
local SWORD_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 0, steps = 12 }
local FIRE_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 1, steps = 15 }
local AQUA_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 2, steps = 15 }
local ELEC_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 3, steps = 15 }
local WOOD_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 4, steps = 15 }

local function slash(id: string, region: Region, effect: Effect): Action
    return swords.slash {
        id = id,
        blade = swords.blade,
        hit = { region = region, collision = HIT, hit_mod = 3 },
        effect = effect,
        sound = swords.swing,
    }
end

local sword: Chip = {
    action = slash("sword/action", regions.single, SWORD_SLASH),
    codes = { "E", "H", "L", "S", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local wideswrd: Chip = {
    action = slash("wideswrd/action", regions.wide, SWORD_SLASH),
    codes = { "C", "E", "L", "Q", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local longswrd: Chip = {
    action = slash("longswrd/action", regions.long, SWORD_SLASH),
    codes = { "E", "I", "L", "R", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local fireswrd: Chip = {
    action = slash("fireswrd/action", regions.wide, FIRE_SLASH),
    codes = { "F", "N", "P", "R", "U" },
    element = "fire",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local aquaswrd: Chip = {
    action = slash("aquaswrd/action", regions.wide, AQUA_SLASH),
    codes = { "A", "H", "N", "P", "T" },
    element = "aqua",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elecswrd: Chip = {
    action = slash("elecswrd/action", regions.wide, ELEC_SLASH),
    codes = { "E", "K", "N", "P", "V" },
    element = "elec",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bambswrd: Chip = {
    action = slash("bambswrd/action", regions.wide, WOOD_SLASH),
    codes = { "B", "L", "N", "P", "W" },
    element = "wood",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local stepswrd: Chip = {
    action = swords.step_slash {
        id = "stepswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide, collision = HIT, hit_mod = 3 },
        effect = SWORD_SLASH,
        sound = swords.swing,
    },
    codes = { "L", "M", "N", "O", "P" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 43,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    sword = sword,
    wideswrd = wideswrd,
    longswrd = longswrd,
    fireswrd = fireswrd,
    aquaswrd = aquaswrd,
    elecswrd = elecswrd,
    bambswrd = bambswrd,
    stepswrd = stepswrd,
}
