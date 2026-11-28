--!strict
-- EXE3 Navi Chips Part 1 (43 chips):
--   FlashMan V1-V5, PlantMan V1-V5, ProtoMan V1-V5, BowlMan V1-V5, MistMan V1-V5,
--   Roll V1-V3, GutsMan V1-V5, BeastMan V1-V5, BubblMan V1-V5.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local h = require("./helpers")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

-- =============================================================================
-- 1. FlashMan (V1-V5): Appears (08-00), charges (anim 6), blinds & paralyzes!
--    Verified against A6BE 0x080B6790..0x080B69F0.
-- =============================================================================
local FLASHMAN_KIND = new.kind {
    id = "navis/flashman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "FlashMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                h.begin_navi(me, h.FLASHMAN_SPRITE, 0)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 14 then
                h.set_navi_anim(me, 6)
            elseif me.timer == 37 then
                battle.palette_flash(1, 24, true, true)
                battle.play_sound(h.FLASHMAN_FLASH_SOUND)
                local region = regions.other_area[me.alliance]
                h.dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 3,
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
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, h.ELEC_FX, 0x10_0000)
                battle.shake_camera(2, 16)
            elseif me.timer == 55 then
                h.set_navi_anim(me, 7)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 72 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 2. PlantMan (V1-V5): Appears (08-04) and summons 4-hit binding Wood vines!
-- =============================================================================
type CountState = { hits: number }

local PLANTMAN_KIND = new.kind {
    id = "navis/plantman-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "PlantMan",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                h.begin_navi(me, h.PLANTMAN_SPRITE, 0)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                h.set_navi_anim(me, 4)
            end
            if me.timer >= 14 and (me.timer - 14) % 10 == 0 and s.hits < 4 then
                s.hits += 1
                local tx, ty = h.enemy_target(me.alliance)
                local last = s.hits == 4
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 4,
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
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.WOOD_FX)
                battle.play_sound(h.SPREADER_SOUND)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 60 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 3. ProtoMan (V1-V5): Appears (08-09), warps before opponent, WideSword slash!
