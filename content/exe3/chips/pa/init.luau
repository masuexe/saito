--!strict
-- EXE3 Core PvP Program Advances (11 PAs):
--   z-canon1, z-canon2, z-canon3 (timed multi-shot Z-Cannons!)
--   lifeswrd (Sword + WideSwrd + LongSwrd)
--   2xhero   (CustSwrd + VarSwrd + ProtoMan)
--   bodygrd  (AntiDmg + AntiNavi + Muramasa)
--   poisphar (PoisMask + PoisFace + Anubis)
--   heatsprd (HeatShot + Heat-V + HeatSide)
--   bubsprd  (Bubbler + Bub-V + BublSide)
--   mstrstyl (FireSwrd + AquaSwrd + ElecSwrd + BambSwrd)
--   500barr  (Barrier + Barr100 + Barr200)

local antis = require("./anti")
local attachment = require("../objects/attachment")
local barrier_chips = require("./barrier")
local barrier_visual = require("../objects/barrier_visual")
local barriers = require("../lib/barriers")
local blades = require("./sword")
local bullet = require("../objects/bullet")
local cannons = require("./cannon")
local collision = require("../rules/collision")
local common_visual = require("@exelib/barriers/visual")
local dimming_chips = require("@exelib/dimming")
local navis = require("./navis")
local poisons = require("./poison")
local projectile = require("../objects/projectile")
local regions = require("@exelib/regions")
local shuriken = require("../objects/shuriken")
local sparks = require("../lib/sparks")
local spreaders = require("./spreader")
local swords = require("../lib/swords")

local cannon_sprite = asset.sprite("cannon")
local big_slash_sprite = asset.sprite("big-slash")
local hit_sparks_sprite = asset.sprite("hit-sparks")
local puff_sprite = asset.sprite("puff")

local Z_CANON_ARM = attachment.look { sprite = cannon_sprite, palette = 0 }
local Z_CANON_SHOT = projectile.variant {
    id = "pa/z-canon-shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.blast,
}

local CANNON_SOUND = asset.sound("cannon")
local APPEAR_SOUND = asset.sound("appear")
local SWING_SOUND = asset.sound("big-sword-swing")
local POISON_SOUND = asset.sound("panel-poison")
local BARRIER_SOUND = asset.sound("barrier")
local SPREADER_SOUND = asset.sound("spreader")

local BIG_SLASH = new.effect { sprite = big_slash_sprite, anim = 0, palette = 0, steps = 15 }
local FIRE_FX = new.effect { sprite = hit_sparks_sprite, anim = 2, palette = 0, steps = 16 }
local AQUA_FX = new.effect { sprite = hit_sparks_sprite, anim = 3, palette = 0, steps = 16 }
local ELEC_FX = new.effect { sprite = hit_sparks_sprite, anim = 4, palette = 0, steps = 16 }
local WOOD_FX = new.effect { sprite = hit_sparks_sprite, anim = 5, palette = 0, steps = 16 }
local POISON_FX = new.effect { sprite = puff_sprite, anim = 0, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }

local SWORD_HIT = { hits = collision.hits_navis, is = collision.slash }

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
    return if side == 0 then 5 else 2, 2
end

-- =============================================================================
-- 1. Z-Canon1 / Z-Canon2 / Z-Canon3: Real timed multi-shot Z-Cannon!
-- Grants invisibility for 180 ticks and lets the user fire rapid Cannon shots
-- (fires on A press or rhythmic bursts every 24 ticks, up to 6 shots).
-- =============================================================================
type ZCanonState = { timer: number, cooldown: number, shots: number }

local function make_zcanon_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16", cooldown = "u8", shots = "u8" },
        update = function(me: Object, s: ZCanonState)
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:set_status("using_action", true)
                me:set_status_timer("flash", 180)
                me:set_status("invisible", true)
                attachment.attach(me, Z_CANON_ARM, "related", 0)
                s.timer = 180
                s.cooldown = 0
                s.shots = 0
                me.step_init = 4
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            elseif s.shots < 6 and (me:held("a") or s.timer % 26 == 0) then
                s.shots += 1
                s.cooldown = 16
                local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.play_sound(CANNON_SOUND)
                projectile.spawn(me, me.panel_x + me.facing, me.panel_y, {
                    variant = Z_CANON_SHOT,
                    damage = dmg,
                    z = 0x18_0000,
                })
            end
            s.timer -= 1
            if s.timer <= 0 or s.shots >= 6 and s.cooldown == 0 then
                me.related1 = nil
                me:exit_attack()
            end
        end,
    }
