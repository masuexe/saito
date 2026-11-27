--!strict
-- EXE3 Bombs: MiniBomb, SnglBomb, DublBomb, TrplBomb, CannBall (compat/chips.toml IDs 0x014..0x018).

local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local function bomb_variant(region: Region, palette: number, breaking: boolean?): BombVariant
    return bombs.variant {
        collision = {
            is = if breaking then collision.thrown_break else collision.thrown,
            hits = collision.hits_navis,
        },
        hit_mod = 3,
        palette = palette,
        anim = 1,
        blast = region,
    }
end

local MINI = bomb_variant(regions.single, 0, false)
local SNGL = bomb_variant(regions.single, 1, false)
local DUBL = bomb_variant(regions.long, 2, false)
local TRPL = bomb_variant(regions.line, 3, false)
local CANNBALL = bomb_variant(regions.single, 0, true)

local minibomb: Chip = {
    action = bombs.throw { id = "minibomb/action", held = bombs.held, held_anim = 0, thrower = bombs.thrower(MINI) },
    codes = { "B", "G", "L", "O", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local snglbomb: Chip = {
    action = bombs.throw { id = "snglbomb/action", held = bombs.held, held_anim = 0, thrower = bombs.thrower(SNGL) },
    codes = { "D", "F", "H", "J", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 12,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local dublbomb: Chip = {
    action = bombs.throw { id = "dublbomb/action", held = bombs.held, held_anim = 0, thrower = bombs.thrower(DUBL) },
    codes = { "A", "C", "H", "K", "Q", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local trplbomb: Chip = {
    action = bombs.throw { id = "trplbomb/action", held = bombs.held, held_anim = 0, thrower = bombs.thrower(TRPL) },
    codes = { "E", "I", "N", "P", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local cannball: Chip = {
    action = bombs.throw { id = "cannball/action", held = bombs.held, held_anim = 0, thrower = bombs.thrower(CANNBALL) },
    codes = { "A", "D", "F", "L", "P", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    minibomb = minibomb,
    snglbomb = snglbomb,
    dublbomb = dublbomb,
    trplbomb = trplbomb,
    cannball = cannball,
}
