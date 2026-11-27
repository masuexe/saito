--!strict
-- EXE3 spreaders action (ShotGun, V-Gun, SideGun, Spreader) over @exelib/spreadr/action.

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/spreadr/action")

local spreader = {}

local NAVI_NUMBER = 0

function spreader.hold(me: Object, look: AttachmentLook, which: Slot)
    attachment.attach(me, look, which, 2 * NAVI_NUMBER)
end

local LOOK: SpreaderLook = {
    sound = asset.sound("spreader"),
    hold = spreader.hold,
    slot = "related",
    bullet_kind = bullet.kind,
    anim = 14,
}

function spreader.action(spec: { id: string, gun: AttachmentLook, delay: number, z: number, shot: BulletVariant }): Action
    return common.action({
        id = spec.id,
        gun = spec.gun,
        fire_after = spec.delay,
        z = spec.z * 0x10000,
        bullet = spec.shot,
    }, LOOK)
end

return spreader
