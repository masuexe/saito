--!strict
-- EXE3 Trap chips: AntiNavi (0x0BE), AntiDmg (0x0BF), AntiSwrd (0x0C0), AntiRecv (0x0C1).
-- Armed cleanly in EXE3 rules state so `rules/init.luau` handles all 4 trap counters directly.

local api = require("../rules/api")
local dimming_chips = require("@exelib/dimming")

local TRAP_SOUND = asset.sound("trap-mark")

type TrapKind = "none" | "anti_damage" | "anti_sword" | "anti_navi" | "anti_recovery"

type TrapState = {
    trap_kind: TrapKind,
}

local TRAP_KIND = new.kind {
    id = "anti/controller",
    pool = "effect",
    state = { trap_kind = { "none", "anti_damage", "anti_sword", "anti_navi", "anti_recovery" } },
    update = dimming_chips.phases {
        name = "Trap",
        hidden = true,
        effect = function(me: Object)
            if me.phase_init == 0 then
                local s = me.state :: TrapState
                api.arm_trap(me.alliance, s.trap_kind, me.damage)
                battle.play_sound(TRAP_SOUND)
                me.timer = 24
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function trap_hook(kind: TrapKind): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(TRAP_KIND, user, spec, true)
        if o then
            local s = o.state :: TrapState
            s.trap_kind = kind
        end
        return o
    end
end

local antinavi: Chip = {
    codes = { "A", "M", "N", "V", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = trap_hook("anti_navi"),
}

local antidmg: Chip = {
    codes = { "C", "F", "H", "M", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 45,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = trap_hook("anti_damage"),
}

local antiswrd: Chip = {
    codes = { "B", "K", "R", "U", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 68,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = trap_hook("anti_sword"),
}

local antirecv: Chip = {
    codes = { "B", "D", "E", "O", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 73,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = trap_hook("anti_recovery"),
}

return {
    antinavi = antinavi,
    antidmg = antidmg,
    antiswrd = antiswrd,
    antirecv = antirecv,
}
