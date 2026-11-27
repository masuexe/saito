--!strict
-- EXE3 Iconic Navi Mega Chips (20 chips):
--   FlashMan V1-V4: dim screen-wide Elec attack inflicting paralyze!
--   PlantMan V1-V4: dim Wood vine 4-hit barrage binding/paralyzing target!
--   ProtoMan V1-V4: dim step-in WideSword slash right in front of opponent!
--   BowlMan  V1-V4: dim bowling pin sweep across all 3 rows!
--   MistMan  V1-V4: dim 3 mist apparitions striking opponent panels!

local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local HIT_SPARKS = asset.sprite("hit-sparks")
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 16 }
local BIG_SLASH_FX = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0, steps = 15 }
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
local MIST_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }

local APPEAR_SOUND = asset.sound("appear")
local SWING_SOUND = asset.sound("big-sword-swing")
local BLAST_SOUND = asset.sound("hit-bomb")
local SPREADER_SOUND = asset.sound("spreader")
local CRACK_SOUND = asset.sound("panel-crack")

local function dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

local function enemy_target(side: number): (number, number)
    local opp = battle.player(bit32.bxor(side, 1))
    if opp and not opp:status("dead") then
        return opp.panel_x, opp.panel_y
    end
    local actors = battle.alive_actors(bit32.bxor(side, 1))
    if #actors > 0 then
        return actors[1].panel_x, actors[1].panel_y
    end
    return if side == 0 then 5 else 2, 2
end

-- =============================================================================
-- 1. FlashMan (V1-V4): Screen-wide Elec flash + Paralyze!
-- =============================================================================
local FLASHMAN_KIND = new.kind {
    id = "navis/flashman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "FlashMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 18, true, true)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local region = regions.other_area[me.alliance]
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 3, -- elec
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.elec,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                    status = statuses.paralyze,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, ELEC_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 16)
            end
            me.timer += 1
            if me.timer >= 36 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function flashman_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(FLASHMAN_KIND, user, spec, true)
end

-- =============================================================================
-- 2. PlantMan (V1-V4): Wood vine 4-hit barrage binding/paralyzing target!
-- =============================================================================
type PlantState = { hits: number }

local PLANTMAN_KIND = new.kind {
    id = "navis/plantman-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "PlantMan",
        effect = function(me: Object)
            local s = me.state :: PlantState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 10 == 4 and s.hits < 4 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local last = s.hits == 4
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 4, -- wood
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.wood,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                    status = if last then statuses.paralyze else nil,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), WOOD_FX)
                battle.play_sound(SPREADER_SOUND)
            end
            me.timer += 1
            if me.timer >= 52 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function plantman_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(PLANTMAN_KIND, user, spec, true)
end

-- =============================================================================
-- 3. ProtoMan (V1-V4): Step-in WideSword slash right in front of opponent!
-- =============================================================================
local PROTOMAN_KIND = new.kind {
    id = "navis/protoman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "ProtoMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 12 then
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH_FX, me.alliance)
                battle.play_sound(SWING_SOUND)
            end
            me.timer += 1
            if me.timer >= 36 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function protoman_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(PROTOMAN_KIND, user, spec)
end

-- =============================================================================
-- 4. BowlMan (V1-V4): Bowling pin sweep across all 3 rows!
-- =============================================================================
type BowlState = { col_step: number }