--    08-09 anim 5 = full purple WideSword swing (24t), anim 6 = follow-through.
-- =============================================================================
local PROTOMAN_KIND = new.kind {
    id = "navis/protoman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "ProtoMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                h.begin_navi(me, h.PROTOMAN_SPRITE, 0)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local tx, ty = h.enemy_target(me.alliance)
                local stand_x = tx - me.facing
                if field.valid(stand_x, ty) then
                    h.move_navi(me, stand_x, ty)
                else
                    h.move_navi(me, tx, ty)
                end
                h.set_navi_anim(me, 5)
            elseif me.timer == 18 then
                local tx, ty = h.enemy_target(me.alliance)
                h.dim_hitbox(me, {
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
                battle.region_effects(tx, ty, regions.wide, me.alliance, h.SLASH_FX, 0x10_0000)
                battle.play_sound(h.SWING_SOUND)
            elseif me.timer == 34 then
                h.set_navi_anim(me, 6)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 46 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 4. BowlMan (V1-V5): Appears (08-0e) and sweeps bowling pins across all 3 rows!
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
                h.begin_navi(me, h.BOWLMAN_SPRITE, 0)
                me.timer = 0
                s.col_step = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                h.set_navi_anim(me, 4)
            end
            if me.timer >= 14 and (me.timer - 14) % 6 == 0 and s.col_step < 6 then
                local x = if me.alliance == 0 then (1 + s.col_step) else (6 - s.col_step)
                s.col_step += 1
                if field.valid(x, 2) then
                    h.dim_hitbox(me, {
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
                    battle.region_effects(x, 2, regions.wide, me.alliance, h.BLAST_FX, 0x10_0000)
                    battle.play_sound(h.CRACK_SOUND)
                end
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 56 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 5. MistMan (V1-V5): Appears (10-44) and strikes opponent panels 3 times!
-- =============================================================================
local MISTMAN_KIND = new.kind {
    id = "navis/mistman-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "MistMan",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                h.begin_navi(me, h.MISTMAN_SPRITE, 0)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                h.set_navi_anim(me, 2)
            end
            if me.timer >= 14 and (me.timer - 14) % 12 == 0 and s.hits < 3 then
                s.hits += 1
                local tx, ty = h.enemy_target(me.alliance)
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.PUFF_FX)
                h.dim_hitbox(me, {
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
                    hit_mod = if s.hits == 3 then 3 else 1,
                })
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.BURST_FX)
                battle.play_sound(h.SPREADER_SOUND)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 56 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 6. Roll (V1-V3): Appears (10-14), strikes enemy 3x + heals user 60/100/140 HP!
-- =============================================================================
type RollState = { hits: number, heal_hp: number, healed: boolean }

local ROLL_KIND = new.kind {
    id = "navis/roll-controller",
    pool = "effect",
    state = { hits = "u8", heal_hp = "u16", healed = "bool" },
    update = dimming_chips.phases {
        name = "Roll",
        effect = function(me: Object)
            local s = me.state :: RollState
            if me.phase_init == 0 then
                h.begin_navi(me, h.ROLL_SPRITE, 0, true)
                me.timer = 0
                s.hits = 0
                s.healed = false
                me.phase_init = 4
            end
            if me.timer >= 12 and (me.timer - 12) % 8 == 0 and s.hits < 3 then
                s.hits += 1
                local tx, ty = h.enemy_target(me.alliance)
                local stand_x = tx - me.facing
                if field.valid(stand_x, ty) then
                    h.move_navi(me, stand_x, ty)
                else
                    h.move_navi(me, tx, ty)
                end
                h.set_navi_anim(me, 4)
                local cx, cy = field.center(tx, ty)
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
                    hit_mod = if s.hits == 3 then 3 else 1,
                })
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.FLASH_FX)
                battle.play_sound(h.SPREADER_SOUND)
            end
            if me.timer == 36 and not s.healed then
                s.healed = true
                local user = me.related1 or battle.player(me.alliance)
                if user and not user:status("dead") then
                    h.move_navi(me, user.panel_x, user.panel_y)
                    h.set_navi_anim(me, 5)
                    user.hp = math.min(user.max_hp, user.hp + s.heal_hp)
                    battle.effect(user.pos, h.HEAL_FX)
                    battle.play_sound(h.RECOVER_SOUND)
                end
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 54 then
                h.end_navi(me)
            end
        end,
    },
}

local function roll_hook(heal_hp: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(ROLL_KIND, user, spec, true)
        if o then
            (o.state :: RollState).heal_hp = heal_hp
        end
        return o
    end
end

-- =============================================================================
-- 7. GutsMan (V1-V5): Appears (08-08), raises fist (anim 2), slams ground (anim 6)!
-- =============================================================================
local GUTSMAN_KIND = new.kind {
    id = "navis/gutsman-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "GutsMan",
        effect = function(me: Object)
            if me.phase_init == 0 then
                h.begin_navi(me, h.GUTSMAN_SPRITE, 0)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                h.set_navi_anim(me, 2)
            elseif me.timer == 18 then
                h.set_navi_anim(me, 6)
                local opp_side = bit32.bxor(me.alliance, 1)
                local region = regions.other_area[me.alliance]
                h.dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                for x = 1, 6 do
                    for y = 1, 3 do
                        local p = field.panel(x, y)
                        if p and p.alliance == opp_side then
                            field.crack(x, y)
                        end
                    end
                end
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, h.IMPACT_FX, 0x08_0000)
                battle.play_sound(h.CRACK_SOUND)
                battle.play_sound(h.THROW_SOUND)
                battle.shake_camera(3, 18)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 48 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 8. BeastMan (V1-V5): Appears (08-01), lunges in front of enemy, 3 claw swipes
--    (anim 6) with BeastMan's own blue claw slash wave (anim 10)!
-- =============================================================================
local BEASTMAN_KIND = new.kind {
    id = "navis/beastman-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "BeastMan",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                h.begin_navi(me, h.BEASTMAN_SPRITE, 0)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local col = h.enemy_front_col(me.alliance)
                local stand_x = col - me.facing
                if field.valid(stand_x, 2) then
                    h.move_navi(me, stand_x, 2)
                else
                    h.move_navi(me, col, 2)
                end
                h.set_navi_anim(me, 5)
            end
            if me.timer >= 16 and (me.timer - 16) % 10 == 0 and s.hits < 3 then
                s.hits += 1
                local col = h.enemy_front_col(me.alliance)
                local tx, ty = h.enemy_target(me.alliance)
                local strike_x = if s.hits == 3 then tx else col
                local strike_y = if s.hits == 3 then ty else 2
                local stand_x = strike_x - me.facing
                if field.valid(stand_x, strike_y) then
                    h.move_navi(me, stand_x, strike_y)
                end
                h.set_navi_anim(me, 6)
                h.dim_hitbox(me, {
                    panel_x = strike_x,
                    panel_y = strike_y,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.wide_long,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == 3 then 3 else 1,
                })
                local cx, cy = field.center(strike_x, strike_y)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.BEAST_CLAW_FX, bit32.bxor(me.alliance, 1))
                battle.play_sound(if s.hits == 3 then h.SWING_SOUND else h.SWORD_SOUND)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= 50 then
                h.end_navi(me)
            end
        end,
    },
}

