--!strict
-- EXE3 Recovery action over @exelib/recov/heal (no mood gain in Gen-1).

local heal = require("@exelib/recov/heal")

local recov = {}

function recov.action(spec: { id: string, hp: number }): Action
    return heal.action { id = spec.id, hp = spec.hp, mood = 0, mood_after_trap = false }
end

return recov
