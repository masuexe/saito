--!strict
-- MegaMan's authentic EXE3 charged shots:
--   - Normal: piercing/plain charged buster shot
--   - Heat (Flamethrower): 3 panels forward multi-hit fire column (melts ice, 2x on grass, does not pierce invis)
--   - Aqua (Bubbler): hits target panel + 1 panel directly behind it on impact (aqua, flinch/flash)
--   - Elec (ZapRing): travelling electric ring that inflicts paralyze on hit
--   - Wood (Twister): strikes panel 2 tiles ahead 8 rapid hits (wood, no mercy invuln until final hit)
--   - Ground (CrackShot): fires shot cracking panels ahead and spreading cracks on impact
-- Honors BrakChrg (`stats.brake_charge`) and WeapLV+1 (`stats.weapon_level`).

local anims = require("../../../lib/anims")
local bullet = require("../../../objects/bullet")
local collision = require("../../../rules/collision")
local projectile = require("../../../lib/projectile")
local regions = require("@exelib/regions")
local shot = require("../../../objects/projectile")
local sparks = require("../../../lib/sparks")
local statuses = require("../../../rules/status")

local charged_shot = {}

local SHOOT, RECOVER = 0x0, 0x4
local SHOT_Z = 0x18_0000
-- EXE3's `pink-flash` (14-04) already bakes a -16px vertical OAM offset (`y_mid = -16.5`,
-- unlike `hit-sparks` 14-05 or EXE4+ `pink-flash` centered at 0). Subtracting 0x10_0000
-- keeps the charged shot burst centered on the target's torso (-24.5px), matching the
-- normal buster spark (`hit-sparks` anim 0 at `SHOT_Z`).
local CHARGED_SHOT_Z = 0x08_0000
local SHOT_SOUND = asset.sound("buster-shot")
local SPREADER_SOUND = asset.sound("spreader")
local CRACK_SOUND = asset.sound("panel-crack")
local CHARGE_TICKS = { 120, 100, 85, 70, 60, 60 }

local HIT_SPARKS = asset.sprite("hit-sparks")
local SMALL_PUFF_SPRITE = asset.sprite("small-puff")
local TORNADO_SPRITE = asset.sprite("tornado")

local FLAME_FX = new.effect { sprite = asset.sprite("flame"), anim = 0, palette = 0, steps = 12 }
local AQUA_FX = new.effect { sprite = asset.sprite("bubble-splash"), anim = 0, palette = 0, steps = 26 }
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 12 }
local WIND_FX = new.effect { sprite = SMALL_PUFF_SPRITE, anim = 1, palette = 0, steps = 12 }
local CRACK_FX = new.effect { sprite = SMALL_PUFF_SPRITE, anim = 1, palette = 0, steps = 12 }

local NULL_SHOT = shot.variant {
    id = "megaman/charged-shot/null",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}

local NULL_BREAK_SHOT = shot.variant {
    id = "megaman/charged-shot/null-break",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.breaking,
}

-- Aqua Bubbler bullet: hits target panel + 1 panel directly behind it (`regions.long`).
local BUBBLER_BULLET = bullet.variant {
    region = regions.long,
    hit_mod = 3,
    effect = AQUA_FX,
    sound = SPREADER_SOUND,
}

-- =============================================================================
-- Heat Style: 3-panel forward Flamethrower column (3 rapid fire ticks)
-- =============================================================================
type FlameState = { ticks: number, burst_count: number, breaking: boolean }

