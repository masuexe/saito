--!strict
-- EXE3's Custom Screen ADD button (slot 11) and hand size computation (base Custom level + ADD bonus, capped at 10).
-- Uses Track B's Gen-1 Custom Screen ADD APIs: `custom.start_add`, `custom.cancel_add`, `custom.is_adding`,
-- `custom.add_bonus`.

local add = {}

local MAX_HAND_SIZE = 10

-- Effective hand size when the Custom Screen opens: NaviStats custom_level (5 base + Custom Style + Custom1/2,
-- capped at 8 before ADD) plus persistent round ADD bonus, capped at 10.
function add.hand_size(side: number): number?
    local stats = battle.navi(side)
    local base = math.min(stats.custom_level, 8)
    local bonus = custom.add_bonus(side)
    return math.min(base + bonus, MAX_HAND_SIZE)
end

-- Slot 11 ADD button definition.
add.buttons = {
    add = {
        slot = 11,
        shown = function(_side: number): boolean
            return true
        end,
        state = function(side: number): "selectable" | "unavailable"
            local stats = battle.navi(side)
            local base = math.min(stats.custom_level, 8)
            local total = base + custom.add_bonus(side)
            if total >= MAX_HAND_SIZE or #custom.picks(side) == 0 then
                return "unavailable"
            end
            return "selectable"
        end,
        pressed = function(side: number)
            local stats = battle.navi(side)
            local base = math.min(stats.custom_level, 8)
            if base + custom.add_bonus(side) >= MAX_HAND_SIZE or #custom.picks(side) == 0 then
                custom.refuse(side)
                return
            end
            custom.confirm_add(side)
        end,
    } :: ButtonSpec,
}

return add
