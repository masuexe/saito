--!strict
-- EXE3 spreaders action (ShotGun, V-Gun, SideGun, Spreader, Bubbler, HeatShot) over @exelib/spreadr/action.
-- Verified against A6BE (`0x080B14F8..0x080B1554`): MegaMan plays his own style buster shoot animation
-- (`anims.buster_shoot`), attaches NO gun sprite (0c-11 is FlameThrower, not Spreader), and spawns
-- muzzle flash Effect #0 entry 0x1F (`0c-1c` `spreader-flash` anim 0).

local anims = require("../../lib/anims")
local bullet = require("../../objects/bullet")
local common = require("@exelib/spreadr/action")

local spreader = {}

local MUZZLE_FLASH = new.effect { sprite = asset.sprite("spreader-flash"), anim = 0, palette = 0, steps = 7 }

local LOOK: SpreaderLook = {
    sound = asset.sound("cannon"),
    muzzle = MUZZLE_FLASH,
    slot = "related",
    bullet_kind = bullet.kind,
    anim = anims.buster_shoot,
}

function spreader.action(spec: { id: string, delay: number, z: number, shot: BulletVariant }): Action
    return common.action({
        id = spec.id,
        fire_after = spec.delay,
        z = spec.z * 0x10000,
        bullet = spec.shot,
    }, LOOK)
end

return spreader
