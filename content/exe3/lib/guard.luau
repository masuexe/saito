--!strict
-- EXE3's B+Left guard action (Shield Style, Shield, Reflect).

local anims = require("./anims")
local shield = require("../objects/shield")

local guard = {}

export type Counter = (me: Object, x: number, y: number, damage: number) -> ()

export type Spec = {
    id: string,
    ticks: number,
    shield: ReflectorShieldLook,
    counter: Counter?,
}

export type State = {
    timer: number,
}

local SHIELD_X, SHIELD_Z = 16, 0
local DROPS = 0x2

local function drop_shield(me: Object)
    me.related1 = nil
    me:set_status("guard", false)
end

function guard.action(spec: Spec): Action
    local counter = spec.counter

    local function guarding(me: Object, s: State)
        if me.step_init == 0 then
            shield.spawn(me, Vec3.px(SHIELD_X * me.facing, 0, SHIELD_Z), spec.shield)
            me:set_status("guard", true)
            me:set_status("using_action", true)
            me.anim = anims.idle(me)
            me.marker = 0
            me.step_init = 4
            s.timer = 0
            return
        end
        local guarded = me.collision.guard_dirs
        if counter and guarded ~= 0 then
            if bit32.btest(guarded, DROPS) then
                drop_shield(me)
            elseif me.marker == 0 then
                me.marker = 1
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                counter(me, me.panel_x + me.facing, me.panel_y, damage)
            end
        end
        local t = s.timer + 1
        s.timer = t
        if t > spec.ticks then
            drop_shield(me)
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16" },
        args = spec,
        update = function(me: Object, s: State)
            if me.step ~= 0 then
                error(string.format("guard phase %#x out of range", me.step))
            end
            guarding(me, s)
        end,
    }
end

return guard
