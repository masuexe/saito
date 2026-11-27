--!strict
-- EXE3's barrier visual following a navi's active barrier/aura.

local common = require("@exelib/barriers/visual")

local visual = {}

local made = common.make {
    id = "barrier-visual",
    blows = false,
    hides_off_field = false,
    offset = function(_owner: Object, _b: Barrier): (number, number)
        return 0, 0
    end,
    stops_running_paused_in_fight = true,
    steps_paused = true,
    side_only = true,
}

visual.kind = made.kind
visual.spawn = made.spawn

return visual
