--!strict
-- EXE3's 7 Style Change forms (pre-battle styles equipped from save byte 0x1881):
-- Guts, Custom, Team, Shield, Ground, Shadow, Bug.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")
local guards = require("../weapons/guards")

local STYLE_CHANGE = new.action {
    id = "styles/noop-change",
    update = function(me: Object)
        me:exit_attack()
    end,
}

local function style_form(spec: {
    sprite: SpriteAsset,
    charge_shot: Weapon?,
    back_special: Weapon?,
    status_reset: { FormEffect }?,
}): FormDef
    return {
        sprite = spec.sprite,
        element = "null",
        buster_bonus = 0,
        change = STYLE_CHANGE,
        weapons = {
            buster = buster.weapon,
            charge_shot = spec.charge_shot or charged_shot.weapon,
            back_special = spec.back_special,
        },
        palette = 0,
        status_reset = spec.status_reset,
    }
end

return {
    guts = style_form {
        sprite = asset.sprite("megaman-guts"),
        status_reset = { "super_armor" },
    },
    custom = style_form {
        sprite = asset.sprite("megaman-custom"),
    },
    team = style_form {
        sprite = asset.sprite("megaman-team"),
    },
    shield = style_form {
        sprite = asset.sprite("megaman-shield"),
        back_special = guards.shield,
    },
    ground = style_form {
        sprite = asset.sprite("megaman-ground"),
        charge_shot = charged_shot.ground,
    },
    shadow = style_form {
        sprite = asset.sprite("megaman-shadow"),
        back_special = guards.anti_dmg,
    },
    bug = style_form {
        sprite = asset.sprite("megaman-bug"),
    },
}
