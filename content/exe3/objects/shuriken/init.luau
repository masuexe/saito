--!strict
-- EXE3 diving shuriken attack object (used by AntiDmg B+Left, AntiDmg chip trap, and BodyGrd PA).

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local shuriken = {}

local SPRITE = asset.sprite("shuriken")
local PUFF = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local LAND_SOUND = asset.sound("panel-crack")
local THROW_SOUND = asset.sound("sword-swing")

local START_OFFSET = 0x90_0000
local STEP = 0x0C_0000
local DIVE_TICKS = 12

type State = {
    hit_mod: number,
}

local function init(me: Object)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 1, 0xFF
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(int.i32(cx - START_OFFSET * me.facing), cy, START_OFFSET)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(collision.thrown_slash, collision.hits_navis, s.hit_mod)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:set_region(nil)
    me.collision:present()
    battle.play_sound(THROW_SOUND)
    me.timer = DIVE_TICKS
    me.action = 0
    me:set_lifecycle("update")
end

local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.action == 0 then
        field.highlight(me.panel_x, me.panel_y)
        if me.timer == 6 then
            battle.effect(me.pos, PUFF)
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            local p = me.pos
            me.pos = Vec3.new(int.i32(p.x + STEP * me.facing), p.y, math.max(0, int.i32(p.z - STEP)))
        else
            local cx, cy = field.center(me.panel_x, me.panel_y)
            me.pos = Vec3.new(cx, cy, 0)
            c:set_region(regions.single)
            battle.play_sound(LAND_SOUND)
            battle.effect(Vec3.new(cx, cy, 0x10_0000), PUFF)
            me.action = 4
            me.timer = 6
        end
    else
        if me.timer == 4 then
            c:set_region(nil)
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            c:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        me.visible = bit32.btest(t, 1)
    end
    me:update_sprite()
    c:present()
end

shuriken.kind = new.kind {
    id = "shuriken",
    pool = "attack",
    state = { hit_mod = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

function shuriken.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    local o = battle.spawn(shuriken.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance, o.flip = owner.alliance, owner.flip
    o:set_damage_word(damage)
    ;(o.state :: State).hit_mod = hit_mod
    return o
end

-- Helper: pick an alive opponent actor panel (defaults to opponent's center panel if none found).
function shuriken.target_enemy(owner: Object): (number?, number?)
    local opp = bit32.bxor(owner.alliance, 1)
    local actors = battle.alive_actors(opp)
    if #actors == 0 then
        return nil, nil
    end
    local pick = actors[battle.rng_positive() % #actors + 1]
    return pick.panel_x, pick.panel_y
end

return shuriken
