--!strict
-- EXE3 Invis (compat/chips.toml ID 0x0A0): non-dimming action turning user invisible for 360 ticks
-- (A6BE Family 17, 0x080B1FB6: b11=0x02, flash=0x168=360t, SFX 0x91, 10t action wait).

local anims = require("../lib/anims")

local INVIS_SOUND = asset.sound("invisible")
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }

type InvisState = { timer: number }

local INVIS_ACTION = new.action {
    id = "invis/action",
    state = { timer = "i16" },
    update = function(me: Object, s: InvisState)
        if me.step == 0 then
            me:set_animation(anims.idle(me))
            me:set_status_timer("flash", 360)
            me:set_status("invisible", true)
            battle.effect(me.pos, PUFF_FX)
            battle.play_sound(INVIS_SOUND)
            s.timer = 10
            me.step = 4
            return
        end
        s.timer -= 1
        if s.timer <= 0 then
            me:exit_attack()
        end
    end,
}

local chip: Chip = {
    action = INVIS_ACTION,
    codes = { "B", "E", "F", "R", "S", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 11,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "when_visible" },
}

return { invis = chip }
