--!strict
-- EXE3's hitscan/linear projectile (buster, charged shots, Cannon, AirShot, Reflect).

local common = require("@exelib/projectile/projectile")

local projectile = {}

projectile.variant = common.variant
-- EXE3 AttackObj #1 (A6BE 0x080D5D5E) advances 1 panel/tick (wait = 0).
projectile.kind = common.make_kind { id = "projectile", wait = 0 }

function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
