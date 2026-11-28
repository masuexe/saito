--!strict
-- EXE3 ZapRing, Rope, Tornado & Burner chips:
-- ZapRing1-3 (0x042..0x044), Rope1-3 (0x04E..0x050), Tornado (0x041), Burner (0x036).

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local HIT_SPARKS = asset.sprite("hit-sparks")
local TORNADO_SPRITE = asset.sprite("tornado")
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 12 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 12 }
local FIRE_FX = new.effect { sprite = asset.sprite("flame"), anim = 0, palette = 0, steps = 18 }
local WIND_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local SHOT_SOUND = asset.sound("buster-shot")
local WIND_SOUND = asset.sound("spreader")
local BLAST_SOUND = asset.sound("hit-bomb")
local SWING_SOUND = asset.sound("sword-swing")

local WIND_ELEM = element.byte("null", { "wind" })

-- =============================================================================
-- ZapRing1 / ZapRing2 / ZapRing3:
-- Travelling electric ring projectile that deals Elec damage and paralyzes!
-- =============================================================================
type RingState = { tick: number, step_ticks: number, elem: number }

local RING_KIND = new.kind {
    id = "zapring/ring",
    pool = "attack",
    state = { tick = "u8", step_ticks = "u8", elem = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local s = me.state :: RingState
            local sprite = me.sprite
            sprite:load(HIT_SPARKS)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = if s.elem == 4 then 5 else 4, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0x10_0000)
            s.tick = 0
            battle.play_sound(SHOT_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: RingState
            if s.tick == 0 then
                local hit = false
                local opp_side = bit32.bxor(me.alliance, 1)
                for _, opp in battle.alive_actors(opp_side) do
                    if opp.panel_x == me.panel_x and opp.panel_y == me.panel_y and not opp:status("dead") then
                        hit = true
                        break
                    end
                end
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = s.elem,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = if s.elem == 4 then sparks.wood else sparks.elec,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                    status = statuses.paralyze,
                })
                battle.effect(me.pos, if s.elem == 4 then WOOD_FX else ELEC_FX)
                if hit then
                    me:set_lifecycle("destroy")
                    return
                end
            end
            s.tick += 1
            if s.tick >= s.step_ticks then
                s.tick = 0
                local nx = me.panel_x + me.facing
                if not field.valid(nx, me.panel_y) then
                    me:set_lifecycle("destroy")
                    return
                end
                me.panel_x = nx
                me:set_coordinates_from_panel()
                local p = me.pos
                me.pos = Vec3.new(p.x, p.y, 0x10_0000)
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type ActTimer = { timer: number }

local function make_zapring_action(id: string, step_ticks: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 5 then
                local sx = me.panel_x + me.facing
                if field.valid(sx, me.panel_y) then
                    local o = battle.spawn(RING_KIND)
                    if o then
                        o.panel_x, o.panel_y = sx, me.panel_y
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 3 -- elec
                        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(damage)
                        local rs = o.state :: RingState
                        rs.step_ticks = step_ticks
                        rs.elem = 3
                    end
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
-- Rope1 / Rope2 / Rope3:
-- Wood binding rope lashing 2 panels ahead, immobilizing & paralyzing targets!
-- =============================================================================
local function make_rope_action(id: string, reach: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 6 then
                local sx = me.panel_x + me.facing
                if field.valid(sx, me.panel_y) then
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    local region = if reach >= 3 then regions.line else regions.long
                    battle.hitbox(me, {
                        panel_x = sx,
                        panel_y = me.panel_y,
                        element = 4, -- wood
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.wood,
                        target = collision.hits_navis,
                        self_type = collision.attack,
                        damage = damage,
                        hit_mod = 3,
                        status = statuses.paralyze,
                    })
                    battle.region_effects(sx, me.panel_y, region, me.alliance, WOOD_FX, 0x10_0000)
                    battle.play_sound(SWING_SOUND)
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
-- Tornado: 8-hit wind twister 2 panels ahead that strips barriers & hits rapidly!
-- =============================================================================
type TornadoState = { hits: number, tick: number }

local TORNADO_KIND = new.kind {
    id = "zapring/tornado",
    pool = "attack",
    state = { hits = "u8", tick = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(TORNADO_SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: TornadoState
            s.hits = 0
            s.tick = 0
            battle.play_sound(WIND_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: TornadoState
            if s.tick % 4 == 0 and s.hits < 8 then
                s.hits += 1
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = WIND_ELEM,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.plain,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == 8 then 3 else 1,
                })
                battle.effect(me.pos, WIND_FX)
                if s.hits % 2 == 1 then
                    battle.play_sound(WIND_SOUND)
                end
            end
            s.tick += 1
            if s.hits >= 8 and s.tick >= 34 then
                me:set_lifecycle("destroy")
                return
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local TORNADO_ACTION = new.action {
    id = "tornado/action",
    state = { timer = "u16" },
    update = function(me: Object, s: ActTimer)
        if me.step_init == 0 then
            me:set_animation(anims.buster_shoot(me))
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == 5 then
            local tx = me.panel_x + me.facing * 2
            if field.valid(tx, me.panel_y) then
                local o = battle.spawn(TORNADO_KIND)
                if o then
                    o.panel_x, o.panel_y = tx, me.panel_y
                    o.alliance, o.flip = me.alliance, me.flip
                    o.element = WIND_ELEM
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    o:set_damage_word(damage)
                end
            end
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

-- =============================================================================
-- Burner: Fire ring igniting all 8 surrounding panels!
-- =============================================================================
local BURNER_ACTION = new.action {
    id = "burner/action",
    state = { timer = "u16" },
    update = function(me: Object, s: ActTimer)
        if me.step_init == 0 then
            me:set_animation(anims.buster_shoot(me))
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == 6 then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = 1, -- fire
                z = 0x10_0000,
                region = regions.ring,
                hit_effect = sparks.fire,
                target = collision.hits_navis,
                self_type = collision.piercing,
                damage = damage,
                hit_mod = 3,
            })
            battle.region_effects(me.panel_x, me.panel_y, regions.ring, me.alliance, FIRE_FX, 0x10_0000)
            battle.play_sound(BLAST_SOUND)
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local zapring1: Chip = {
    action = make_zapring_action("zapring1/action", 5),
    codes = { "A", "M", "P", "Q", "S", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local zapring2: Chip = {
    action = make_zapring_action("zapring2/action", 4),
    codes = { "B", "G", "N", "R", "W" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local zapring3: Chip = {
    action = make_zapring_action("zapring3/action", 3),
    codes = { "C", "E", "O", "T", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rope1: Chip = {
    action = make_rope_action("rope1/action", 2),
    codes = { "D", "G", "J", "M", "O", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 45,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rope2: Chip = {
    action = make_rope_action("rope2/action", 2),
    codes = { "E", "G", "O", "T", "U" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 55,
    damage = 70,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rope3: Chip = {
    action = make_rope_action("rope3/action", 3),
    codes = { "H", "I", "T", "U", "V" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 65,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local tornado: Chip = {
    action = TORNADO_ACTION,
    codes = { "C", "L", "R", "T", "U" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local burner: Chip = {
    action = BURNER_ACTION,
    codes = { "B", "F", "Q", "S", "W" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 15,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    zapring1 = zapring1,
    zapring2 = zapring2,
    zapring3 = zapring3,
    rope1 = rope1,
    rope2 = rope2,
    rope3 = rope3,
    tornado = tornado,
    burner = burner,
}