local FLAME_KIND = new.kind {
    id = "megaman/charged-shot/flamethrower",
    pool = "attack",
    state = { ticks = "u16", burst_count = "u8", breaking = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me:set_coordinates_from_panel()
            if not me:create_collision() then
                me:free()
                return
            end
            local s = me.state :: FlameState
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 1)
            me.collision.element = 1 -- fire
            me.collision:set_hit_effect(sparks.fire)
            me.collision:set_region(nil)
            me.collision:present()
            s.ticks = 0
            s.burst_count = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            me:hit_spark()
            if battle.over() then
                c:set_region(nil)
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: FlameState
            if s.ticks % 6 == 0 and s.burst_count < 3 then
                s.burst_count += 1
                local hit_mod = if s.burst_count == 3 then 3 else 1
                local is_col = if s.breaking then collision.breaking else collision.attack
                me:reset_collision_types(is_col, collision.hits_navis, hit_mod)
                c.element = 1
                c:set_hit_effect(sparks.fire)
                c:set_region(regions.line)
                battle.play_sound(SPREADER_SOUND)
                -- Melt any ice panels along the 3-panel flame column & show fire effects.
                for d = 0, 2 do
                    local px = me.panel_x + me.facing * d
                    local py = me.panel_y
                    if field.valid(px, py) then
                        local p = field.panel(px, py)
                        if p and p.kind == "ice" then
                            field.set_type(px, py, "normal")
                        end
                        local cx, cy = field.center(px, py)
                        battle.effect(Vec3.new(cx, cy, SHOT_Z), FLAME_FX)
                    end
                end
            elseif s.ticks % 6 == 2 then
                c:set_region(nil)
            end
            s.ticks += 1
            if s.ticks >= 20 then
                c:set_region(nil)
                me:set_lifecycle("destroy")
                return
            end
            c:present()
        else
            me:destroy()
        end
    end,
}

local function spawn_flamethrower(owner: Object, total_damage: number, breaking: boolean)
    local x = owner.panel_x + owner.facing
    if not field.valid(x, owner.panel_y) then
        return
    end
    local o = battle.spawn(FLAME_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, owner.panel_y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 1
    local per_hit = math.max(10, math.floor(total_damage / 3))
    o:set_damage_word(per_hit)
    ;(o.state :: FlameState).breaking = breaking
end

-- =============================================================================
-- Elec Style: Travelling ZapRing (paralyzes target on hit!)
-- =============================================================================
type ZapState = { step_ticks: number, breaking: boolean }

local ZAPRING_KIND = new.kind {
    id = "megaman/charged-shot/zapring",
    pool = "attack",
    state = { step_ticks = "u8", breaking = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(HIT_SPARKS)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 4, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, SHOT_Z)
            if not me:create_collision() then
                me:free()
                return
            end
            local s = me.state :: ZapState
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 3)
            local c = me.collision
            c.element = 3 -- elec
            c:set_hit_effect(sparks.elec)
            c.status_base = statuses.paralyze
            c:set_region(regions.single)
            c:present()
            s.step_ticks = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            me:hit_spark()
            if battle.over() or c.hit_flags ~= 0 then
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: ZapState
            s.step_ticks += 1
            if s.step_ticks >= 4 then
                s.step_ticks = 0
                me.panel_x += me.facing
                if not field.valid(me.panel_x, me.panel_y) then
                    c:set_region(nil)
                    me.visible = false
                    me:set_lifecycle("destroy")
                    return
                end
                me:set_coordinates_from_panel()
                local p = me.pos
                me.pos = Vec3.new(p.x, p.y, SHOT_Z)
                me:update_collision_panels()
                battle.effect(me.pos, ELEC_FX)
            end
            me:update_sprite()
            c:present()
        else
            me:destroy()
        end
    end,
}

local function spawn_zapring(owner: Object, damage: number, breaking: boolean)
    local x = owner.panel_x + owner.facing
    if not field.valid(x, owner.panel_y) then
        return
    end
    local o = battle.spawn(ZAPRING_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, owner.panel_y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 3
    o:set_damage_word(damage)
    ;(o.state :: ZapState).breaking = breaking
end

-- =============================================================================
-- Wood Style: Twister / Tornado 2 tiles ahead (8 rapid wood hits, no mercy invuln until hit 8!)
-- =============================================================================
type TwisterState = { tick: number, hits_done: number, breaking: boolean }

local TWISTER_KIND = new.kind {
    id = "megaman/charged-shot/twister",
    pool = "attack",
    state = { tick = "u16", hits_done = "u8", breaking = "bool" },
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
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0)
            if not me:create_collision() then
                me:free()
                return
            end
            local s = me.state :: TwisterState
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 1)
            me.collision.element = 4 -- wood
            me.collision:set_hit_effect(sparks.wood)
            me.collision:set_region(nil)
            me.collision:present()
            s.tick = 0
            s.hits_done = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            me:hit_spark()
            if battle.over() then
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: TwisterState
            if c.hit_flags ~= 0 then
                c:set_region(nil)
            end
            if s.tick % 3 == 0 and s.hits_done < 8 then
                s.hits_done += 1
                local hit_mod = if s.hits_done == 8 then 3 else 1
                local is_col = if s.breaking then collision.breaking else collision.attack
                me:reset_collision_types(is_col, collision.hits_navis, hit_mod)
                c.element = 4
                c:set_hit_effect(sparks.wood)
                c:set_region(regions.single)
                if s.hits_done % 2 == 1 then
                    battle.effect(me.pos, WIND_FX)
                    battle.play_sound(SHOT_SOUND)
                end
            elseif s.tick % 3 == 1 then
                c:set_region(nil)
            end
            s.tick += 1
            if s.tick >= 26 then
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            me:update_sprite()
            c:present()
        else
            me:destroy()
        end
    end,
}

