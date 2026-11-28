--!strict
-- EXE3 Non-Navi Mega Chips (15 chips):
--   guardian (0x0CB), zeushamr (0x0D0), godstone (0x0D1), oldwood (0x0D2),
--   meteors  (0x0D4), poltrgst (0x0D5), jealousy (0x0D6),
--   standout (0x0D7), salamndr (0x111),
--   watrline (0x0D8), fountain (0x112),
--   ligtning (0x0D9), bolt     (0x113),
--   gaiaswrd (0x0DA), gaiablad (0x114).

local anims = require("../lib/anims")
local attachment = require("../objects/attachment")
local chip_hand = require("@exelib/chip_hand")
local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local guardian_controller = require("@exelib/guardian/controller")
local guardian_statue = require("@exelib/guardian/statue")
local jealousy_controller = require("@exelib/jealousy/controller")
local regions = require("@exelib/regions")
local rising_bubble = require("../objects/rising_bubble")
local sparks = require("../lib/sparks")
local swords = require("../lib/swords")

local GUARDIAN_SPRITE = asset.sprite("guardian")
local ROCKCUBE_SPRITE = asset.sprite("rockcube")
local BIG_SLASH_SPRITE = asset.sprite("big-slash")
local ZEUSHAMR_ARM = attachment.look { sprite = asset.sprite("hammer"), palette = 1 }

