--!strict
-- EXE3 Wind (0x08F) and Fan (0x090):
-- Wind spawns a blowing fan in front of the user that pushes the opponent back and blows away barriers/auras;
-- Fan spawns a suction fan that pulls the opponent forward!

local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local element = require("@exelib/element")
local regions = require("@exelib/regions")

local FAN_SPRITE = asset.sprite("air-shooter")
local WIND_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 10 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local WIND_SOUND = asset.sound("spreader")
local APPEAR_SOUND = asset.sound("appear")

local WIND_ELEMENT = element.byte("null", { "wind" })

type FanState = { pulls: boolean, life: number, tick: number }

local FAN_KIND = new.kind {
    id = "wind/fan-object",
    pool = "attack",
    state = { pulls = "bool", life = "u16", tick = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(FAN_SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local s = me.state :: FanState
            s.life = 480
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: FanState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick % 24 == 0 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and not opp:status("dead") then
                    -- Wind blows away barriers/auras via secondary wind element hitbox (0 damage).
                    if not s.pulls then
                        battle.hitbox(me, {
                            panel_x = opp.panel_x,
                            panel_y = opp.panel_y,
                            element = WIND_ELEMENT,
                            z = 0x10_0000,
                            region = regions.single,
                            hit_effect = nil,
                            target = collision.hits_navis,
                            self_type = collision.piercing,
                            damage = 0,
                            hit_mod = 0,
                        })
                    end
                    -- Push back (Wind) or pull forward (Fan) if target panel is valid & owned by opponent.
                    local step_dx = if s.pulls then -me.facing else me.facing
                    local nx = opp.panel_x + step_dx
                    local ny = opp.panel_y
                    if field.valid(nx, ny) then
                        local p = field.panel(nx, ny)
                        if p and p.alliance == opp.alliance and p.kind ~= "missing" and p.kind ~= "broken" then
                            if opp:can_move() then
                                opp.panel_x = nx
                                opp:set_coordinates_from_panel()
                                opp:update_collision_panels()
                                battle.effect(opp.pos, WIND_FX)
                                battle.play_sound(WIND_SOUND)
                            end
                        end
                    end
                end
            end
            s.tick += 1
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type CtrlState = { pulls: boolean }

local WIND_CTRL = new.kind {
    id = "wind/controller",
    pool = "effect",
    state = { pulls = "bool" },
    update = dimming_chips.phases {
        name = "Wind",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                local s = me.state :: CtrlState
                if user then
                    local tx = user.panel_x + user.facing
                    local ty = user.panel_y
                    if field.valid(tx, ty) then
                        local o = battle.spawn(FAN_KIND)
                        if o then
                            o.panel_x, o.panel_y = tx, ty
                            o.alliance, o.flip = user.alliance, user.flip
                            ;(o.state :: FanState).pulls = s.pulls
                        end
                    end
                end
                me.timer = 20
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

local function wind_hook(pulls: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(WIND_CTRL, user, spec)
        if o then
            local st = o.state :: CtrlState
            st.pulls = pulls
        end
        return o
    end
end

local wind: Chip = {
    codes = { "A", "E", "H", "O", "S", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = wind_hook(false),
}

local fan: Chip = {
    codes = { "A", "G", "L", "R", "T", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = wind_hook(true),
}

return {
    wind = wind,
    fan = fan,
}
