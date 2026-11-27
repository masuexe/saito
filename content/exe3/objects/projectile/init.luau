--!strict
-- EXE3's hitscan/linear projectile (buster, charged shots, Cannon, AirShot, Reflect).

local common = require("@exelib/projectile/projectile")

local projectile = {}

projectile.variant = common.variant
projectile.kind = common.make_kind { id = "projectile" }

function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    return common.spawn(projectile.kind, owner, x, y, shot)
end

return projectile
