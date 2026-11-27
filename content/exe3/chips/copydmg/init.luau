--!strict
-- EXE3 CopyDmg (0x0C2): marks the opponent so damage taken is copied!

local collision = require("../rules/collision")
local common_mark = require("@exelib/copydmg/mark")

local mark = common_mark.make {
    id = "copydmg/mark",
    sprite = asset.sprite("hit-sparks"),
    anim = 7,
    sound = asset.sound("trap-mark"),
    is = collision.attack,
    hits = collision.hits_navis,
    ticks = 120,
    over_navi = "position",
    reload_anim = true,
    own_gauges_hit = nil,
}

local SOUND = asset.sound("buster-shot")
local SHOOT, RECOVER = 0x0, 0x4

type State = { timer: number }

local copydmg: Chip = {
    action = new.action {
        id = "copydmg/action",
        state = { timer = "u16" },
        update = function(me: Object, s: State)
            if me.step == SHOOT then
                if me.step_init == 0 then
                    me:set_animation(0x12)
                    me:set_status("using_action", true)
                    me.step_init = 4
                    s.timer = 0
                end
                if s.timer == 1 then
                    battle.play_sound(SOUND)
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
                    mark.spawn(me, me.panel_x + me.facing, me.panel_y, 0, damage)
                end
                s.timer += 1
                if s.timer > 4 then
                    me.step, me.step_init = RECOVER, 0
                end
            else
                if me.step_init == 0 then
                    s.timer = 10
                    me.step_init = 4
                end
                local t = s.timer - 1
                s.timer = int.u16(t)
                if t < 0 then
                    me.related1 = nil
                    me:exit_attack()
                end
            end
        end,
    },
    codes = { "A", "F", "H", "L", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { copydmg = copydmg }
