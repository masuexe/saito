--!strict
-- Firing EXE3's projectiles and computing buster recovery.

local common = require("@exelib/projectile/projectile")
local recovery_rows = require("../rules/buster").recovery
local shot = require("../objects/projectile")

local projectile = {}

local CLOSED = 0x0F880080

function projectile.recovery(navi: Object, x: number, y: number, rapid: number): number
    local open = 0
    while true do
        local f = field.flags(x, y)
        if f == 0 or bit32.btest(f, CLOSED) then
            break
        end
        open += 1
        x += navi.facing
    end
    local row = recovery_rows[math.min(rapid, #recovery_rows - 1) + 1]
    return row[math.min(open, 5) + 1]
end

function projectile.spawn(owner: Object, x: number, y: number, s: ProjectileShot): Object?
    return common.spawn(shot.kind, owner, x, y, s)
end

function projectile.fire(navi: Object, s: ProjectileShot): number
    local x, y = navi.panel_x + navi.facing, navi.panel_y
    common.spawn(shot.kind, navi, x, y, s)
    return projectile.recovery(navi, x, y, battle.navi(navi.alliance).rapid)
end

return projectile
