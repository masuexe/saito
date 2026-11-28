--!strict
-- MegaMan's authentic EXE3 charged shots, verified against A6BE (`bn3.gba`):
--   - Normal (0x080B0F30): plain/BrakChrg charged buster shot, `(attack + 1) * 10` damage
--   - Heat   (0x080B1B98 / 0x080D6FB4): Flamethrower (`0c-11` arm + 3-panel `0c-10` flame stream at z=0, SFX 0x82, 50/80/100)
--   - Aqua   (0x080B14D2 / 0x080D5FEA): Bubbler (`0c-1c` flash + `0c-16` bubble splash at z=0, SFX 0x7D / 0xAE, 40/60/80)
--   - Elec   (0x080B1CB8 / 0x080D71A8): ZapRing (`0c-1b` arm + `10-03` spinning ring at z=24, SFX 0xB0, paralyze, 20/30/40)
--   - Wood   (0x080B1D6E / 0x080D72E4): Twister (`0c-19` arm + `0c-1a` pal 3 green tornado at z=0, SFX 0xC9, 8x 10/15/20)
--   - Ground (fallback when un-elemented): CrackShot cracking panels ahead
-- Charge times from A6BE `0x080B5980` (`0x8000 / rate`); honors BrakChrg (`stats.brake_charge`) and WeapLV+1 (`stats.weapon_level`).

local anims = require("../../../lib/anims")
local api = require("../../../rules/api")
local attachment = require("../../../objects/attachment")
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
-- EXE3's `pink-flash` (14-04) bakes a -16px vertical OAM offset (`y_mid = -16.5`), so subtracting 0x10_0000
-- keeps the charged shot burst centered on the target's torso (-24.5px).
local CHARGED_SHOT_Z = 0x08_0000

-- Authentic charge thresholds derived from A6BE `0x080B5980` (`0x8000 / rate` across Charge Lv 1..5).
local NORMAL_CHARGE_TICKS: { number } = { 100, 85, 70, 55, 40, 40 }
local HEAT_WOOD_CHARGE_TICKS: { number } = { 130, 110, 90, 70, 50, 50 }
local ELEC_CHARGE_TICKS: { number } = { 150, 125, 105, 85, 65, 65 }

-- Authentic Style WeaponLv 1..3 damage tables (A6BE 0x080B1BF4, 0x080B1540, 0x080B1CFC, 0x080B1DDC).
local HEAT_DAMAGE: { number } = { 50, 80, 100 }
local AQUA_DAMAGE: { number } = { 40, 60, 80 }
local ELEC_DAMAGE: { number } = { 20, 30, 40 }
local WOOD_DAMAGE: { number } = { 10, 15, 20 }

local SHOT_SOUND = asset.sound("buster-shot")
local CANNON_SOUND = asset.sound("spreader") -- A6BE 0x080B1554 = 0x7D (Aqua Bubbler fire)
local BUBBLER_SPLASH_SOUND = asset.sound("freeze") -- A6BE 0x080D5FAF[1] = 0xAE
local FLAME_SOUND = asset.sound("flamethrower") -- A6BE 0x080B1C76 = 0x82
local ZAPRING_SOUND = asset.sound("zapring") -- A6BE 0x080B1D4E = 0xB0
local TWISTER_SOUND = asset.sound("twister") -- A6BE 0x080B1E82 / 0x080D737A = 0xC9
local CRACK_SOUND = asset.sound("panel-crack")

local FLAME_SPRITE = asset.sprite("flame") -- 0c-10
local FLAMETHROWER_ARM = attachment.look { sprite = asset.sprite("flamethrower"), palette = 0 } -- 0c-11
local TWISTER_ARM = attachment.look { sprite = asset.sprite("twister-arm"), palette = 0 } -- 0c-19 (A6BE 0x0c19xx80: pal 0 blue)
local TORNADO_SPRITE = asset.sprite("tornado") -- 0c-1a (pal 3 = Wood green)
local ZAPRING_ARM = attachment.look { sprite = asset.sprite("zapring-arm"), palette = 0 } -- 0c-1b
local ZAPRING_SPRITE = asset.sprite("zapring") -- 10-03
local SMALL_PUFF_SPRITE = asset.sprite("small-puff")