end

local z_canon1: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 0, code_run = { chip = cannons.cannon, count = 3 } },
    },
    action = make_zcanon_action("z-canon1/action"),
}

local z_canon2: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 1, code_run = { chip = cannons.hicannon, count = 3 } },
    },
    action = make_zcanon_action("z-canon2/action"),
}

local z_canon3: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 2, code_run = { chip = cannons["m-cannon"], count = 3 } },
    },
    action = make_zcanon_action("z-canon3/action"),
}

-- =============================================================================
-- 2. LifeSwrd (0x150): Sword + WideSwrd + LongSwrd
-- =============================================================================
local lifeswrd: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 400,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 3, sequence = { blades.sword, blades.wideswrd, blades.longswrd } },
    },
    action = swords.slash {
        id = "lifeswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = SWORD_HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
}

-- =============================================================================
-- 3. 2xHero (0x153): CustSwrd + VarSwrd + ProtoMan (dim 8-hit x 70 slash barrage!)
-- =============================================================================
type CountState = { hits: number }

local HERO2X_KIND = new.kind {
    id = "pa/2xhero-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "2xHero",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 6 == 2 and s.hits < 8 then
                s.hits += 1
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
                    hit_mod = if s.hits == 8 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH, me.alliance)
                battle.play_sound(SWING_SOUND)
            end
            me.timer += 1
            if me.timer >= 58 then
                dimming_chips.done(me)
            end
        end,
    },
}

local hero_2x: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 70,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 4, sequence = { blades.custswrd, blades.varswrd, navis.protoman } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(HERO2X_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 4. BodyGrd (0x158): AntiDmg + AntiNavi + Muramasa (12 diving shurikens x 100!)
-- =============================================================================
type StrikerState = { left: number, tick: number }

local BODYGRD_STRIKER = new.kind {
    id = "pa/bodygrd-striker",
    pool = "effect",
    state = { left = "u8", tick = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            local s = me.state :: StrikerState
            s.left = 12
            s.tick = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: StrikerState
            if s.tick == 0 and s.left > 0 then
                s.left -= 1
                s.tick = 16
                local tx, ty = shuriken.target_enemy(me)
                if tx and ty then
                    shuriken.spawn(me, tx, ty, 0, if s.left == 0 then 3 else 1, me.damage)
                end
            else
                s.tick -= 1
            end
            if s.left == 0 and s.tick == 0 then
                me:set_lifecycle("destroy")
            end
        else
            me:destroy()
        end
    end,
}

local BODYGRD_CTRL = new.kind {
    id = "pa/bodygrd-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "BodyGrd",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local o = battle.spawn(BODYGRD_STRIKER)
                if o then
                    o.alliance, o.flip = me.alliance, me.flip
                    o:set_damage_word(me.damage)
                end
                battle.play_sound(APPEAR_SOUND)
                me.timer = 18
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

local bodygrd: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 5, sequence = { antis.antidmg, antis.antinavi, blades.muramasa } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BODYGRD_CTRL, user, spec, true)
    end,
}

-- =============================================================================
-- 5. PoisPhar (0x157): PoisMask + PoisFace + Anubis (Pharaoh rapid poison drain!)
-- =============================================================================
type PharState = { life: number, tick: number }

local PHAR_STATUE = new.kind {
    id = "pa/poisphar-statue",
    pool = "attack",
    state = { life = "u16", tick = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("grab-shot"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: PharState
            s.life = 540
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PharState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick % 2 == 0 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and not opp:status("dead") and opp.hp > 0 then
                    opp.hp = math.max(0, opp.hp - 2)
                    if s.tick % 20 == 0 then
                        battle.effect(opp.pos, POISON_FX)
                        battle.play_sound(POISON_SOUND)
                    end
                end
            end
            s.tick += 1
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local POISPHAR_CTRL = new.kind {
    id = "pa/poisphar-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "PoisPhar",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                if user then
                    local tx = user.panel_x + user.facing
                    local ty = user.panel_y
                    if field.valid(tx, ty) then
                        local o = battle.spawn(PHAR_STATUE)
                        if o then
                            o.panel_x, o.panel_y = tx, ty
                            o.alliance, o.flip = user.alliance, user.flip
                        end
                    end
                end
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

local poisphar: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming" },
    program_advances = {
        { order = 6, sequence = { poisons.poismask, poisons.poisface, poisons.anubis } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(POISPHAR_CTRL, user, spec)
    end,
}

-- =============================================================================
-- 6 & 7. HeatSprd (0x14F) and BubSprd (0x14E): 300-dmg elemental 3x3 spreaders!
-- =============================================================================
local HEAT_BURST = bullet.variant { region = regions.square, hit_mod = 3, effect = FIRE_FX, sound = SPREADER_SOUND }
local BUB_BURST = bullet.variant { region = regions.square, hit_mod = 3, effect = AQUA_FX, sound = SPREADER_SOUND }

type SpreadState = { timer: number }

local function make_big_spreader(id: string, elem: number, burst: BulletVariant): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: SpreadState)
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 2 then
                local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.play_sound(SPREADER_SOUND)
                bullet.fire(me, me.panel_x + me.facing, me.panel_y, 0x10_0000, burst, elem, dmg)
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

local heatsprd: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 7, sequence = { spreaders.heatshot, spreaders["heat-v"], spreaders.heatside } },
    },
    action = make_big_spreader("heatsprd/action", 1, HEAT_BURST),
}

local bubsprd: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 8, sequence = { spreaders.bubbler, spreaders["bub-v"], spreaders.bublside } },
    },
    action = make_big_spreader("bubsprd/action", 2, BUB_BURST),
}

