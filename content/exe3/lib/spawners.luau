--!strict
-- EXE3's instant spawner action (used by Atk+10, Atk+30, invalid chip).

local spawners = {}

export type Spawner = {
    id: string,
    spawn: (user: Object, element: number, damage: number) -> (),
    hold: number?,
}

type State = { timer: number }

local SPAWN = 0x0

function spawners.action(spec: Spawner): Action
    return new.action {
        id = spec.id,
        state = { timer = "i16" },
        update = function(me: Object, s: State)
            if me.step == SPAWN then
                me.step = 4
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + bit32.band(me.extra, 0xFF))
                spec.spawn(me, me.attack_element, damage)
                if not spec.hold then
                    me:exit_attack()
                    return
                end
                s.timer = spec.hold
            end
            s.timer -= 1
            if s.timer <= 0 then
                me:exit_attack()
            end
        end,
    }
end

return spawners