local FLAME_FX = new.effect { sprite = asset.sprite("flame"), anim = 0, palette = 0, steps = 12 }
local HEAT_BLAST_FX = new.effect { sprite = asset.sprite("heat-blast"), anim = 0, palette = 0, steps = 26 }
local BUBBLE_SPLASH_FX = new.effect { sprite = asset.sprite("bubble-splash"), anim = 0, palette = 0, steps = 26 }
local LIGHTNING_FX = new.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0, steps = 12 }
local WOOD_VINES_FX = new.effect { sprite = asset.sprite("wood-vines"), anim = 0, palette = 0, steps = 27 }
local METEOR_FX = new.effect { sprite = asset.sprite("fireball"), anim = 0, palette = 0, steps = 15 }
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 18 }
local GROUND_CRACK_FX = new.effect { sprite = asset.sprite("ground-crack"), anim = 0, palette = 0, steps = 22 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local ARRIVAL_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local WOOD_SLASH_FX = new.effect { sprite = BIG_SLASH_SPRITE, anim = 0, palette = 0, steps = 15 }

local APPEAR_SOUND = asset.sound("appear")
local PLACE_SOUND = asset.sound("grab-shot-land")
local BLAST_SOUND = asset.sound("hit-bomb")
local CRACK_SOUND = asset.sound("panel-crack")
local THROW_SOUND = asset.sound("obstacle-throw")
local LIFT_SOUND = asset.sound("obstacle-lift")
local SPREADER_SOUND = asset.sound("spreader")
local BONUS_SOUND = asset.sound("damage-bonus")
local BUBBLE_SOUND = asset.sound("bubble")

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

local function find_hole_panel(user: Object): (number, number)
    local fx = user.panel_x + user.facing
    local fy = user.panel_y
    if field.valid(fx, fy) then
        local p = field.panel(fx, fy)
        if p and (p.kind == "broken" or p.kind == "missing") then
            return fx, fy
        end
    end
    for x = 1, 6 do
        for y = 1, 3 do
            local p = field.panel(x, y)
            if p and p.alliance == user.alliance and (p.kind == "broken" or p.kind == "missing") then
                return x, y
            end
        end
    end
    if field.valid(fx, fy) then
        return fx, fy
    end
    return user.panel_x, user.panel_y
end

-- =============================================================================
-- 1. Guardian (0x0CB): Summons Guardian statue; if hit, strikes attacker for 200!
-- =============================================================================
type GuardianStatueFlag = { strike: boolean }

local GUARDIAN_STRIKE_KIND = new.kind {
    id = "mega/guardian-strike",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Guardian",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local statue = me.related1
                if statue then
                    (statue.state :: GuardianStatueFlag).strike = true
                end
                me.timer = 48
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t <= 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local GUARDIAN_STRIKE = {
    telop = {} :: DimmingTelop,
    spawn = function(statue: Object): Object?
        local o = battle.spawn(GUARDIAN_STRIKE_KIND, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = statue.panel_x, statue.panel_y
        o.element = statue.element
        o.related1 = statue
        o.alliance = statue.alliance
        o.damage, o.stamina = statue.damage, statue.stamina
        return o
    end,
}

local function guardian_return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

local GUARDIAN_STATUE = guardian_statue.make {
    id = "mega/guardian-statue",
    sprite = GUARDIAN_SPRITE,
    hp = 10,
    sounds = { place = PLACE_SOUND, strike = BLAST_SOUND },
    arrival = ARRIVAL_FX,
    burst = LIGHTNING_FX,
    is = collision.navi,
    hits = collision.navi_target,
    strike_is = collision.piercing_break,
    strike_hits = collision.hits_navis,
    strike = GUARDIAN_STRIKE,
    effects = { puff = PUFF_FX },
    bubble = rising_bubble.kind,
    reads_no_removal = true,
    untracked = true,
    strikes_on_any_hit = true,
    keeps_region_striking = true,
    actions = {
        [1] = guardian_return_to_idle,
        [3] = function(_me: Object) end,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
    },
}

local GUARDIAN_CTRL = guardian_controller.make {
    id = "mega/guardian-controller",
    statue = GUARDIAN_STATUE.spawn,
}

local guardian: Chip = {
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 82,
    damage = 200,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = GUARDIAN_CTRL.hook,
}

-- =============================================================================
-- 2. ZeusHamr (0x0D0): Dim 250 breaking damage to ALL grounded Navis on field!
-- =============================================================================
local ZEUSHAMR_KIND = new.kind {
    id = "mega/zeushamr-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "ZeusHamr",
        effect = function(me: Object)
            local user = me.related1 or battle.player(me.alliance)
            if me.phase_init == 0 then
                if user then
                    user:set_animation(anims.HAMMER)
                    attachment.attach(user, ZEUSHAMR_ARM, "related", 0, true)
                end
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 14, true, true)
                me.timer = 0
                me.phase_init = 4
            end
            if user then
                user:update_sprite_while_dimmed()
            end
            if me.timer == 14 then
                -- Strike all grounded enemies across the entire field via breaking hitbox.
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0,
                    region = regions.field,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                -- ZeusHamr also strikes the user if grounded (not floating/invisible).
                if user and not user:status("dead") and not user:status("invisible") then
                    local stats = battle.navi(me.alliance)
                    if not stats.float_shoes and not stats.air_shoes then
                        local hit_dmg = me.damage
                        local p = field.panel(user.panel_x, user.panel_y)
                        if p and p.kind == "holy" then
                            hit_dmg = math.floor(hit_dmg / 2)
                        end
                        user.hp = math.max(0, user.hp - hit_dmg)
                        battle.effect(user.pos, IMPACT_FX)
                    end
                end
                battle.region_effects(me.panel_x, me.panel_y, regions.field, me.alliance, GROUND_CRACK_FX, 0x08_0000)
                battle.play_sound(BLAST_SOUND)
                battle.play_sound(CRACK_SOUND)
                battle.shake_camera(3, 22)
            end
            me.timer += 1
            if me.timer >= 42 then
                if user then
                    user.related1 = nil
                    user:set_animation(anims.idle(user))
                end
                dimming_chips.done(me)
            end
        end,
    },
}

local zeushamr: Chip = {
    codes = { "Z" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 4,
    mb = 90,
    damage = 250,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(ZEUSHAMR_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 3. GodStone (0x0D1): Summons GodStone on a hole panel, firing rapid meteors!
-- =============================================================================
type TurretState = { shots: number, tick: number, is_wood: boolean }

local HOLE_TURRET_KIND = new.kind {
    id = "mega/hole-turret",
    pool = "attack",
    state = { shots = "u8", tick = "u16", is_wood = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(ROCKCUBE_SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 9, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: TurretState
            s.shots = 0
            s.tick = 0
            battle.play_sound(PLACE_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: TurretState
            local max_shots = if s.is_wood then 6 else 5
            if s.tick % 16 == 8 and s.shots < max_shots then
                s.shots += 1
                local tx, ty = enemy_target(me.alliance)
                if s.shots % 2 == 0 then
                    local rx, ry = regions.random_panel(me.panel_x, me.panel_y, regions.other_area[me.alliance], me.facing, 0, 0x8000)
                    if rx and ry then
                        tx, ty = rx, ry
                    end
                end
                local last = s.shots == max_shots
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = if s.is_wood then 4 else 0,
                    z = 0x10_0000,
                    region = if s.is_wood then regions.plus else regions.single,
                    hit_effect = if s.is_wood then sparks.wood else sparks.breaking,
                    target = collision.hits_navis,
                    self_type = if s.is_wood then collision.attack else collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), if s.is_wood then WOOD_VINES_FX else METEOR_FX)
                battle.play_sound(if s.is_wood then SPREADER_SOUND else BLAST_SOUND)
                if not s.is_wood and field.valid(tx, ty) then
                    field.crack(tx, ty)
                end
            end
            s.tick += 1
            if s.tick >= max_shots * 16 + 20 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type TurretCtrlState = { is_wood: boolean }

local HOLE_TURRET_CTRL = new.kind {
    id = "mega/hole-turret-controller",
    pool = "effect",
    state = { is_wood = "bool" },
    update = dimming_chips.phases {
        name = "HoleSummon",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1 or battle.player(me.alliance)
                local s = me.state :: TurretCtrlState
                if user then
                    local hx, hy = find_hole_panel(user)
                    local o = battle.spawn(HOLE_TURRET_KIND)
                    if o then
                        o.panel_x, o.panel_y = hx, hy
                        o.alliance, o.flip = user.alliance, user.flip
                        o.element = me.element
                        o:set_damage_word(dimming_chips.damage_plus(me, 0))
                        ;(o.state :: TurretState).is_wood = s.is_wood
                    end
                end
                battle.play_sound(APPEAR_SOUND)
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

local function hole_summon_hook(is_wood: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(HOLE_TURRET_CTRL, user, spec)
        if o then
            (o.state :: TurretCtrlState).is_wood = is_wood
        end
        return o
    end
end

local godstone: Chip = {
    codes = { "S" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = hole_summon_hook(false),
}

-- =============================================================================
-- 4. OldWood (0x0D2): Summons OldWood on a hole panel, raining wood shots!
-- =============================================================================
local oldwood: Chip = {
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = hole_summon_hook(true),
}

-- =============================================================================
-- 5. Meteors (0x0D4): Dim Fire meteor rain falling across enemy panels!
-- =============================================================================
type CountState = { hits: number }

local METEORS_KIND = new.kind {
    id = "mega/meteors-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "Meteors",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 4 == 2 and s.hits < 18 then
                s.hits += 1
                local region = regions.other_area[me.alliance]
                local rx, ry = regions.random_panel(me.panel_x, me.panel_y, region, me.facing, 0, 0x8000)
                local tx, ty: number, number
                if rx and ry then
                    tx, ty = rx, ry
                else
                    tx, ty = enemy_target(me.alliance)
                end
                local last = s.hits == 18
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 1,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.fire,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), if s.hits % 2 == 0 then METEOR_FX else BLAST_FX)
                if s.hits % 3 == 0 and field.valid(tx, ty) then
                    field.crack(tx, ty)
                end
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(1, 4)
            end
            me.timer += 1
            if me.timer >= 84 then
                dimming_chips.done(me)
            end
        end,
    },
}

local meteors: Chip = {
    codes = { "R" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 86,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(METEORS_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 6. Poltrgst (0x0D5): Dim throws every obstacle on field at opponent (150 each)!
-- =============================================================================
type PoltState = { thrown: number, pending: number }

local POLTRGST_KIND = new.kind {
    id = "mega/poltrgst-controller",
    pool = "effect",
    state = { thrown = "u8", pending = "u8" },
    update = dimming_chips.phases {
        name = "Poltrgst",
        effect = function(me: Object)
            local s = me.state :: PoltState
            if me.phase_init == 0 then
                battle.play_sound(LIFT_SOUND)
                local objs = field.all_objects(me.alliance)
                local count = 0
                for _, obj in objs do
                    if obj.hp > 0 then
                        count += 1
                        battle.effect(obj.pos, PUFF_FX)
                        obj.hp = 0
                        obj:set_lifecycle("destroy")
                    end
                end
                s.pending = count
                s.thrown = 0
                me.timer = 0
                me.phase_init = 4
            end
            if s.thrown < s.pending and me.timer % 8 == 6 then
                s.thrown += 1
                local tx, ty = enemy_target(me.alliance)
                local last = s.thrown == s.pending
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.thrown_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BLAST_FX)
                battle.play_sound(THROW_SOUND)
                battle.shake_camera(2, 8)
            end
            me.timer += 1
            if me.timer >= math.max(24, s.pending * 8 + 16) then
                dimming_chips.done(me)
            end
        end,
    },
}

local poltrgst: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 58,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(POLTRGST_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 7. Jealousy (0x0D6): Dim 100 dmg x number of chips left in opponent's hand!
-- =============================================================================
local function jealousy_count(me: Object): number
    local opp = battle.player(bit32.bxor(me.alliance, 1))
    if not opp then
        return 0
    end
    return chip_hand.left(opp.alliance)
end

local jealousy: Chip = {
    codes = { "J" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 64,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = jealousy_controller.make {
        id = "mega/jealousy-controller",
        count = jealousy_count,
        is = collision.piercing,
        hits = collision.hits_navis,
        spark = sparks.heavy,
    },
}

-- =============================================================================
-- 8 & 9. StandOut (0x0D7) / Salamndr (0x111): Fire pillar erupting across enemy row/area!
-- =============================================================================
type ElemState = { upgraded: boolean, step: number }

local FIRE_PILLAR_KIND = new.kind {
    id = "mega/fire-pillar-controller",
    pool = "effect",
    state = { upgraded = "bool", step = "u8" },
    update = dimming_chips.phases {
        name = "FirePillar",
        effect = function(me: Object)
            local s = me.state :: ElemState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local tx = me.panel_x + me.facing
                local ty = me.panel_y
                if not field.valid(tx, ty) then
                    tx, ty = enemy_target(me.alliance)
                end
                local region = if s.upgraded then regions.deep_fan else regions.row
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 1,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.fire,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(tx, ty, region, me.alliance, if s.upgraded then HEAT_BLAST_FX else FLAME_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 14)
            end
            me.timer += 1
            if me.timer >= 34 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function elem_hook(kind: Kind, upgraded: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            (o.state :: ElemState).upgraded = upgraded
        end
        return o
    end
end

local standout: Chip = {
    codes = { "P", "*" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 2,
    mb = 22,
    damage = 220,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(FIRE_PILLAR_KIND, false),
}

local salamndr: Chip = {
    codes = { "S", "*" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 50,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(FIRE_PILLAR_KIND, true),
}

-- =============================================================================
-- 10 & 11. WatrLine (0x0D8) / Fountain (0x112): Aqua jet sweeping forward & back!
-- =============================================================================
local AQUA_JET_KIND = new.kind {
    id = "mega/aqua-jet-controller",
    pool = "effect",
    state = { upgraded = "bool", step = "u8" },
    update = dimming_chips.phases {
        name = "AquaJet",
        effect = function(me: Object)
            local s = me.state :: ElemState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer % 5 == 2 and s.step < 6 then
                s.step += 1
                -- Sweeps forward across panels then rebounds!
                local dist = if s.step <= 3 then s.step else (7 - s.step)
                local tx = me.panel_x + me.facing * dist
                local ty = me.panel_y
                if field.valid(tx, ty) then
                    local region = if s.upgraded then regions.wide else regions.single
                    dim_hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = 2,
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.aqua,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    battle.region_effects(tx, ty, region, me.alliance, BUBBLE_SPLASH_FX, 0x10_0000)
                    battle.play_sound(BUBBLE_SOUND)
                end
            end
            me.timer += 1
            if me.timer >= 40 then
                dimming_chips.done(me)
            end
        end,
    },
}

local watrline: Chip = {
    codes = { "C", "*" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 2,
    mb = 18,
    damage = 180,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(AQUA_JET_KIND, false),
}

local fountain: Chip = {
    codes = { "D", "*" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 2,
    mb = 48,
    damage = 240,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(AQUA_JET_KIND, true),
}

-- =============================================================================
-- 12 & 13. Ligtning (0x0D9) / Bolt (0x113): Elec strike hitting enemy & adjacent panels!
-- =============================================================================
local ELEC_BOLT_KIND = new.kind {
    id = "mega/elec-bolt-controller",
    pool = "effect",
    state = { upgraded = "bool", step = "u8" },
    update = dimming_chips.phases {
        name = "ElecBolt",
        effect = function(me: Object)
            local s = me.state :: ElemState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 10, true, true)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local tx, ty = enemy_target(me.alliance)
                local region = if s.upgraded then regions.square else regions.plus
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 3,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.elec,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(tx, ty, region, me.alliance, LIGHTNING_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 12)
            end
            me.timer += 1
            if me.timer >= 34 then
                dimming_chips.done(me)
            end
        end,
    },
}

local ligtning: Chip = {
    codes = { "L", "*" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 30,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(ELEC_BOLT_KIND, false),
}

local bolt: Chip = {
    codes = { "T", "*" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 52,
    damage = 210,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = elem_hook(ELEC_BOLT_KIND, true),
}

-- =============================================================================
-- 14 & 15. GaiaSwrd (0x0DA) / GaiaBlad (0x114):
-- Absorbs following chips' attack power in hand into a massive Wood slash!
-- =============================================================================
type GaiaState = { timer: number, total_dmg: number }

local function make_gaia_action(id: string, region: Region): Action
    return new.action {
        id = id,
        state = { timer = "u16", total_dmg = "u16" },
        update = function(me: Object, s: GaiaState)
            if me.step_init == 0 then
                me.anim = anims.SWORD_SWING
                me:set_status("using_action", true)
                attachment.attach(me, swords.wood_blade, "related", 0)
                local base = me.attack_damage + me.extra
                local absorbed = 0
                local side = me.alliance
                for _ = 1, 5 do
                    local cur = battle.hand_cursor(side)
                    local c = chip_hand.chip(side, cur)
                    if c == nil then
                        break
                    end
                    local d = (battle.hand_entry(side, cur, "damage") :: number) or 0
                    local ab = (battle.hand_entry(side, cur, "attack_bonus") :: number) or 0
                    local cb = (battle.hand_entry(side, cur, "charge_bonus") :: number) or 0
                    if chip_hand.damages(side, cur) and d < 9999 then
                        absorbed += d + ab + cb
                    end
                    battle.advance_hand(side)
                end
                if absorbed > 0 then
                    battle.play_sound(BONUS_SOUND)
                end
                s.total_dmg = math.clamp(base + absorbed, 0, 9999)
                s.timer = 0
                battle.play_sound(swords.big_swing)
                me.step_init = 4
            end
            if s.timer == 6 then
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = 4,
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.wood,
                        target = collision.hits_navis,
                        self_type = collision.slash,
                        damage = s.total_dmg,
                        stamina = me.hit_param,
                        hit_mod = 3,
                    })
                    local cx, cy = field.center(tx, me.panel_y)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), WOOD_SLASH_FX, me.alliance)
                end
            end
            s.timer += 1
            if s.timer >= 20 then
                me.related1 = nil
                me:exit_attack()
            end
        end,
    }
end

local gaiaswrd: Chip = {
    action = make_gaia_action("gaiaswrd/action", regions.wide),
    codes = { "G", "*" },
    element = "wood",
    family = "sword",
    class = "mega",
    rarity = 2,
    mb = 28,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "library" },
}

local gaiablad: Chip = {
    action = make_gaia_action("gaiablad/action", regions.wide_long),
    codes = { "G", "*" },
    element = "wood",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 55,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return {
    guardian = guardian,
    zeushamr = zeushamr,
    godstone = godstone,
    oldwood = oldwood,
    meteors = meteors,
    poltrgst = poltrgst,
    jealousy = jealousy,
    standout = standout,
    salamndr = salamndr,
    watrline = watrline,
    fountain = fountain,
    ligtning = ligtning,
    bolt = bolt,
    gaiaswrd = gaiaswrd,
    gaiablad = gaiablad,
}
