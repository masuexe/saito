--!strict
-- EXE3's 11 panel types (compat/panels.toml, A6BE IWRAM 0x03006E84):
-- missing (0), broken (1), normal (2), cracked (3), poison (4), metal (5),
-- grass (6), ice (7), lava (8), holy (9), sand (10).

local panel_types = require("@exelib/panel_types")
local sparks = require("../lib/sparks")

local BURN = panel_types.burn {
    damage = 50,
    spared_by = { "invisible", "submerged", "flashing" },
    mood = 0,
    players_while_dimmed = true,
    spark = sparks.fire,
}

return {
    types = {
        missing = { flags = 0x00018000 },
        broken = { flags = 0x00014000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110, trail_sound = asset.sound("panel-poison"), stand = panel_types.poison },
        metal = { flags = 0x00030010, unbreakable = true },
        grass = { flags = 0x00010410, doubles = "fire", stand = panel_types.grass(nil), cleared_by = "fire" },
        ice = {
            flags = 0x00010810,
            doubles = "elec",
            move_end = panel_types.ice_slides,
            slide = panel_types.ice,
        },
        lava = { flags = 0x00011010, expires = 960, burn = BURN, cleared_by = "aqua" },
        holy = { flags = 0x00012010, damage_shift = 1 },
        sand = {
            flags = 0x00010210,
            traps = true,
            slide = panel_types.pitfall,
        },
    },
    numbers = {
        "missing",
        "broken",
        "normal",
        "cracked",
        "poison",
        "metal",
        "grass",
        "ice",
        "lava",
        "holy",
        "sand",
    },
    roles = { missing = "missing", broken = "broken", cracked = "cracked", normal = "normal" },
    start_visible = {
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
    },
    front_edges = {
        { false, false, false, false, false, false, false, false },
        { false, false, false, false, false, false, false, false },
        { false, false, false, false, false, false, false, false },
        { false, true, true, true, true, true, true, false },
        { false, false, false, false, false, false, false, false },
    },
    step = {
        grounded = {
            { require = 0x00000010, forbid = 0x0b8800a0 },
            { require = 0x00000030, forbid = 0x07880080 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x0b8800a0 },
            { require = 0x00000020, forbid = 0x07880080 },
        },
    },
    dash_step = {
        grounded = {
            { require = 0x00000010, forbid = 0x0f8800a0 },
            { require = 0x00000030, forbid = 0x0f880080 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x0f8800a0 },
            { require = 0x00000020, forbid = 0x0f880080 },
        },
    },
    any_side_step = {
        grounded = {
            { require = 0x00000010, forbid = 0x07800000 },
            { require = 0x00000010, forbid = 0x0B800000 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x07800000 },
            { require = 0x00000000, forbid = 0x0B800000 },
        },
    },
    mend = { normal = 600, battle_mode_1 = 600 },
    reservations = "unmarked",
    type_mask = 0x00023F5F,
    drawn = "own_tables",
}
