--!strict
-- EXE3's MegaMan (Battle sprite 04-00, Normal + 7 Style Change forms).

local buster = require("@self/weapons/buster")
local charged_shot = require("@self/weapons/charged_shot")
local styles = require("@self/forms/styles")
local megaman_portrait_sprite = asset.sprite("portrait-megaman")
local megaman_sprite = asset.sprite("megaman")

local navi: NaviDef = {
    sprite = megaman_sprite,
    element = "null",
    buster_bonus = 0,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("megaman-deleted"), lose = asset.banner("megaman-deleted") },
    run_message = {
        portrait = megaman_portrait_sprite,
    },
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    fresh = { hp = 100, mega_level = 5, giga_level = 1 },
    forms = {
        styles = {
            form_list = {
                styles.guts,
                styles.custom,
                styles.team,
                styles.shield,
                styles.ground,
                styles.shadow,
                styles.bug,
            },
        },
    },
    identity = new.identity {
        class = "player",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        chip_icons_at = { 0, -55 },
        status_mark = { 8, 50 },
    },
}

return { megaman = navi }