local function spawn_twister(owner: Object, per_hit_damage: number, breaking: boolean)
    local x = owner.panel_x + owner.facing * 2
    if not field.valid(x, owner.panel_y) then
        return
    end
    local o = battle.spawn(TWISTER_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, owner.panel_y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 4
    o:set_damage_word(per_hit_damage)
    ;(o.state :: TwisterState).breaking = breaking
end

-- =============================================================================
-- Ground Style: CrackShot (cracks panels ahead and spreads cracks on impact/landing)
-- =============================================================================
type GroundState = { step_ticks: number, breaking: boolean }

local function crack_spread(x: number, y: number, facing: number)
    local targets = {
        { x, y },
        { x + facing, y },
        { x, y - 1 },
        { x, y + 1 },
    }
    local any = false
    for _, t in targets do
        local px, py = t[1], t[2]
        if field.valid(px, py) and field.crack(px, py) then
            any = true
            local cx, cy = field.center(px, py)
            battle.effect(Vec3.new(cx, cy, 0x08_0000), CRACK_FX)
        end
    end
    if any then
        battle.play_sound(CRACK_SOUND)
    end
end

local GROUND_KIND = new.kind {
    id = "megaman/charged-shot/ground-shot",
    pool = "attack",
    state = { step_ticks = "u8", breaking = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, SHOT_Z)
            if not me:create_collision() then
                me:free()
                return
            end
            local s = me.state :: GroundState
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 3)
            me.collision.element = 0
            me.collision:set_hit_effect(sparks.breaking)
            me.collision:set_region(regions.single)
            me.collision:present()
            s.step_ticks = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            me:hit_spark()
            if battle.over() then
                c:set_region(nil)
                me:set_lifecycle("destroy")
                return
            end
            if c.hit_flags ~= 0 then
                crack_spread(me.panel_x, me.panel_y, me.facing)
                c:set_region(nil)
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: GroundState
            s.step_ticks += 1
            if s.step_ticks >= 3 then
                s.step_ticks = 0
                if field.valid(me.panel_x, me.panel_y) then
                    field.crack(me.panel_x, me.panel_y)
                end
                local next_x = me.panel_x + me.facing
                if not field.valid(next_x, me.panel_y) then
                    crack_spread(me.panel_x, me.panel_y, me.facing)
                    c:set_region(nil)
                    me:set_lifecycle("destroy")
                    return
                end
                me.panel_x = next_x
                me:set_coordinates_from_panel()
                local p = me.pos
                me.pos = Vec3.new(p.x, p.y, SHOT_Z)
                me:update_collision_panels()
            end
            c:present()
        else
            me:destroy()
        end
    end,
}

