--!strict
-- EXE3's Cannon action wrapper over @exelib/cannon.

local anims = require("./anims")
local attachment = require("../objects/attachment")
local common = require("@exelib/cannon")
local projectile = require("./projectile")

local cannon = {}

cannon.STATE = common.STATE

local NAVI_NUMBER = 0
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8318, 0x1E

local function afterimages(me: Object, spec: CannonSpec, flip: number)
    local form = battle.navi(me.alliance).form
    battle.afterimage(me, me.pos, {
        sprite = form.sprite,
        anim = anims.CANNON_RAISE,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
    battle.afterimage(me, me.pos, {
        sprite = spec.arm.sprite,
        anim = NAVI_NUMBER,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
    })
end

local LOOK: CannonLook = {
    sound = asset.sound("cannon"),
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, NAVI_NUMBER)
    end,
    projectiles = projectile,
    raise_anim = anims.CANNON_RAISE,
    lower_anim = anims.CANNON_LOWER,
    afterimages = afterimages,
}

function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
