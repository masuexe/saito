--!strict
-- EXE3 Magnum, Ratton, NoBeam, IceWave & Arrow chips:
-- Magnum1-3 (0x051..0x053), Ratton1-3 (0x03A..0x03C), NoBeam1-3 (0x094..0x096),
-- IceWave1-3 (0x05A..0x05C), Arrow1-3 (0x060..0x062).

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local HIT_SPARKS = asset.sprite("hit-sparks")
local TARGET_SPRITE = HIT_SPARKS
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
local NOBEAM_FX = new.effect { sprite = asset.sprite("nobeam"), anim = 0, palette = 0, steps = 10 }
local AQUA_FX = new.effect { sprite = HIT_SPARKS, anim = 3, palette = 0, steps = 12 }
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 12 }
local CANNON_SOUND = asset.sound("cannon")
local BLAST_SOUND = asset.sound("hit-bomb")
local CRACK_SOUND = asset.sound("panel-crack")
local CURSOR_SOUND = asset.sound("sfx-cursor")
local SHOT_SOUND = asset.sound("buster-shot")
local FREEZE_SOUND = asset.sound("freeze")

local CURSOR_FIRE = element.byte("fire", { "cursor", "break" })

local function dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

-- =============================================================================
-- Magnum1 / Magnum2 / Magnum3:
-- Dimming cursor sweeps enemy panels; pressing A (or auto-lock) blasts & breaks
-- a vertical column (Magnum1), horizontal row (Magnum2), or cross (Magnum3)!
-- =============================================================================
type MagnumState = { mode: number, cx: number, cy: number, sweeps: number, fired: boolean }

