--!strict
-- EXE3 Yo-Yo, Boomerang & Lance chips:
-- Yo-Yo1 (0x045), Yo-Yo2 (0x046), Yo-Yo3 (0x047),
-- Boomer1 (0x054), Boomer2 (0x055), Boomer3 (0x056), Lance (0x04B).

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local HIT_SPARKS = asset.sprite("hit-sparks")
local YOYO_SPRITE = asset.sprite("yoyo")
local BOOMER_SPRITE = asset.sprite("boomer")
local LANCE_SPRITE = asset.sprite("lance")
local SPIN_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 12 }
local LANCE_FX = new.effect { sprite = LANCE_SPRITE, anim = 0, palette = 0, steps = 16 }
local SHOT_SOUND = asset.sound("buster-shot")
local SWING_SOUND = asset.sound("sword-swing")
local THROW_SOUND = asset.sound("minibomb-throw")

-- =============================================================================
-- Yo-Yo1 / Yo-Yo2 / Yo-Yo3:
-- Travels 3 panels forward, spins for 3 multi-hits at the tip, then returns!
-- =============================================================================
type YoYoState = { step: number, tick: number, origin_x: number, hits_left: number }

local YOYO_KIND = new.kind {
    id = "yoyo/spinner",
    pool = "attack",
    state = { step = "u8", tick = "u8", origin_x = "u8", hits_left = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(YOYO_SPRITE)
            sprite.shadow = "with_sprite"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: YoYoState
            s.step = 0
            s.tick = 0
            s.hits_left = 3
            battle.play_sound(THROW_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: YoYoState
            -- Phase 0: advance up to 3 panels ahead
            if s.step < 3 then
                if s.tick == 0 then
                    battle.hitbox(me, {
                        panel_x = me.panel_x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 1,
                    })
                    local p = me.pos
                    battle.effect(Vec3.new(p.x, p.y, 0x10_0000), SPIN_FX)
                end
                s.tick += 1
                if s.tick >= 4 then
                    s.tick = 0
                    s.step += 1
                    if s.step < 3 then
                        local nx = me.panel_x + me.facing
                        if field.valid(nx, me.panel_y) then
                            me.panel_x = nx
                            me:set_coordinates_from_panel()
                        else
                            s.step = 3
                        end
                    end
                end
            -- Phase 1: spin at tip dealing remaining hits
            elseif s.step == 3 then
                if s.tick % 6 == 0 and s.hits_left > 0 then
                    s.hits_left -= 1
                    battle.hitbox(me, {
                        panel_x = me.panel_x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = if s.hits_left == 0 then 3 else 1,
                    })
                    local p = me.pos
                    battle.effect(Vec3.new(p.x, p.y, 0x10_0000), SPIN_FX)
                    battle.play_sound(SHOT_SOUND)
                end
                s.tick += 1
                if s.hits_left <= 0 and s.tick >= 18 then
                    s.step = 4
                    s.tick = 0
                end
            -- Phase 2: return toward origin_x
            else
                s.tick += 1
                if s.tick >= 4 then
                    s.tick = 0
                    if me.panel_x == s.origin_x then
                        me:set_lifecycle("destroy")
                        return
                    end
                    local nx = me.panel_x - me.facing
                    if not field.valid(nx, me.panel_y) then
                        me:set_lifecycle("destroy")
                        return
                    end
                    me.panel_x = nx
                    me:set_coordinates_from_panel()
                    battle.hitbox(me, {
                        panel_x = me.panel_x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 1,
                    })
                    if nx == s.origin_x then
                        me:set_lifecycle("destroy")
                        return
                    end
                end
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type ThrowActState = { timer: number }

local function make_yoyo_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ThrowActState)
            if me.step_init == 0 then
                me:set_animation(anims.THROW_LOW)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 5 then
                local sx = me.panel_x + me.facing
                if field.valid(sx, me.panel_y) then
                    local o = battle.spawn(YOYO_KIND)
                    if o then
                        o.panel_x, o.panel_y = sx, me.panel_y
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 0
                        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(damage)
                        ;(o.state :: YoYoState).origin_x = me.panel_x
                    end
                end
            end
            s.timer += 1
            if s.timer >= 20 then
                me:exit_attack()
            end
        end,
    }
end

-- =============================================================================
-- Boomer1 / Boomer2 / Boomer3:
-- Wood boomerang sweeping the outer perimeter of the battlefield!
-- =============================================================================
local PERIMETER_SIDE0: { { number } } = {
    { 1, 3 }, { 2, 3 }, { 3, 3 }, { 4, 3 }, { 5, 3 }, { 6, 3 },
    { 6, 2 }, { 6, 1 },
    { 5, 1 }, { 4, 1 }, { 3, 1 }, { 2, 1 }, { 1, 1 },
    { 1, 2 },
}

type BoomerState = { idx: number, tick: number, step_ticks: number }

local BOOMER_KIND = new.kind {
    id = "yoyo/boomerang",
    pool = "attack",
    state = { idx = "u8", tick = "u8", step_ticks = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(BOOMER_SPRITE)
            sprite.shadow = "with_sprite"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 1, 0xFF
            local s = me.state :: BoomerState
            s.idx = 1
            s.tick = 0
            local pt = PERIMETER_SIDE0[1]
            me.panel_x = if me.alliance == 0 then pt[1] else (7 - pt[1])
            me.panel_y = pt[2]
            me:set_coordinates_from_panel()
            battle.play_sound(THROW_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: BoomerState
            if s.tick == 0 then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 4, -- wood
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.wood,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, 0x10_0000), WOOD_FX)
                if s.idx % 3 == 1 then
                    battle.play_sound(SWING_SOUND)
                end
            end
            s.tick += 1
            if s.tick >= s.step_ticks then
                s.tick = 0
                s.idx += 1
                if s.idx > #PERIMETER_SIDE0 then
                    me:set_lifecycle("destroy")
                    return
                end
                local pt = PERIMETER_SIDE0[s.idx]
                me.panel_x = if me.alliance == 0 then pt[1] else (7 - pt[1])
                me.panel_y = pt[2]
                me:set_coordinates_from_panel()
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local function make_boomer_action(id: string, step_ticks: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ThrowActState)
            if me.step_init == 0 then
                me:set_animation(anims.THROW_LOW)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 5 then
                local o = battle.spawn(BOOMER_KIND)
                if o then
                    o.alliance, o.flip = me.alliance, me.flip
                    o.element = 4
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    o:set_damage_word(damage)
                    ;(o.state :: BoomerState).step_ticks = step_ticks
                end
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

-- =============================================================================
-- Lance: Bamboo lance thrusting forward from the enemy's back column!
-- =============================================================================
local LANCE_ACTION = new.action {
    id = "lance/action",
    state = { timer = "u16" },
    update = function(me: Object, s: ThrowActState)
        if me.step_init == 0 then
            me:set_animation(anims.buster_shoot(me))
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == 6 then
            local back_col = if me.alliance == 0 then 6 else 1
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            battle.hitbox(me, {
                panel_x = back_col,
                panel_y = 2,
                element = 4, -- wood
                z = 0x10_0000,
                region = regions.wide,
                hit_effect = sparks.wood,
                target = collision.hits_navis,
                self_type = collision.piercing,
                damage = damage,
                hit_mod = 3,
            })
            battle.region_effects(back_col, 2, regions.wide, me.alliance, LANCE_FX, 0)
            battle.play_sound(SWING_SOUND)
            -- Pull hit opponent 1 panel forward from back column!
            local opp_side = bit32.bxor(me.alliance, 1)
            for _, opp in battle.alive_actors(opp_side) do
                if opp.panel_x == back_col and not opp:status("dead") and not opp:status("super_armor") then
                    local nx = back_col - me.facing
                    if field.valid(nx, opp.panel_y) then
                        local p = field.panel(nx, opp.panel_y)
                        if p and p.alliance == opp.alliance and p.kind ~= "missing" and p.kind ~= "broken" and opp:can_move() then
                            opp.panel_x = nx
                            opp:set_coordinates_from_panel()
                            opp:update_collision_panels()
                        end
                    end
                end
            end
        end
        s.timer += 1
        if s.timer >= 18 then
            me:exit_attack()
        end
    end,
}

local yoyo1: Chip = {
    action = make_yoyo_action("yo-yo1/action"),
    codes = { "C", "D", "E", "F", "G", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 72,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local yoyo2: Chip = {
    action = make_yoyo_action("yo-yo2/action"),
    codes = { "H", "I", "J", "K", "L" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 80,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local yoyo3: Chip = {
    action = make_yoyo_action("yo-yo3/action"),
    codes = { "M", "N", "O", "P", "Q" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 88,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local boomer1: Chip = {
    action = make_boomer_action("boomer1/action", 5),
    codes = { "F", "H", "J", "M", "T", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 15,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local boomer2: Chip = {
    action = make_boomer_action("boomer2/action", 4),
    codes = { "E", "I", "K", "N", "V", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local boomer3: Chip = {
    action = make_boomer_action("boomer3/action", 3),
    codes = { "L", "O", "P", "U", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local lance: Chip = {
    action = LANCE_ACTION,
    codes = { "D", "E", "H", "R", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 46,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    ["yo-yo1"] = yoyo1,
    ["yo-yo2"] = yoyo2,
    ["yo-yo3"] = yoyo3,
    boomer1 = boomer1,
    boomer2 = boomer2,
    boomer3 = boomer3,
    lance = lance,
}