-- =============================================================================
-- 9. BubblMan (V1-V5): Appears (08-02), sprays Aqua bubbles scaled by low HP!
-- =============================================================================
type BubbleState = { shots: number, total: number }

local BUBBLMAN_KIND = new.kind {
    id = "navis/bubblman-controller",
    pool = "effect",
    state = { shots = "u8", total = "u8" },
    update = dimming_chips.phases {
        name = "BubblMan",
        effect = function(me: Object)
            local s = me.state :: BubbleState
            if me.phase_init == 0 then
                h.begin_navi(me, h.BUBBLMAN_SPRITE, 0)
                me.timer = 0
                s.shots = 0
                -- Scale extra bubbles as user's HP gets lower (classic EXE3 BubblMan!)
                local user = me.related1 or battle.player(me.alliance)
                local bonus = 0
                if user and user.max_hp > 0 then
                    local pct = math.floor((user.hp * 100) / user.max_hp)
                    if pct <= 25 then
                        bonus = 5
                    elseif pct <= 50 then
                        bonus = 3
                    elseif pct <= 75 then
                        bonus = 1
                    end
                end
                s.total = math.min(14, s.total + bonus)
                me.phase_init = 4
            end
            if me.timer == 10 then
                h.set_navi_anim(me, 3)
            end
            if me.timer >= 14 and (me.timer - 14) % 4 == 0 and s.shots < s.total then
                s.shots += 1
                local tx, ty = h.enemy_target(me.alliance)
                -- Alternate homing on target and spraying enemy area panels!
                if s.shots % 3 == 0 then
                    local rx, ry = regions.random_panel(me.panel_x, me.panel_y, regions.other_area[me.alliance], me.facing, 0, 0x8000)
                    if rx and ry then
                        tx, ty = rx, ry
                    end
                end
                local last = s.shots == s.total
                h.dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 2,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.aqua,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), h.BUBBLE_FX)
                battle.play_sound(if last then h.BUBBLE_POP_SOUND else h.BUBBLE_SOUND)
            end
            me:update_sprite_while_dimmed()
            me.timer += 1
            if me.timer >= s.total * 4 + 26 then
                h.end_navi(me)
            end
        end,
    },
}