local MAGNUM_CTRL = new.kind {
    id = "magnum/controller",
    pool = "effect",
    state = { mode = "u8", cx = "u8", cy = "u8", sweeps = "u8", fired = "bool" },
    update = dimming_chips.phases {
        name = "Magnum",
        effect = function(me: Object)
            local s = me.state :: MagnumState
            local user = me.related1
            if me.phase_init == 0 then
                me.visible = true
                local sprite = me.sprite
                sprite:load(TARGET_SPRITE)
                sprite.shadow = "hidden"
                me.anim, me.anim_loaded = 0, 0xFF
                s.cx = if me.alliance == 0 then 4 else 3
                s.cy = 1
                s.sweeps = 0
                s.fired = false
                me.timer = 0
                me.panel_x, me.panel_y = s.cx, s.cy
                me:set_coordinates_from_panel()
                me.phase_init = 4
            end
            if not s.fired then
                local pressed = user ~= nil and (user:dimmed_pressed("a") or user:pressed("a"))
                -- Sweep cursor across opponent's panels every 7 ticks.
                if me.timer > 0 and me.timer % 7 == 0 and not pressed then
                    s.cy += 1
                    if s.cy > 3 then
                        s.cy = 1
                        s.cx += me.facing
                        local min_x = if me.alliance == 0 then 4 else 1
                        local max_x = if me.alliance == 0 then 6 else 3
                        if s.cx < min_x or s.cx > max_x then
                            s.cx = if me.alliance == 0 then 4 else 3
                            s.sweeps += 1
                        end
                    end
                    me.panel_x, me.panel_y = s.cx, s.cy
                    me:set_coordinates_from_panel()
                    battle.play_sound(CURSOR_SOUND)
                end
                -- Auto-lock on opponent if player doesn't press A within 2 sweeps.
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                local auto_hit = s.sweeps >= 1 and opp ~= nil and opp.panel_x == s.cx and opp.panel_y == s.cy
                if pressed or auto_hit or s.sweeps >= 2 then
                    s.fired = true
                    me.timer = 0
                    me.visible = false
                    local region = if s.mode == 1
                        then regions.wide
                        elseif s.mode == 2 then regions.region_1d
                        else regions.plus
                    dim_hitbox(me, {
                        panel_x = s.cx,
                        panel_y = s.cy,
                        element = CURSOR_FIRE,
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.blast,
                        target = collision.hits_navis,
                        self_type = collision.piercing_break,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    battle.region_effects(s.cx, s.cy, region, me.alliance, BLAST_FX, 0x10_0000)
                    for _, offset in region.panels or {} do
                        local px = s.cx + offset[1] * me.facing
                        local py = s.cy + offset[2]
                        if field.valid(px, py) then
                            field.break_panel(px, py)
                        end
                    end
                    battle.play_sound(BLAST_SOUND)
                    battle.play_sound(CRACK_SOUND)
                    battle.shake_camera(2, 14)
                else
                    field.highlight(s.cx, s.cy)
                end
                me.timer += 1
                me:update_sprite_while_dimmed()
            else
                me.timer += 1
                if me.timer >= 20 then
                    dimming_chips.done(me)
                end
            end
        end,
    },
}

local function magnum_hook(mode: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(MAGNUM_CTRL, user, spec)
        if o then
            local st = o.state :: MagnumState
            st.mode = mode
        end
        return o
    end
end

-- =============================================================================
-- Ratton1 / Ratton2 / Ratton3:
-- Homing rat missile that travels forward and turns once toward opponent's row!
-- =============================================================================
type RattonState = { tick: number, step_ticks: number, turned: boolean, dir_x: number, dir_y: number }

local RATTON_KIND = new.kind {
    id = "magnum/ratton",
    pool = "attack",
    state = { tick = "u8", step_ticks = "u8", turned = "bool", dir_x = "i8", dir_y = "i8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("ratton"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 1, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: RattonState
            s.tick = 0
            s.turned = false
            s.dir_x = me.facing
            s.dir_y = 0
            battle.play_sound(SHOT_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: RattonState
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
                    element = 0,
                    z = 0x0C_0000,
                    region = regions.single,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                if hit then
                    battle.effect(me.pos, BLAST_FX)
                    battle.play_sound(BLAST_SOUND)
                    me:set_lifecycle("destroy")
                    return
                end
                -- Turn once when aligned with opponent's column!
                if not s.turned then
                    local opp = battle.player(opp_side)
                    if opp and not opp:status("dead") and opp.panel_x == me.panel_x and opp.panel_y ~= me.panel_y then
                        s.turned = true
                        s.dir_x = 0
                        s.dir_y = if opp.panel_y > me.panel_y then 1 else -1
                    end
                end
            end
            s.tick += 1
            if s.tick >= s.step_ticks then
                s.tick = 0
                local nx = me.panel_x + s.dir_x
                local ny = me.panel_y + s.dir_y
                if not field.valid(nx, ny) then
                    me:set_lifecycle("destroy")
                    return
                end
                me.panel_x, me.panel_y = nx, ny
                me:set_coordinates_from_panel()
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type ActTimer = { timer: number }

local function make_ratton_action(id: string, step_ticks: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
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
                    local o = battle.spawn(RATTON_KIND)
                    if o then
                        o.panel_x, o.panel_y = sx, me.panel_y
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 0
                        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(damage)
                        ;(o.state :: RattonState).step_ticks = step_ticks
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
-- NoBeam1 / NoBeam2 / NoBeam3:
-- High-power piercing beam! Fires at full power when an obstacle/object is
-- behind the user (or fires straight piercing row beam!).
-- =============================================================================
local function make_nobeam_action(id: string): Action
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
                    local has_behind = false
                    local bx = me.panel_x - me.facing
                    while field.valid(bx, me.panel_y) do
                        for _, obj in field.all_objects() do
                            if obj.active and obj.panel_x == bx and obj.panel_y == me.panel_y then
                                has_behind = true
                                break
                            end
                        end
                        if has_behind then
                            break
                        end
                        bx -= me.facing
                    end
                    local base = me.attack_damage + me.extra
                    local final_dmg = if has_behind then base else math.max(10, base // 2)
                    local damage = int.u32(final_dmg + me.hit_param * 0x10000)
                    battle.hitbox(me, {
                        panel_x = sx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.row,
                        hit_effect = sparks.charged,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = damage,
                        hit_mod = 3,
                    })
                    battle.region_effects(sx, me.panel_y, regions.row, me.alliance, NOBEAM_FX, 0x10_0000)
                    battle.play_sound(CANNON_SOUND)
                    battle.shake_camera(2, 10)
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
-- IceWave1 / IceWave2 / IceWave3:
-- Aqua sliding ice wave sweeping across the row & freezing panels to ice!
-- =============================================================================
type IceWaveState = { tick: number, step_ticks: number, freeze_panels: boolean }

local ICEWAVE_KIND = new.kind {
    id = "magnum/icewave",
    pool = "attack",
    state = { tick = "u8", step_ticks = "u8", freeze_panels = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("ice"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            ;(me.state :: IceWaveState).tick = 0
            battle.play_sound(FREEZE_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: IceWaveState
            if s.tick == 0 then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 2, -- aqua
                    z = 0x0C_0000,
                    region = regions.single,
                    hit_effect = sparks.aqua,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.effect(me.pos, AQUA_FX)
                if s.freeze_panels and field.solid(me.panel_x, me.panel_y) then
                    field.set_type(me.panel_x, me.panel_y, "ice")
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
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local function make_icewave_action(id: string, step_ticks: number, rows: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.THROW_LOW)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 5 then
                local sx = me.panel_x + me.facing
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                for row = 1, 3 do
                    if rows >= 3 or row == me.panel_y or (rows == 2 and math.abs(row - me.panel_y) <= 1) then
                        if field.valid(sx, row) then
                            local o = battle.spawn(ICEWAVE_KIND)
                            if o then
                                o.panel_x, o.panel_y = sx, row
                                o.alliance, o.flip = me.alliance, me.flip
                                o.element = 2
                                o:set_damage_word(damage)
                                local is = o.state :: IceWaveState
                                is.step_ticks = step_ticks
                                is.freeze_panels = true
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
end

-- =============================================================================
-- Arrow1 / Arrow2 / Arrow3:
-- Elec piercing multi-row lightning arrows sweeping across enemy rows!
-- =============================================================================
type ArrowState = { tick: number, step_ticks: number }

local ARROW_KIND = new.kind {
    id = "magnum/arrow",
    pool = "attack",
    state = { tick = "u8", step_ticks = "u8" },
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
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 4, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0x10_0000)
            ;(me.state :: ArrowState).tick = 0
            battle.play_sound(SHOT_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: ArrowState
            if s.tick == 0 then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 3, -- elec
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.elec,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.effect(me.pos, ELEC_FX)
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

local function make_arrow_action(id: string, tier: number): Action
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
                local sx = if me.alliance == 0 then 1 else 6
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                for row = 1, 3 do
                    if tier >= 3 or row == me.panel_y or (tier == 2 and row ~= 2) or (tier == 1 and row == me.panel_y) then
                        local o = battle.spawn(ARROW_KIND)
                        if o then
                            o.panel_x, o.panel_y = sx, row
                            o.alliance, o.flip = me.alliance, me.flip
                            o.element = 3
                            o:set_damage_word(damage)
                            ;(o.state :: ArrowState).step_ticks = math.max(3, 6 - tier)
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
end

local magnum1: Chip = {
    codes = { "A", "C", "H", "T", "V" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 35,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = magnum_hook(1),
}

local magnum2: Chip = {
    codes = { "B", "G", "N", "O", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 55,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = magnum_hook(2),
}

local magnum3: Chip = {
    codes = { "D", "F", "I", "S", "W" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 75,
    damage = 180,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = magnum_hook(3),
}

local ratton1: Chip = {
    action = make_ratton_action("ratton1/action", 7),
    codes = { "A", "C", "F", "H", "J" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local ratton2: Chip = {
    action = make_ratton_action("ratton2/action", 5),
    codes = { "A", "C", "F", "N", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local ratton3: Chip = {
    action = make_ratton_action("ratton3/action", 4),
    codes = { "A", "C", "F", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local nobeam1: Chip = {
    action = make_nobeam_action("nobeam1/action"),
    codes = { "C", "F", "G", "K", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 66,
    damage = 200,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local nobeam2: Chip = {
    action = make_nobeam_action("nobeam2/action"),
    codes = { "E", "I", "S", "U", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 77,
    damage = 260,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local nobeam3: Chip = {
    action = make_nobeam_action("nobeam3/action"),
    codes = { "H", "M", "V", "W", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 88,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local icewave1: Chip = {
    action = make_icewave_action("icewave1/action", 6, 1),
    codes = { "A", "K", "M", "Q", "W", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local icewave2: Chip = {
    action = make_icewave_action("icewave2/action", 5, 2),
    codes = { "D", "H", "L", "P", "R", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local icewave3: Chip = {
    action = make_icewave_action("icewave3/action", 4, 3),
    codes = { "C", "D", "J", "R", "V" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local arrow1: Chip = {
    action = make_arrow_action("arrow1/action", 1),
    codes = { "B", "E", "M", "Q", "T" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 40,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local arrow2: Chip = {
    action = make_arrow_action("arrow2/action", 2),
    codes = { "F", "R", "S", "U", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 60,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local arrow3: Chip = {
    action = make_arrow_action("arrow3/action", 3),
    codes = { "A", "H", "J", "N", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 80,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    magnum1 = magnum1,
    magnum2 = magnum2,
    magnum3 = magnum3,
    ratton1 = ratton1,
    ratton2 = ratton2,
    ratton3 = ratton3,
    nobeam1 = nobeam1,
    nobeam2 = nobeam2,
    nobeam3 = nobeam3,
    icewave1 = icewave1,
    icewave2 = icewave2,
    icewave3 = icewave3,
    arrow1 = arrow1,
    arrow2 = arrow2,
    arrow3 = arrow3,
}
