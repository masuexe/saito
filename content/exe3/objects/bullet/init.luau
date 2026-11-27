--!strict
-- EXE3's probing bullet (ShotGun, V-Gun, SideGun, Spreader).

local collision = require("../rules/collision")
local common = require("@exelib/bullet")

local bullet = {}

bullet.kind = common.make_kind("bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
    single_unturned = true,
})

bullet.variant = common.variant

function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number): Object?
    return common.fire(bullet.kind, owner, x, y, z, variant, element, damage)
end

return bullet
