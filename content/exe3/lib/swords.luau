--!strict
-- EXE3's sword action wrapper over @exelib/swords/slash (including StepSwrd step-forward slashes).

local anims = require("./anims")
local attachment = require("../objects/attachment")
local slash = require("@exelib/swords/slash")
local sword_sprite = asset.sprite("sword")

local swords = {}

local NAVI_NUMBER = 0

-- Held sword blade attachments verified against A6BE `0x080B176C` (`0c-00` standard, `0c-08..0b` elemental).
swords.blade = attachment.look { sprite = sword_sprite, palette = 0 }
swords.fire_blade = attachment.look { sprite = asset.sprite("fire-sword"), palette = 0 }
swords.aqua_blade = attachment.look { sprite = asset.sprite("aqua-sword"), palette = 0 }
swords.elec_blade = attachment.look { sprite = asset.sprite("elec-sword"), palette = 0 }
swords.wood_blade = attachment.look { sprite = asset.sprite("wood-sword"), palette = 0 }
swords.swing = asset.sound("sword-swing")
swords.big_swing = asset.sound("big-sword-swing")

local NO_STEP: SlashSteps = {
    steps = function(_me: Object): boolean
        return false
    end,
    kind = function(_me: Object, _set_off: (Object) -> boolean): ("step" | "warp")?
        return nil
    end,
    warp_target = function(_me: Object): (number?, number?)
        return nil, nil
    end,
}

local ALWAYS_STEP: SlashSteps = {
    steps = function(_me: Object): boolean
        return true
    end,
    kind = function(me: Object, _set_off: (Object) -> boolean): ("step" | "warp")?
        if not me:can_move() then
            return nil
        end
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        return "step"
    end,
    warp_target = function(_me: Object): (number?, number?)
        return nil, nil
    end,
}

local function make_game(steps: SlashSteps): SlashGame
    return {
        steps = steps,
        warp_out_steps = function(_: Object): number
            return 4
        end,
        blade_anim = function(_me: Object): number
            return NAVI_NUMBER
        end,
        navi_afterimage = function(me: Object): SpriteAsset
            return battle.navi(me.alliance).form.sprite
        end,
        hit_mod = function(_me: Object, chip_hit_mod: number): number
            return chip_hit_mod
        end,
        hold = function(me: Object, blade: AttachmentLook, anim: number)
            attachment.attach(me, blade, "related", anim)
        end,
        refreshes_form_overlay = false,
        recovery_clears_moving = false,
        swing_anim = anims.SWORD_SWING,
        warp_out_anim = anims.MOVE_DEPART,
        idle_anim = anims.idle,
    }
end

local GAME = make_game(NO_STEP)
local STEP_GAME = make_game(ALWAYS_STEP)

function swords.slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, GAME)
end

function swords.step_slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, STEP_GAME)
end

return swords
