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

local megaman_sprite = asset.sprite("megaman")

local function style_form(spec: {
    palette: number,
    charge_shot: Weapon?,
    back_special: Weapon?,
    status_reset: { FormEffect }?,
}): FormDef
    return {
        sprite = megaman_sprite,
        element = "null",
        buster_bonus = 0,
        change = STYLE_CHANGE,
        weapons = {
            buster = buster.weapon,
            charge_shot = spec.charge_shot or charged_shot.weapon,
            back_special = spec.back_special,
        },
        palette = spec.palette,
        status_reset = spec.status_reset,
    }
end

return {
    guts = style_form {
        palette = 6,
        status_reset = { "super_armor" },
    },
    custom = style_form {
        palette = 9,
    },
    team = style_form {
        palette = 27,
    },
    shield = style_form {
        palette = 24,
        back_special = guards.shield,
    },
    ground = style_form {
        palette = 18,
        charge_shot = charged_shot.ground,
    },
    shadow = style_form {
        palette = 15,
        back_special = guards.anti_dmg,
    },
    bug = style_form {
        palette = 21,
    },
}
