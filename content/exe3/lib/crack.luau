--!strict
-- EXE3's PanlOut1 and PanlOut3 (A6BE family 0x16, `0x080B22DC..0x080B23FC`):
--   Phase 0 (`SWING`, ticks 0..16): MegaMan anim `0x24` + held `0c-07` anim 0 (both 32t);
--     at tick 16 (`0x24` frame 6, flag `0x01`), the pickaxe slams the ground -> hitbox + break panel(s)
--     + `small-puff` (`14-02` anim 1) + `panel-crack` SFX (`0x7B`).
--   Phase 4 (`HOLD`, ticks 16..32): hold slam pose until `0x24` / `0c-07` anim 0 completes (32t total),
--     then switch to MegaMan anim `0x25` + `0c-07` anim 1 (both 5t).
--   Phase 8 (`RETRACT`, ticks 32..37): 5 ticks to pull back and dematerialize pickaxe, then exit.

local anims = require("./anims")
local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local panlout_tool = attachment.look { sprite = asset.sprite("panlout-tool"), palette = 0 }
local BREAK_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 16 }
local BREAK_SOUND = asset.sound("panel-crack")

local crack = {}

crack.ahead = { { 0, 0 } }
crack.column = { { 0, 0 }, { 0, -1 }, { 0, 1 } }

local SWING, HOLD, RETRACT = 0x0, 0x4, 0x8
local STRIKE_TICKS = 16
local HOLD_TICKS = 16
local RETRACT_TICKS = 5

export type CrackSpec = {
    id: string,
    panels: { { number } },
}

type State = { timer: number }

function crack.action(spec: CrackSpec): Action
    local panels = spec.panels
    return new.action {
        id = spec.id,
        state = { timer = "u16" },
        args = { panels = panels },
        update = function(me: Object, s: State)
            local step = me.step
            if step == SWING then
                if me.step_init == 0 then
                    me:set_animation(anims.PANLOUT)
                    me:open_counter_window()
                    attachment.attach(me, panlout_tool, "related", 0)
                    me:set_status("using_action", true)
                    s.timer = 0
                    me.step_init = 4
                end
                s.timer += 1
                if s.timer >= STRIKE_TICKS then
                    local dir = me.facing
                    local base_x, base_y = me.panel_x + dir, me.panel_y
                    local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    local damage = bit32.band(word, 0xFFFF)
                    local stamina = bit32.rshift(word, 16)
                    local broke_any = false
                    for _, p in panels do
                        local px, py = base_x + dir * p[1], base_y + p[2]
                        if field.valid(px, py) then
                            battle.hitbox(me, {
                                panel_x = px,
                                panel_y = py,
                                element = me.attack_element,
                                z = 0,
                                region = regions.single,
                                hit_effect = nil,
                                target = collision.hits_navis,
                                self_type = collision.thrown,
                                damage = damage,
                                stamina = stamina,
                                hit_mod = 0,
                            })
                            if field.break_panel(px, py, nil) then
                                local cx, cy = field.center(px, py)
                                battle.effect(Vec3.new(cx, cy, 0), BREAK_FX, me.alliance)
                                broke_any = true
                            end
                        end
                    end
                    if broke_any then
                        battle.play_sound(BREAK_SOUND)
                    end
                    me.step, me.step_init = HOLD, 0
                end
            elseif step == HOLD then
                if me.step_init == 0 then
                    s.timer = 0
                    me.step_init = 4
                end
                s.timer += 1
                if s.timer >= HOLD_TICKS then
                    me:set_animation(anims.PANLOUT_END)
                    local tool = me.related1
                    if tool then
                        tool.anim = 1
                    end
                    me.step, me.step_init = RETRACT, 0
                end
            elseif step == RETRACT then
                if me.step_init == 0 then
                    s.timer = 0
                    me.step_init = 4
                end
                s.timer += 1
                if s.timer >= RETRACT_TICKS then
                    me.related1 = nil
                    me:exit_attack()
                end
            else
                me.related1 = nil
                me:exit_attack()
            end
        end,
    }
end

return crack