local function bubblman_hook(base_shots: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(BUBBLMAN_KIND, user, spec, true)
        if o then
            (o.state :: BubbleState).total = base_shots
        end
        return o
    end
end

local flashman_hook = h.simple_hook(FLASHMAN_KIND, true)
local plantman_hook = h.simple_hook(PLANTMAN_KIND, true)
local protoman_hook = h.simple_hook(PROTOMAN_KIND)
local bowlman_hook = h.simple_hook(BOWLMAN_KIND)
local mistman_hook = h.simple_hook(MISTMAN_KIND, true)
local gutsman_hook = h.simple_hook(GUTSMAN_KIND, true)
local beastman_hook = h.simple_hook(BEASTMAN_KIND)

return {
    -- FlashMan V1-V5 (0x0E8..0x0EC)
    flashman = h.navi_chip({ "F" }, "elec", "elec", 50, 39, 2, flashman_hook),
    flashmnv2 = h.navi_chip({ "F" }, "elec", "elec", 70, 59, 3, flashman_hook),
    flashmnv3 = h.navi_chip({ "F" }, "elec", "elec", 90, 69, 4, flashman_hook),
    flashmnv4 = h.navi_chip({ "F" }, "elec", "elec", 120, 79, 4, flashman_hook),
    flashmnv5 = h.navi_chip({ "F" }, "elec", "elec", 150, 89, 4, flashman_hook, "giga"),
    -- PlantMan V1-V5 (0x0FC..0x100)
    plantman = h.navi_chip({ "P" }, "wood", "wood", 20, 60, 2, plantman_hook),
    plantmnv2 = h.navi_chip({ "P" }, "wood", "wood", 30, 65, 3, plantman_hook),
    plantmnv3 = h.navi_chip({ "P" }, "wood", "wood", 40, 70, 4, plantman_hook),
    plantmnv4 = h.navi_chip({ "P" }, "wood", "wood", 50, 75, 4, plantman_hook),
    plantmnv5 = h.navi_chip({ "P" }, "wood", "wood", 60, 80, 4, plantman_hook, "giga"),
    -- ProtoMan V1-V5 (0x0E3..0x0E7)
    protoman = h.navi_chip({ "B" }, "null", "sword", 160, 68, 2, protoman_hook),
    protomnv2 = h.navi_chip({ "B" }, "null", "sword", 180, 76, 3, protoman_hook),
    protomnv3 = h.navi_chip({ "B" }, "null", "sword", 200, 82, 4, protoman_hook),
    protomnv4 = h.navi_chip({ "B" }, "null", "sword", 220, 90, 4, protoman_hook),
    protomnv5 = h.navi_chip({ "B" }, "null", "sword", 240, 98, 4, protoman_hook, "giga"),
    -- BowlMan V1-V5 (0x11F..0x123)
    bowlman = h.navi_chip({ "B" }, "null", "break", 100, 80, 2, bowlman_hook),
    bowlmanv2 = h.navi_chip({ "B" }, "null", "break", 120, 82, 3, bowlman_hook),
    bowlmanv3 = h.navi_chip({ "B" }, "null", "break", 140, 84, 4, bowlman_hook),
    bowlmanv4 = h.navi_chip({ "B" }, "null", "break", 160, 86, 4, bowlman_hook),
    bowlmanv5 = h.navi_chip({ "B" }, "null", "break", 180, 88, 4, bowlman_hook, "giga"),
    -- MistMan V1-V5 (0x11A..0x11E)
    mistman = h.navi_chip({ "M" }, "null", "null", 90, 68, 2, mistman_hook),
    mistmanv2 = h.navi_chip({ "M" }, "null", "null", 110, 71, 3, mistman_hook),
    mistmanv3 = h.navi_chip({ "M" }, "null", "null", 130, 74, 4, mistman_hook),
    mistmanv4 = h.navi_chip({ "M" }, "null", "null", 150, 77, 4, mistman_hook),
    mistmanv5 = h.navi_chip({ "M" }, "null", "null", 170, 80, 4, mistman_hook, "giga"),
    -- Roll V1-V3 (0x0DB..0x0DD; A6BE 0x080B6F0C [15,25,40]*2 -> 30/50/80 HP heal)
    roll = h.navi_chip({ "R" }, "null", "recovery", 20, 10, 2, roll_hook(30)),
    rollv2 = h.navi_chip({ "R" }, "null", "recovery", 30, 22, 3, roll_hook(50)),
    rollv3 = h.navi_chip({ "R" }, "null", "recovery", 40, 34, 4, roll_hook(80)),
    -- GutsMan V1-V5 (0x0DE..0x0E2)
    gutsman = h.navi_chip({ "G" }, "null", "break", 50, 15, 2, gutsman_hook),
    gutsmanv2 = h.navi_chip({ "G" }, "null", "break", 70, 41, 3, gutsman_hook),
    gutsmanv3 = h.navi_chip({ "G" }, "null", "break", 90, 57, 4, gutsman_hook),
    gutsmanv4 = h.navi_chip({ "G" }, "null", "break", 100, 63, 4, gutsman_hook),
    gutsmanv5 = h.navi_chip({ "G" }, "null", "break", 120, 79, 4, gutsman_hook, "giga"),
    -- BeastMan V1-V5 (0x0ED..0x0F1)
    beastman = h.navi_chip({ "B" }, "null", "sword", 40, 38, 2, beastman_hook),
    beastmnv2 = h.navi_chip({ "B" }, "null", "sword", 50, 60, 3, beastman_hook),
    beastmnv3 = h.navi_chip({ "B" }, "null", "sword", 60, 68, 4, beastman_hook),
    beastmnv4 = h.navi_chip({ "B" }, "null", "sword", 70, 76, 4, beastman_hook),
    beastmnv5 = h.navi_chip({ "B" }, "null", "sword", 80, 84, 4, beastman_hook, "giga"),
    -- BubblMan V1-V5 (0x0F2..0x0F6)
    bubblman = h.navi_chip({ "B" }, "aqua", "aqua", 20, 50, 2, bubblman_hook(4)),
    bubblmnv2 = h.navi_chip({ "B" }, "aqua", "aqua", 20, 62, 3, bubblman_hook(5)),
    bubblmnv3 = h.navi_chip({ "B" }, "aqua", "aqua", 20, 74, 4, bubblman_hook(6)),
    bubblmnv4 = h.navi_chip({ "B" }, "aqua", "aqua", 20, 80, 4, bubblman_hook(7)),
    bubblmnv5 = h.navi_chip({ "B" }, "aqua", "aqua", 20, 86, 4, bubblman_hook(8), "giga"),
}
