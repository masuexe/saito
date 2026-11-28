--!strict
-- EXE3's 7 Style Change forms (pre-battle styles equipped from save byte 0x1881):
-- Guts, Custom, Team, Shield, Ground, Shadow, Bug.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")

local STYLE_CHANGE = new.action {
    id = "styles/noop-change",
    update = function(me: Object)
        me:exit_attack()
    end,
}

local megaman_sprite = asset.sprite("megaman")

local function style_form(spec: {
    palette: number,
    idle_anim: number,
    charge_shot: Weapon?,
    back_special: Weapon?,
    status_reset: { FormEffect }?,
    rapid_presses: { presses: number, window: number }?,
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
        idle_anim = spec.idle_anim,
        status_reset = spec.status_reset,
        rapid_presses = spec.rapid_presses,
        traits = { "stats_element", "stats_charge_shot" },
    }
end

return {
    guts = style_form {
        palette = 6,
        idle_anim = 1,
        -- A6BE 0x080B55FA: 5 rapid B presses within 10 ticks trigger Guts Machine Gun (`actions.forced_charged_shot`).
        -- No innate SuperArmor in EXE3 (A6BE 0x0804735E): SuperArmor comes from the SprArmor NCP.
        rapid_presses = { presses = 5, window = 10 },
    },
    custom = style_form {
        palette = 9,
        idle_anim = 2,
    },
    team = style_form {
        palette = 27,
        idle_anim = 3,
    },
    shield = style_form {
        palette = 24,
        idle_anim = 4,
    },
    ground = style_form {
        palette = 18,
        idle_anim = 5,
        charge_shot = charged_shot.ground,
    },
    shadow = style_form {
        palette = 15,
        idle_anim = 6,
    },
    bug = style_form {
        palette = 21,
        idle_anim = 7,
    },
}