local MUZZLE_FLASH = new.effect { sprite = asset.sprite("spreader-flash"), anim = 0, palette = 0, steps = 7 }
local AQUA_FX = new.effect { sprite = asset.sprite("bubble-splash"), anim = 0, palette = 0, steps = 26 }
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

-- Aqua Bubbler bullet (A6BE 0x080D5FEA): hits target panel + 1 panel behind (`regions.long`),
-- spawns `0c-16` bubble splash at z=0 with sound 0xAE.
local BUBBLER_BULLET = bullet.variant {
    region = regions.long,
    hit_mod = 3,
    effect = AQUA_FX,
    sound = BUBBLER_SPLASH_SOUND,
}

-- Ground-anchored attack visuals (flame `0c-10`, tornado `0c-1a`) draw at z=0, while their hit sparks
-- spawn at torso height (`SHOT_Z`, matching A6BE `0x080D704A` / `0x080D7372`).
local function hit_spark_at_torso(me: Object)
    local saved = me.pos
    me.pos = Vec3.new(saved.x, saved.y, SHOT_Z)
    me:hit_spark()
    me.pos = saved
end

local function style_level_damage(table_dmg: { number }, stats: Navi): number
    local wlv: number = stats.weapon_level or 0
    local idx = math.clamp(1 + wlv, 1, #table_dmg)
    return table_dmg[idx]
end

-- =============================================================================
-- Heat Style: Flamethrower (A6BE 0x080B1B98 + 0x080D6FB4)
-- 3 chained `0c-10` flame segments at z=0 (overlapping via 0x080D7068 x-offsets 0/8/16px),
-- single solid fire hit (hit_mod=3), melts ice panels, 2x on grass.
-- =============================================================================
type FlameState = {
    ticks: number,
    duration: number,
    remain: number,
    chained: boolean,
    breaking: boolean,
    ground_crack: boolean,
}

-- A6BE `0x080D7068`: [0x18, 0x10, 0x08, 0x00] indexed by `remain` (3 -> 0px, 2 -> 8px, 1 -> 16px).
local FLAME_X_SHIFT: { [number]: number } = { [1] = 16, [2] = 8, [3] = 0 }

local FLAME_KIND: Kind

local function spawn_flame_segment(
    owner: Object,
    x: number,
    y: number,
    damage: number,
    remain: number,
    duration: number,
    breaking: boolean,
    ground_crack: boolean
)
    if not field.valid(x, y) or duration <= 0 then
        return
    end
    local o = battle.spawn(FLAME_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 1 -- fire
    o:set_damage_word(damage)
    local fs = o.state :: FlameState
    fs.remain = remain
    fs.duration = duration
    fs.breaking = breaking
    fs.ground_crack = ground_crack
end

FLAME_KIND = new.kind {
    id = "megaman/charged-shot/flamethrower",
    pool = "attack",
    state = {
        ticks = "u16",
        duration = "u16",
        remain = "u8",
        chained = "bool",
        breaking = "bool",
        ground_crack = "bool",
    },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            local s = me.state :: FlameState
            me.visible = true
            local sprite = me.sprite
            sprite:load(FLAME_SPRITE)
            sprite.shadow = "with_sprite"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local shift = (FLAME_X_SHIFT[s.remain] or 0) * me.facing * 0x1_0000
            local p = me.pos
            me.pos = Vec3.new(p.x - shift, p.y, 0)
            if not me:create_collision() then
                me:free()
                return
            end
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 3)
            local c = me.collision
            c.element = 1 -- fire
            c:set_hit_effect(sparks.fire)
            c:set_region(regions.single)
            c:present()
            s.ticks = 0
            s.chained = false
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            hit_spark_at_torso(me)
            local s = me.state :: FlameState
            if c.hit_flags ~= 0 then
                -- A6BE 0x080D7086: disable hitbox after connecting once while the flame visual finishes.
                c:set_region(nil)
            end
            if battle.over() or s.duration <= 0 then
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            local panel = field.panel(me.panel_x, me.panel_y)
            if panel and panel.kind == "ice" then
                field.set_type(me.panel_x, me.panel_y, "normal")
            end
            if s.ground_crack and s.ticks == 0 then
                field.crack(me.panel_x, me.panel_y)
            end
            -- A6BE 0x080D70DC: after 1 flame animation loop (6 ticks), chain into the next panel forward.
            if not s.chained and s.ticks >= 6 then
                s.chained = true
                if s.remain > 1 then
                    spawn_flame_segment(
                        me,
                        me.panel_x + me.facing,
                        me.panel_y,
                        me.damage,
                        s.remain - 1,
                        s.duration,
                        s.breaking,
                        s.ground_crack
                    )
                end
            end
            s.ticks += 1
            s.duration -= 1
            me:update_sprite()
            c:present()
        else
            me:destroy()
        end
    end,
}

-- =============================================================================
-- Elec Style: ZapRing (A6BE 0x080B1CB8 + 0x080D71A8)
-- Spinning `10-03` electric ring flying smoothly at 10 px/tick at z=24, paralyzing on hit
-- with hit_mod=0 (no mercy flash, matching A6BE 0x080D7230).
-- =============================================================================
type ZapState = { vx: number, breaking: boolean, ground_crack: boolean }

local ZAPRING_SPEED = 10 * 0x1_0000 -- A6BE 0x080D7190[0] = 0x0A px/tick

local ZAPRING_KIND = new.kind {
    id = "megaman/charged-shot/zapring",
    pool = "attack",
    state = { vx = "i32", breaking = "bool", ground_crack = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            local s = me.state :: ZapState
            me.visible = true
            local sprite = me.sprite
            sprite:load(ZAPRING_SPRITE)
            sprite.shadow = "with_sprite"
            sprite.palette = 0
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, SHOT_Z)
            s.vx = ZAPRING_SPEED * me.facing
            if not me:create_collision() then
                me:free()
                return
            end
            local is_col = if s.breaking then collision.breaking else collision.attack
            -- A6BE 0x080D7230: hit_mod=0 + paralyze (no mercy invulnerability flash).
            me:setup_collision(is_col, collision.hits_navis, 0)
            local c = me.collision
            c.element = 3 -- elec
            c:set_hit_effect(sparks.elec)
            c.status_base = statuses.paralyze
            c:set_region(regions.single)
            c:present()
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            me:hit_spark()
            local s = me.state :: ZapState
            if battle.over() or c.hit_flags ~= 0 then
                if c.hit_flags ~= 0 and s.ground_crack then
                    field.crack(me.panel_x, me.panel_y)
                end
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            local p = me.pos
            me.pos = Vec3.new(p.x + s.vx, p.y, p.z)
            me:set_panel_from_coordinates()
            if not field.valid(me.panel_x, me.panel_y) then
                c:set_region(nil)
                me.visible = false
                me:set_lifecycle("destroy")
                return
            end
            me:update_collision_panels()
            me:update_sprite()
            c:present()
        else
            me:destroy()
        end
    end,
}

local function spawn_zapring(owner: Object, damage: number, breaking: boolean, ground_crack: boolean)
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
    local zs = o.state :: ZapState
    zs.breaking = breaking
    zs.ground_crack = ground_crack
end

-- =============================================================================
-- Wood Style: Twister (A6BE 0x080B1D6E + 0x080D72E4)
-- Green tornado (`0c-1a` palette 3, `0x080D7394[1] = 3`) 2 panels ahead at z=0,
-- 8 rapid wood hits over 32 ticks (every 4 ticks, hit_mod=1, SFX 0xC9).
-- =============================================================================
type TwisterState = { tick: number, hits_done: number, breaking: boolean, ground_crack: boolean }

local TWISTER_KIND = new.kind {
    id = "megaman/charged-shot/twister",
    pool = "attack",
    state = { tick = "u16", hits_done = "u8", breaking = "bool", ground_crack = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            local s = me.state :: TwisterState
            me.visible = true
            local sprite = me.sprite
            sprite:load(TORNADO_SPRITE)
            sprite.shadow = "with_sprite"
            sprite.palette = 3 -- A6BE 0x080D7394[1] = 3 (Wood green tornado)
            me.anim, me.anim_loaded = 0, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0)
            if not me:create_collision() then
                me:free()
                return
            end
            local is_col = if s.breaking then collision.breaking else collision.attack
            me:setup_collision(is_col, collision.hits_navis, 1)
            me.collision.element = 4 -- wood
            me.collision:set_hit_effect(sparks.wood)
            me.collision:set_region(nil)
            me.collision:present()
            if s.ground_crack then
                field.crack(me.panel_x, me.panel_y)
            end
            s.tick = 0
            s.hits_done = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            local c = me.collision
            c:remove()
            hit_spark_at_torso(me)
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
            -- A6BE 0x080D73FE..0x080D741A: 32 ticks total, re-arming collision every 4 ticks -> 8 hits (hit_mod=1).
            if s.tick % 4 == 0 and s.hits_done < 8 then
                s.hits_done += 1
                local is_col = if s.breaking then collision.breaking else collision.attack
                me:reset_collision_types(is_col, collision.hits_navis, 1)
                c.element = 4
                c:set_hit_effect(sparks.wood)
                c:set_region(regions.single)
            elseif s.tick % 4 == 1 then
                c:set_region(nil)
            end
            s.tick += 1
            if s.tick >= 32 then
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

local function spawn_twister(owner: Object, per_hit_damage: number, breaking: boolean, ground_crack: boolean)
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
    local ts = o.state :: TwisterState
    ts.breaking = breaking
    ts.ground_crack = ground_crack
end

-- =============================================================================
-- Ground Style fallback (un-elemented CrackShot)
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
-- Actions for Normal, Heat, Aqua, Elec, Wood, and Ground charged shots
-- =============================================================================
type TimerState = { timer: number }

local function make_buster_action(id: string, fire_fn: (me: Object, breaking: boolean) -> number): Action
    return new.action {
        id = id,
        state = { timer = "i16" },
        update = function(me: Object, s: TimerState)
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

local NULL_ACTION = make_buster_action("megaman/charged-shot/action", function(me, breaking)
    local v = if breaking then NULL_BREAK_SHOT else NULL_SHOT
    local z = if breaking then SHOT_Z else CHARGED_SHOT_Z
    return projectile.fire(me, { variant = v, damage = me.attack_damage, z = z })
end)

local GROUND_ACTION = make_buster_action("megaman/charged-shot/ground-action", function(me, breaking)
    spawn_ground_shot(me, me.attack_damage, breaking)
    return 14
end)

-- Heat Flamethrower action (A6BE 0x080B1B98): raise 0x36 (15t) + hold 0x37 (30t) with 0c-11 arm & SFX 0x82.
local HEAT_ACTION = new.action {
    id = "megaman/charged-shot/heat-action",
    state = { timer = "i16" },
    update = function(me: Object, s: TimerState)
        local step = me.step
        if step == 0x0 then
            if me.step_init == 0 then
                me:set_animation(anims.FLAMETHROWER)
                me.anim_loaded = 0xFF
                attachment.attach(me, FLAMETHROWER_ARM, "related", 0)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            s.timer += 1
            if s.timer >= 15 then
                me.step, me.step_init = 0x4, 0
            end
        elseif step == 0x4 then
            if me.step_init == 0 then
                me:set_animation(anims.FLAMETHROWER_HOLD)
                me.anim_loaded = 0xFF
                attachment.attach(me, FLAMETHROWER_ARM, "related", 1)
                local stats = battle.navi(me.alliance)
                local breaking: boolean = stats.brake_charge == true
                local ground_crack = api.style_type(me.alliance) == "ground"
                spawn_flame_segment(
                    me,
                    me.panel_x + me.facing,
                    me.panel_y,
                    me.attack_damage,
                    3,
                    30,
                    breaking,
                    ground_crack
                )
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 0 or s.timer == 15 then
                battle.play_sound(FLAME_SOUND)
            end
            s.timer += 1
            if s.timer >= 30 then
                me.related1 = nil
                me:exit_attack()
            end
        else
            error(string.format("Heat Flamethrower phase %#x out of range", step))
        end
    end,
}

-- Aqua Bubbler action (A6BE 0x080B14D2): buster shoot anim + 0c-1c flash + SFX 0x7D + bullet at z=0.
local AQUA_ACTION = new.action {
    id = "megaman/charged-shot/aqua-action",
    state = { timer = "i16" },
    update = function(me: Object, s: TimerState)
        local step = me.step
        if step == SHOOT then
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me.anim_loaded = 0xFF
                battle.effect(me.pos, MUZZLE_FLASH, me.alliance)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 1 then
                battle.play_sound(CANNON_SOUND)
                -- A6BE 0x080B158E: z = 0 so `0c-16` bubble-splash lands on the target panel!
                bullet.fire(me, me.panel_x + me.facing, me.panel_y, 0, BUBBLER_BULLET, 2, me.attack_damage)
            end
            s.timer += 1
            if s.timer > 4 then
                me.step, me.step_init = RECOVER, 0
            end
        elseif step == RECOVER then
            if me.step_init == 0 then
                s.timer = 15 -- A6BE 0x080B159A: 0x0F ticks
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer < 0 then
                me.related1 = nil
                me:exit_attack(true)
            end
        else
            error(string.format("Aqua Bubbler phase %#x out of range", step))
        end
    end,
}

-- Elec ZapRing action (A6BE 0x080B1CB8): anim 0x36 + 0c-1b arm + SFX 0xB0 + 10-03 ring at z=24.
local ELEC_ACTION = new.action {
    id = "megaman/charged-shot/elec-action",
    state = { timer = "i16" },
    update = function(me: Object, s: TimerState)
        local step = me.step
        if step == SHOOT then
            if me.step_init == 0 then
                me:set_animation(anims.FLAMETHROWER)
                me.anim_loaded = 0xFF
                attachment.attach(me, ZAPRING_ARM, "related", 0)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 1 then
                local stats = battle.navi(me.alliance)
                local breaking: boolean = stats.brake_charge == true
                local ground_crack = api.style_type(me.alliance) == "ground"
                spawn_zapring(me, me.attack_damage, breaking, ground_crack)
                battle.play_sound(ZAPRING_SOUND)
                me.step, me.step_init = RECOVER, 0
                return
            end
            s.timer += 1
        elseif step == RECOVER then
            if me.step_init == 0 then
                s.timer = 20 -- A6BE 0x080B1D4A: 0x14 ticks
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer <= 0 then
                me.related1 = nil
                me:exit_attack()
            end
        else
            error(string.format("Elec ZapRing phase %#x out of range", step))
        end
    end,
}

-- Wood Twister action (A6BE 0x080B1D6E): anim 0x2D (11t) -> 0x2E (1t) -> 0x2F (32t) -> 0x30 (5t) with 0c-19 arm (pal 0 blue) & SFX 0xC9.
local WOOD_ACTION = new.action {
    id = "megaman/charged-shot/wood-action",
    state = { timer = "i16" },
    update = function(me: Object, s: TimerState)
        local step = me.step
        if step == 0x0 then
            if me.step_init == 0 then
                me:set_animation(anims.TWISTER_WINDUP)
                me.anim_loaded = 0xFF
                attachment.attach(me, TWISTER_ARM, "related", 0)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            s.timer += 1
            if s.timer >= 11 then
                me.step, me.step_init = 0x4, 0
            end
        elseif step == 0x4 then
            if me.step_init == 0 then
                me:set_animation(anims.TWISTER_OPEN)
                me.anim_loaded = 0xFF
                attachment.attach(me, TWISTER_ARM, "related", 1)
                s.timer = 0
                me.step_init = 4
            end
            s.timer += 1
            if s.timer >= 1 then
                me.step, me.step_init = 0x8, 0
            end
        elseif step == 0x8 then
            if me.step_init == 0 then
                me:set_animation(anims.TWISTER_HOLD)
                me.anim_loaded = 0xFF
                attachment.attach(me, TWISTER_ARM, "related", 2)
                local stats = battle.navi(me.alliance)
                local breaking: boolean = stats.brake_charge == true
                local ground_crack = api.style_type(me.alliance) == "ground"
                spawn_twister(me, me.attack_damage, breaking, ground_crack)
                battle.play_sound(TWISTER_SOUND)
                s.timer = 32 -- A6BE 0x080B1E7A: 0x20 ticks
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer <= 0 then
                me.step, me.step_init = 0xC, 0
            end
        elseif step == 0xC then
            if me.step_init == 0 then
                me:set_animation(anims.TWISTER_CLOSE)
                me.anim_loaded = 0xFF
                attachment.attach(me, TWISTER_ARM, "related", 3)
                s.timer = 5 -- A6BE 0c-19 anim 3 delays: [2, 1, 1, 1]
                me.step_init = 4
            end
            s.timer -= 1
            if s.timer <= 0 then
                me.related1 = nil
                me:exit_attack()
            end
        else
            error(string.format("Wood Twister phase %#x out of range", step))
        end
    end,
}

local function make_weapon(
    id: string,
    action: Action,
    charge_ticks: { number },
    elem: number,
    damage_fn: (stats: Navi) -> number
): Weapon
    return new.weapon {
        id = id,
        charge_ticks = charge_ticks,
        setup = function(navi: Object): Action
            local stats = battle.navi(navi.alliance)
            navi.attack_damage = damage_fn(stats)
            navi.attack_element = elem
            navi.charged = 0
            navi.extra = 0
            navi.hit_param = 0
            navi.attack_lockout = 0
            return action
        end,
    }
end

charged_shot.weapon = make_weapon("megaman/charged-shot", NULL_ACTION, NORMAL_CHARGE_TICKS, 0, function(stats)
    return (stats.attack + 1) * 10
end)
charged_shot.heat = make_weapon("megaman/charged-shot/heat", HEAT_ACTION, HEAT_WOOD_CHARGE_TICKS, 1, function(stats)
    return style_level_damage(HEAT_DAMAGE, stats)
end)
charged_shot.aqua = make_weapon("megaman/charged-shot/aqua", AQUA_ACTION, NORMAL_CHARGE_TICKS, 2, function(stats)
    return style_level_damage(AQUA_DAMAGE, stats)
end)
charged_shot.elec = make_weapon("megaman/charged-shot/elec", ELEC_ACTION, ELEC_CHARGE_TICKS, 3, function(stats)
    return style_level_damage(ELEC_DAMAGE, stats)
end)
charged_shot.wood = make_weapon("megaman/charged-shot/wood", WOOD_ACTION, HEAT_WOOD_CHARGE_TICKS, 4, function(stats)
    return style_level_damage(WOOD_DAMAGE, stats)
end)
charged_shot.ground = make_weapon("megaman/charged-shot/ground", GROUND_ACTION, NORMAL_CHARGE_TICKS, 0, function(stats)
    local wlv: number = stats.weapon_level or 0
    return 30 + stats.attack * 10 + wlv * 10
end)

return charged_shot
