--!strict
-- EXE3 Stealth chips: Mole1 (0x0A2), Mole2 (0x0A3), Mole3 (0x0A4), Shadow (0x0A5).
-- Mole dives underground (`submerged` + `invisible`, catchable by Rush); Shadow grants immunity to non-sword attacks!

local api = require("../rules/api")
local dimming_chips = require("@exelib/dimming")

local INVIS_SOUND = asset.sound("invisible")
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }

type StealthState = { ticks: number, mode: string }

local STEALTH_KIND = new.kind {
    id = "stealth/controller",
    pool = "effect",
    state = { ticks = "u16", mode = { "mole", "shadow" } },
    update = dimming_chips.phases {
        name = "Stealth",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                local s = me.state :: StealthState
                if user then
                    if s.mode == "mole" then
                        -- A6BE 0x080F8D80: Mole sets timed submerged status only (480/720/960t).
                        user:set_status_timer("submerged", s.ticks)
                        user:set_status("submerged", true)
                    else
                        -- A6BE 0x080FA4F8: Shadow blocks non-sword hits via rules hit_guard;
                        -- do NOT set flash (which would block swords under flash_timer_first).
                        api.set_shadow_ticks(user.alliance, s.ticks)
                    end
                    battle.effect(user.pos, PUFF_FX)
                end
                battle.play_sound(INVIS_SOUND)
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

local function hook(mode: "mole" | "shadow", ticks: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(STEALTH_KIND, user, spec, true)
        if o then
            local s = o.state :: StealthState
            s.mode = mode
            s.ticks = ticks
        end
        return o
    end
end

local mole1: Chip = {
    codes = { "A", "D", "H", "J", "O", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = hook("mole", 480),
}

local mole2: Chip = {
    codes = { "B", "G", "I", "K", "R", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = hook("mole", 720),
}

local mole3: Chip = {
    codes = { "C", "E", "L", "M", "Q" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = hook("mole", 960),
}

local shadow: Chip = {
    codes = { "H", "J", "N", "Q", "U", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = hook("shadow", 360),
}

return {
    mole1 = mole1,
    mole2 = mole2,
    mole3 = mole3,
    shadow = shadow,
}