-- =============================================================================
-- 8. MstrStyl (0x15F): FireSwrd + AquaSwrd + ElecSwrd + BambSwrd (4-element barrage!)
-- =============================================================================
local MSTR_ELEMS = { 1, 2, 3, 4 }
local MSTR_SPARKS = { sparks.fire, sparks.aqua, sparks.elec, sparks.wood }
local MSTR_FX = { FIRE_FX, AQUA_FX, ELEC_FX, WOOD_FX }

local MSTRSTYL_KIND = new.kind {
    id = "pa/mstrstyl-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "MstrStyl",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 14, true, true)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 10 == 4 and s.hits < 4 then
                s.hits += 1
                local idx = s.hits
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = MSTR_ELEMS[idx],
                    z = 0x10_0000,
                    region = regions.wide_long,
                    hit_effect = MSTR_SPARKS[idx],
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if idx == 4 then 3 else 1,
                })
                battle.region_effects(tx, ty, regions.wide, me.alliance, MSTR_FX[idx], 0x10_0000)
                battle.play_sound(SWING_SOUND)
            end
            me.timer += 1
            if me.timer >= 52 then
                dimming_chips.done(me)
            end
        end,
    },
}

local mstrstyl: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 9, sequence = { blades.fireswrd, blades.aquaswrd, blades.elecswrd, blades.bambswrd } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(MSTRSTYL_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 9. 500Barr (0x159): Barrier + Barr100 + Barr200 -> raises 500-HP barrier (ID 4)!
-- =============================================================================
local BARR500_KIND = new.kind {
    id = "pa/500barr-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "500Barr",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1 or battle.player(me.alliance)
                if user then
                    user:raise_barrier {
                        id = 4,
                        behavior = "plain",
                        hp = 500,
                        threshold = 0,
                        timer = 0xFFFF,
                        weak_element = 0,
                    }
                    local old = user.barrier_visual
                    if old then
                        common_visual.finish(old)
                    end
                    barrier_visual.spawn(user, barriers.barrier_200)
                    battle.play_sound(BARRIER_SOUND)
                end
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

local barr_500: Chip = {
    codes = { "A", "B", "C", "D", "E", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming" },
    program_advances = {
        { order = 10, sequence = { barrier_chips.barrier, barrier_chips.barr100, barrier_chips.barr200 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BARR500_KIND, user, spec, true)
    end,
}

return {
    ["z-canon1"] = z_canon1,
    ["z-canon2"] = z_canon2,
    ["z-canon3"] = z_canon3,
    lifeswrd = lifeswrd,
    ["2xhero"] = hero_2x,
    bodygrd = bodygrd,
    poisphar = poisphar,
    heatsprd = heatsprd,
    bubsprd = bubsprd,
    mstrstyl = mstrstyl,
    ["500barr"] = barr_500,
}