local BOWLMAN_KIND = new.kind {
    id = "navis/bowlman-controller",
    pool = "effect",
    state = { col_step = "u8" },
    update = dimming_chips.phases {
        name = "BowlMan",
        effect = function(me: Object)
            local s = me.state :: BowlState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.col_step = 0
                me.phase_init = 4
            end
            if me.timer % 6 == 2 and s.col_step < 6 then
                local x = if me.alliance == 0 then (1 + s.col_step) else (6 - s.col_step)
                s.col_step += 1
                if field.valid(x, 2) then
                    dim_hitbox(me, {
                        panel_x = x,
                        panel_y = 2,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.wide,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    battle.region_effects(x, 2, regions.wide, me.alliance, BLAST_FX, 0x10_0000)
                    battle.play_sound(CRACK_SOUND)
                end
            end
            me.timer += 1
            if me.timer >= 46 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function bowlman_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(BOWLMAN_KIND, user, spec)
end

-- =============================================================================
-- 5. MistMan (V1-V4): 3 mist apparitions striking opponent panels!
-- =============================================================================
type MistState = { strikes: number }

local MISTMAN_KIND = new.kind {
    id = "navis/mistman-controller",
    pool = "effect",
    state = { strikes = "u8" },
    update = dimming_chips.phases {
        name = "MistMan",
        effect = function(me: Object)
            local s = me.state :: MistState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.strikes = 0
                me.phase_init = 4
            end
            if me.timer % 12 == 4 and s.strikes < 3 then
                s.strikes += 1
                local tx, ty = enemy_target(me.alliance)
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), PUFF_FX)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.plus,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.strikes == 3 then 3 else 1,
                })
                battle.effect(Vec3.new(cx, cy, 0x10_0000), MIST_FX)
                battle.play_sound(SPREADER_SOUND)
            end
            me.timer += 1
            if me.timer >= 48 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function mistman_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(MISTMAN_KIND, user, spec, true)
end

local function navi_chip(
    codes: { string },
    elem: string,
    fam: string,
    power: number,
    mb: number,
    rarity: number,
    hook: (user: Object, spec: DimmingChipSpec) -> Object?
): Chip
    return {
        codes = codes,
        element = elem,
        family = fam,
        class = "mega",
        rarity = rarity,
        mb = mb,
        damage = power,
        hit_param = 0,
        lockout = 0,
        flags = { "dimming", "has_damage", "navi", "library" },
        dimming = hook,
    }
end

return {
    -- FlashMan V1-V4 (0x0E8..0x0EB)
    flashman = navi_chip({ "F" }, "elec", "elec", 50, 39, 2, flashman_hook),
    flashmnv2 = navi_chip({ "F" }, "elec", "elec", 70, 59, 3, flashman_hook),
    flashmnv3 = navi_chip({ "F" }, "elec", "elec", 90, 69, 4, flashman_hook),
    flashmnv4 = navi_chip({ "F" }, "elec", "elec", 120, 79, 4, flashman_hook),
    -- PlantMan V1-V4 (0x0FC..0x0FF)
    plantman = navi_chip({ "P" }, "wood", "wood", 20, 60, 2, plantman_hook),
    plantmnv2 = navi_chip({ "P" }, "wood", "wood", 30, 65, 3, plantman_hook),
    plantmnv3 = navi_chip({ "P" }, "wood", "wood", 40, 70, 4, plantman_hook),
    plantmnv4 = navi_chip({ "P" }, "wood", "wood", 50, 75, 4, plantman_hook),
    -- ProtoMan V1-V4 (0x0E3..0x0E6)
    protoman = navi_chip({ "B" }, "null", "sword", 160, 68, 2, protoman_hook),
    protomnv2 = navi_chip({ "B" }, "null", "sword", 180, 76, 3, protoman_hook),
    protomnv3 = navi_chip({ "B" }, "null", "sword", 200, 82, 4, protoman_hook),
    protomnv4 = navi_chip({ "B" }, "null", "sword", 220, 90, 4, protoman_hook),
    -- BowlMan V1-V4 (0x11F..0x122)
    bowlman = navi_chip({ "B" }, "null", "break", 100, 80, 2, bowlman_hook),
    bowlmanv2 = navi_chip({ "B" }, "null", "break", 120, 82, 3, bowlman_hook),
    bowlmanv3 = navi_chip({ "B" }, "null", "break", 140, 84, 4, bowlman_hook),
    bowlmanv4 = navi_chip({ "B" }, "null", "break", 160, 86, 4, bowlman_hook),
    -- MistMan V1-V4 (0x11A..0x11D)
    mistman = navi_chip({ "M" }, "null", "null", 90, 68, 2, mistman_hook),
    mistmanv2 = navi_chip({ "M" }, "null", "null", 110, 71, 3, mistman_hook),
    mistmanv3 = navi_chip({ "M" }, "null", "null", 130, 74, 4, mistman_hook),
    mistmanv4 = navi_chip({ "M" }, "null", "null", 150, 77, 4, mistman_hook),
}