local function spawn_ground_shot(owner: Object, damage: number, breaking: boolean)
    local x = owner.panel_x + owner.facing
    if not field.valid(x, owner.panel_y) then
        return
    end
    local o = battle.spawn(GROUND_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, owner.panel_y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 0
    o:set_damage_word(damage)
    ;(o.state :: GroundState).breaking = breaking
end

-- =============================================================================
-- Shared action wrapper for MegaMan's charged shots
-- =============================================================================
type State = { timer: number }

local function make_action(id: string, fire_fn: (me: Object, breaking: boolean) -> number): Action
    return new.action {
        id = id,
        state = { timer = "i16" },
        update = function(me: Object, s: State)
            local step = me.step
            if step == SHOOT then
                if me.step_init == 0 then
                    me:set_animation(anims.buster_shoot(me))
                    me.step_init = 4
                    me:set_status("using_action", true)
                    s.timer = 0
                end
                if s.timer == 1 then
                    battle.play_sound(SHOT_SOUND)
                    local breaking: boolean = battle.navi(me.alliance).brake_charge == true
                    me.attack_count = fire_fn(me, breaking)
                end
                s.timer += 1
                if s.timer > 4 then
                    me.step = RECOVER
                    me.step_init = 0
                end
            elseif step == RECOVER then
                if me.step_init == 0 then
                    s.timer = me.attack_count
                    me.step_init = 4
                end
                s.timer -= 1
                if s.timer < 0 then
                    me.related1 = nil
                    me:exit_attack(true)
                end
                if not me:can_move() then
                    return
                end
                local dir = me:held_direction()
                if dir == 0 or me:step_target(dir) == nil then
                    return
                end
                me.related1 = nil
                me:exit_attack()
                me:start_move(dir)
            else
                error(string.format("charged shot phase %#x out of range", step))
            end
        end,
    }
end

local NULL_ACTION = make_action("megaman/charged-shot/action", function(me, breaking)
    local v = if breaking then NULL_BREAK_SHOT else NULL_SHOT
    local z = if breaking then SHOT_Z else CHARGED_SHOT_Z
    return projectile.fire(me, { variant = v, damage = me.attack_damage, z = z })
end)

local HEAT_ACTION = make_action("megaman/charged-shot/heat-action", function(me, breaking)
    spawn_flamethrower(me, me.attack_damage, breaking)
    return 18
end)

local AQUA_ACTION = make_action("megaman/charged-shot/aqua-action", function(me, _breaking)
    bullet.fire(me, me.panel_x + me.facing, me.panel_y, SHOT_Z, BUBBLER_BULLET, 2, me.attack_damage)
    return 14
end)

local ELEC_ACTION = make_action("megaman/charged-shot/elec-action", function(me, breaking)
    spawn_zapring(me, me.attack_damage, breaking)
    return 14
end)

local WOOD_ACTION = make_action("megaman/charged-shot/wood-action", function(me, breaking)
    spawn_twister(me, me.attack_damage, breaking)
    return 24
end)

local GROUND_ACTION = make_action("megaman/charged-shot/ground-action", function(me, breaking)
    spawn_ground_shot(me, me.attack_damage, breaking)
    return 14
end)

local function make_weapon(id: string, action: Action, elem: number, base_damage: number, scale: number, weap_bonus: number): Weapon
    return new.weapon {
        id = id,
        charge_ticks = CHARGE_TICKS,
        setup = function(navi: Object): Action
            local stats = battle.navi(navi.alliance)
            local atk = stats.attack
            local wlv: number = stats.weapon_level or 0
            navi.attack_damage = base_damage + atk * scale + wlv * weap_bonus
            navi.attack_element = elem
            navi.charged = 0
            navi.extra = 0
            navi.hit_param = 0
            navi.attack_lockout = 0
            return action
        end,
    }
end

charged_shot.weapon = make_weapon("megaman/charged-shot", NULL_ACTION, 0, 10, 10, 10)
-- Heat Flamethrower: ~60..120 total fire damage across 3 flame bursts
charged_shot.heat = make_weapon("megaman/charged-shot/heat", HEAT_ACTION, 1, 60, 15, 15)
-- Aqua Bubbler: 40..80+ aqua splash to target + 1 panel behind
charged_shot.aqua = make_weapon("megaman/charged-shot/aqua", AQUA_ACTION, 2, 40, 10, 10)
-- Elec ZapRing: 20..60+ elec + Paralyze
charged_shot.elec = make_weapon("megaman/charged-shot/elec", ELEC_ACTION, 3, 20, 10, 10)
-- Wood Twister: 8 rapid hits of 10..30+ wood damage (80..240+ total!)
charged_shot.wood = make_weapon("megaman/charged-shot/wood", WOOD_ACTION, 4, 10, 5, 5)
-- Ground CrackShot: 30..70+ null + spreads cracks
charged_shot.ground = make_weapon("megaman/charged-shot/ground", GROUND_ACTION, 0, 30, 10, 10)

return charged_shot
