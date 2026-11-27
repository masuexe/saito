--!strict
-- EXE3 Navi Chips Part 2 (36 chips):
--   DesrtMan V1-V5, FlamMan V1-V5, DrillMan V1-V5, MetalMan V1-V5,
--   KingMan V1-V5, DarkMan V1-V5, JapanMan V1-V5, Punk.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local h = require("./helpers")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

type StepState = { step: number, count: number }

-- =============================================================================
-- 10. DesrtMan (V1-V5): Summons sand pillars/fists striking enemy rows!
-- =============================================================================
local DESRTMAN_KIND = new.kind {
    id = "navis/desrtman-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "DesrtMan",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 10 == 4 and s.step < 3 then
                s.step += 1
                local col = h.enemy_front_col(me.alliance)
                local row = s.step
                local x = col + (s.step - 1) * me.facing
                if not field.valid(x, row) then
                    x = col
                end
                h.dim_hitbox(me, {
                    panel_x = x,
                    panel_y = row,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.line,
                    hit_effect = sparks.impact,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(x, row, regions.line, me.alliance, h.DUST_FX, 0x10_0000)
                battle.play_sound(h.THROW_SOUND)
                battle.shake_camera(2, 8)
            end
            me.timer += 1
            if me.timer >= 42 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 11. FlamMan (V1-V5): 3-row Fire flame sweep!
-- =============================================================================
local FLAMMAN_KIND = new.kind {
    id = "navis/flamman-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "FlamMan",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 8 == 4 and s.step < 3 then
                s.step += 1
                local start_x = if me.alliance == 0 then 1 else 6
                local row = s.step
                h.dim_hitbox(me, {
                    panel_x = start_x,
                    panel_y = row,
                    element = 1,
                    z = 0x10_0000,
                    region = regions.row,
                    hit_effect = sparks.fire,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(start_x, row, regions.row, me.alliance, h.FIRE_FX, 0x10_0000)
                battle.play_sound(h.BLAST_SOUND)
            end
            me.timer += 1
            if me.timer >= 38 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 12. DrillMan (V1-V5): 3-row breaking drill pierce across enemy area!
-- =============================================================================
local DRILLMAN_KIND = new.kind {
    id = "navis/drillman-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "DrillMan",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 6 == 2 and s.step < 6 then
                local x = if me.alliance == 0 then (1 + s.step) else (6 - s.step)
                s.step += 1
                if field.valid(x, 2) then
                    h.dim_hitbox(me, {
                        panel_x = x,
                        panel_y = 2,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.wide,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.piercing_break,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = if s.step >= 5 then 3 else 1,
                    })
                    battle.region_effects(x, 2, regions.wide, me.alliance, h.BREAK_FX, 0x10_0000)
                    battle.play_sound(h.CRACK_SHOT_SOUND)
                end
            end
            me.timer += 1
            if me.timer >= 46 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 13. MetalMan (V1-V5): Breaking MetalFist smashing panel 2 ahead & cracking!
-- =============================================================================
local METALMAN_KIND = new.kind {
    id = "navis/metalman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "MetalMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 14 then
                local tx = me.panel_x + me.facing * 2
                local ty = me.panel_y
                if not field.valid(tx, ty) then
                    tx, ty = h.enemy_target(me.alliance)
                end
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                if field.valid(tx, ty) then
                    field.break_panel(tx, ty)
                    local cx, cy = field.center(tx, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), h.IMPACT_FX)
                end
                battle.play_sound(h.THROW_SOUND)
                battle.play_sound(h.CRACK_SOUND)
                battle.shake_camera(3, 14)
            end
            me.timer += 1
            if me.timer >= 36 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 14. KingMan (V1-V5): Checkmate leap striking center/target + surrounding panels!
-- =============================================================================
local KINGMAN_KIND = new.kind {
    id = "navis/kingman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "KingMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 14 then
                local tx, ty = h.enemy_target(me.alliance)
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.plus,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(tx, ty, regions.plus, me.alliance, h.IMPACT_FX, 0x10_0000)
                battle.play_sound(h.ARRIVE_SOUND)
                battle.play_sound(h.BLAST_SOUND)
                battle.shake_camera(3, 14)
            end
            me.timer += 1
            if me.timer >= 38 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 15. DarkMan (V1-V5): Dark bat swarm inflicting random status (blind/confuse/paralyze)!
-- =============================================================================
local DARK_STATUSES = { statuses.blind, statuses.confuse, statuses.paralyze }
local DARK_FXS = { h.BLIND_FX, h.CONFUSE_FX, h.ELEC_FX }

local DARKMAN_KIND = new.kind {
    id = "navis/darkman-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "DarkMan",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 7 == 3 and s.step < s.count then
                s.step += 1
                local tx, ty = h.enemy_target(me.alliance)
                local pick = (battle.rng_positive() % 3) + 1
                local last = s.step == s.count
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.charged,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                    status = if last then DARK_STATUSES[pick] else nil,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), DARK_FXS[pick])
                battle.play_sound(if last then h.CONFUSE_SOUND else h.BURST_SOUND)
            end
            me.timer += 1
            if me.timer >= s.count * 7 + 14 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function darkman_hook(bats: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(DARKMAN_KIND, user, spec, true)
        if o then
            (o.state :: StepState).count = bats
        end
        return o
    end
end

-- =============================================================================
-- 16. JapanMan (V1-V5): Multi-hit spear thrust across target!
-- =============================================================================
local JAPANMAN_KIND = new.kind {
    id = "navis/japanman-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "JapanMan",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 5 == 2 and s.step < s.count then
                s.step += 1
                local tx, ty = h.enemy_target(me.alliance)
                local last = s.step == s.count
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.long,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), if last then h.BIG_SLASH_FX else h.SLASH_FX, me.alliance)
                battle.play_sound(if last then h.SWING_SOUND else h.SWORD_SOUND)
            end
            me.timer += 1
            if me.timer >= s.count * 5 + 14 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function japanman_hook(thrusts: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(JAPANMAN_KIND, user, spec)
        if o then
            (o.state :: StepState).count = thrusts
        end
        return o
    end
end

-- =============================================================================
-- 17. Punk (0x110): High-speed rolling wheel smash!
-- =============================================================================
local PUNK_KIND = new.kind {
    id = "navis/punk-controller",
    pool = "effect",
    state = { step = "u8", count = "u8" },
    update = dimming_chips.phases {
        name = "Punk",
        effect = function(me: Object)
            local s = me.state :: StepState
            if me.phase_init == 0 then
                battle.play_sound(h.APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 4 == 2 and s.step < 6 then
                local x = if me.alliance == 0 then (1 + s.step) else (6 - s.step)
                s.step += 1
                local _, ty = h.enemy_target(me.alliance)
                if field.valid(x, ty) then
                    h.dim_hitbox(me, {
                        panel_x = x,
                        panel_y = ty,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 5,
                    })
                    local cx, cy = field.center(x, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), h.BURST_FX)
                    battle.play_sound(h.CANNON_SOUND)
                end
            end
            me.timer += 1
            if me.timer >= 36 then
                dimming_chips.done(me)
            end
        end,
    },
}

local desrtman_hook = h.simple_hook(DESRTMAN_KIND)
local flamman_hook = h.simple_hook(FLAMMAN_KIND)
local drillman_hook = h.simple_hook(DRILLMAN_KIND)
local metalman_hook = h.simple_hook(METALMAN_KIND)
local kingman_hook = h.simple_hook(KINGMAN_KIND, true)
local punk_hook = h.simple_hook(PUNK_KIND)

return {
    -- DesrtMan V1-V5 (0x0F7..0x0FB)
    desrtman = h.navi_chip({ "D" }, "null", "null", 120, 38, 2, desrtman_hook),
    desrtmnv2 = h.navi_chip({ "D" }, "null", "null", 140, 52, 3, desrtman_hook),
    desrtmnv3 = h.navi_chip({ "D" }, "null", "null", 160, 64, 4, desrtman_hook),
    desrtmnv4 = h.navi_chip({ "D" }, "null", "null", 180, 72, 4, desrtman_hook),
    desrtmnv5 = h.navi_chip({ "D" }, "null", "null", 210, 80, 4, desrtman_hook, "giga"),
    -- FlamMan V1-V5 (0x101..0x105)
    flamman = h.navi_chip({ "F" }, "fire", "fire", 120, 32, 2, flamman_hook),
    flammanv2 = h.navi_chip({ "F" }, "fire", "fire", 150, 52, 3, flamman_hook),
    flammanv3 = h.navi_chip({ "F" }, "fire", "fire", 180, 62, 4, flamman_hook),
    flammanv4 = h.navi_chip({ "F" }, "fire", "fire", 210, 72, 4, flamman_hook),
    flammanv5 = h.navi_chip({ "F" }, "fire", "fire", 240, 82, 4, flamman_hook, "giga"),
    -- DrillMan V1-V5 (0x106..0x10A)
    drillman = h.navi_chip({ "D" }, "null", "break", 70, 66, 2, drillman_hook),
    drillmnv2 = h.navi_chip({ "D" }, "null", "break", 90, 70, 3, drillman_hook),
    drillmnv3 = h.navi_chip({ "D" }, "null", "break", 110, 74, 4, drillman_hook),
    drillmnv4 = h.navi_chip({ "D" }, "null", "break", 130, 78, 4, drillman_hook),
    drillmnv5 = h.navi_chip({ "D" }, "null", "break", 150, 82, 4, drillman_hook, "giga"),
    -- MetalMan V1-V5 (0x10B..0x10F)
    metalman = h.navi_chip({ "M" }, "null", "break", 100, 40, 2, metalman_hook),
    metalmnv2 = h.navi_chip({ "M" }, "null", "break", 130, 60, 3, metalman_hook),
    metalmnv3 = h.navi_chip({ "M" }, "null", "break", 160, 70, 4, metalman_hook),
    metalmnv4 = h.navi_chip({ "M" }, "null", "break", 190, 80, 4, metalman_hook),
    metalmnv5 = h.navi_chip({ "M" }, "null", "break", 250, 90, 4, metalman_hook, "giga"),
    -- Punk (0x110)
    punk = h.navi_chip({ "P" }, "null", "break", 180, 92, 4, punk_hook),
    -- KingMan V1-V5 (0x115..0x119)
    kingman = h.navi_chip({ "K" }, "null", "break", 140, 36, 2, kingman_hook),
    kingmanv2 = h.navi_chip({ "K" }, "null", "break", 170, 48, 3, kingman_hook),
    kingmanv3 = h.navi_chip({ "K" }, "null", "break", 200, 60, 4, kingman_hook),
    kingmanv4 = h.navi_chip({ "K" }, "null", "break", 240, 72, 4, kingman_hook),
    kingmanv5 = h.navi_chip({ "K" }, "null", "break", 280, 84, 4, kingman_hook, "giga"),
    -- DarkMan V1-V5 (0x124..0x128)
    darkman = h.navi_chip({ "D" }, "null", "null", 30, 72, 2, darkman_hook(3)),
    darkmanv2 = h.navi_chip({ "D" }, "null", "null", 30, 76, 3, darkman_hook(4)),
    darkmanv3 = h.navi_chip({ "D" }, "null", "null", 30, 80, 4, darkman_hook(5)),
    darkmanv4 = h.navi_chip({ "D" }, "null", "null", 30, 84, 4, darkman_hook(6)),
    darkmanv5 = h.navi_chip({ "D" }, "null", "null", 30, 88, 4, darkman_hook(7), "giga"),
    -- JapanMan V1-V5 (0x129..0x12D)
    japanman = h.navi_chip({ "Y" }, "null", "null", 40, 82, 2, japanman_hook(4)),
    japanmnv2 = h.navi_chip({ "Y" }, "null", "null", 45, 85, 3, japanman_hook(5)),
    japanmnv3 = h.navi_chip({ "Y" }, "null", "null", 50, 88, 4, japanman_hook(6)),
    japanmnv4 = h.navi_chip({ "Y" }, "null", "null", 55, 91, 4, japanman_hook(7)),
    japanmnv5 = h.navi_chip({ "Y" }, "null", "null", 60, 94, 4, japanman_hook(8), "giga"),
}
