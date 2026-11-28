--!strict
-- EXE3 Remaining Standard Utility, Elemental, Meteor, Totem, Counter, Team & Virus Chips (52 chips):
-- GrabBack, GrabRvng, NrthWind, Fanfare, Discord, Timpani, Panic,
-- RndmMetr, HoleMetr, ShotMetr, LavaCan1-3, Volcano, IceBall, LavaBall,
-- AirStrm1-3, Totem1-3, Spice1-3, CrsShld1-3, Geyser, Shake1-3, Condor,
-- Burning, FireRatn, RockArm1-3, Needler1-3, Team1, Team2,
-- Mettaur, Bunny, Spikey, Swordy, Jelly, Mushy, Momogra, KillrEye, Scuttlst.

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local HIT_SPARKS = asset.sprite("hit-sparks")
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
local FIRE_FX = new.effect { sprite = HIT_SPARKS, anim = 2, palette = 0, steps = 16 }
local AQUA_FX = new.effect { sprite = HIT_SPARKS, anim = 3, palette = 0, steps = 14 }
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 16 }
local WIND_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 10 }
local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 14 }
local SLASH_FX = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0, steps = 12 }
local HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }
local FLASH_FX = new.effect { sprite = asset.sprite("pink-flash"), anim = 1, palette = 0, steps = 10 }
local CONFUSE_FX = new.effect { sprite = asset.sprite("confusion"), anim = 0, palette = 0, steps = 16 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }

local APPEAR_SOUND = asset.sound("appear")
local BLAST_SOUND = asset.sound("hit-bomb")
local CANNON_SOUND = asset.sound("cannon")
local CRACK_SOUND = asset.sound("panel-crack")
local SHOT_SOUND = asset.sound("buster-shot")
local WIND_SOUND = asset.sound("spreader")
local SWING_SOUND = asset.sound("sword-swing")
local THROW_SOUND = asset.sound("minibomb-throw")
local FREEZE_SOUND = asset.sound("freeze")
local RECOVER_SOUND = asset.sound("sfx-recover")
local CONFUSE_SOUND = asset.sound("confusion")
local SHIELD_SOUND = asset.sound("shield")
local GUARD_SOUND = asset.sound("guard")
local BONUS_SOUND = asset.sound("damage-bonus")

local WIND_ELEM = element.byte("null", { "wind" })
local BREAK_ELEM = element.byte("null", { "break" })

local function dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

local function enemy_pos(side: number): (number, number)
    local opp = battle.player(bit32.bxor(side, 1))
    if opp and not opp:status("dead") then
        return opp.panel_x, opp.panel_y
    end
    return if side == 0 then 5 else 2, 2
end

type ActTimer = { timer: number }

-- =============================================================================
-- 1. GrabBack & GrabRvng: Deals per-panel damage for each stolen panel & reclaims!
-- =============================================================================
local GRAB_REVENGE_CTRL = new.kind {
    id = "utility/grab-revenge-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "GrabRvng",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local stolen = 0
                for py = 1, 3 do
                    for px = 1, 6 do
                        local p = field.panel(px, py)
                        if p and p.home == me.alliance and p.alliance ~= me.alliance then
                            stolen += 1
                            field.set_alliance(px, py, me.alliance)
                            local cx, cy = field.center(px, py)
                            battle.effect(Vec3.new(cx, cy, 0x08_0000), FLASH_FX)
                        end
                    end
                end
                if stolen > 0 then
                    local tx, ty = enemy_pos(me.alliance)
                    dim_hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.impact,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage * stolen,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    local cx, cy = field.center(tx, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), IMPACT_FX)
                    battle.play_sound(BLAST_SOUND)
                    battle.shake_camera(2, 12)
                else
                    battle.play_sound(APPEAR_SOUND)
                end
                me.timer = 20
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 2. NrthWind: Strips ALL barriers & auras from both sides!
-- =============================================================================
local NRTHWIND_CTRL = new.kind {
    id = "utility/nrthwind-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "NrthWind",
        effect = function(me: Object)
            if me.phase_init == 0 then
                for side = 0, 1 do
                    for _, actor in battle.alive_actors(side) do
                        if actor.has_collision and actor.collision.barrier == "up" then
                            actor.collision.barrier = "popped"
                            actor.collision.barrier_hp = 0
                            battle.effect(actor.pos, WIND_FX)
                        end
                    end
                end
                battle.region_effects(3, 2, regions.field, me.alliance, WIND_FX, 0x10_0000)
                battle.play_sound(WIND_SOUND)
                me.timer = 22
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 3. Instruments: Fanfare (invuln), Discord (confuse), Timpani (immobilize/paralyze)
-- =============================================================================
-- mode: 1 = fanfare, 2 = discord, 3 = timpani
type InstState = { mode: number }

local INSTRUMENT_CTRL = new.kind {
    id = "utility/instrument-controller",
    pool = "effect",
    state = { mode = "u8" },
    update = dimming_chips.phases {
        name = "Music",
        effect = function(me: Object)
            local s = me.state :: InstState
            if me.phase_init == 0 then
                if s.mode == 1 then
                    local user = me.related1
                    if user then
                        user:set_status("invulnerable", true)
                        user:set_status_timer("invulnerable", 300)
                        battle.effect(user.pos, HEAL_FX)
                    end
                    battle.play_sound(BONUS_SOUND)
                else
                    local opp = battle.player(bit32.bxor(me.alliance, 1))
                    if opp and not opp:status("dead") then
                        if s.mode == 2 then
                            opp:take_status(statuses.confuse)
                            battle.effect(opp.pos, CONFUSE_FX)
                            battle.play_sound(CONFUSE_SOUND)
                        else
                            opp:take_status(statuses.paralyze)
                            battle.effect(opp.pos, ELEC_FX)
                            battle.play_sound(APPEAR_SOUND)
                        end
                    end
                end
                me.timer = 22
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function inst_hook(mode: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(INSTRUMENT_CTRL, user, spec)
        if o then
            local st = o.state :: InstState
            st.mode = mode
        end
        return o
    end
end

-- =============================================================================
-- 4. Panic: Wave that confuses the opponent!
-- =============================================================================
local PANIC_ACTION = new.action {
    id = "panic/action",
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
                battle.hitbox(me, {
                    panel_x = sx,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.row,
                    hit_effect = sparks.plain,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = 0,
                    hit_mod = 0,
                    status = statuses.confuse,
                })
                battle.region_effects(sx, me.panel_y, regions.row, me.alliance, CONFUSE_FX, 0x10_0000)
                battle.play_sound(CONFUSE_SOUND)
            end
        end
        s.timer += 1
        if s.timer >= 18 then
            me:exit_attack()
        end
    end,
}

-- =============================================================================
-- 5. Meteor Barrages: RndmMetr (random), HoleMetr (breaks panels), ShotMetr (aimed)
-- =============================================================================
-- mode: 1 = random, 2 = hole (cracks/breaks), 3 = shot (targets opponent)
type MeteorState = { mode: number, drops: number, tick: number }

local METEOR_CTRL = new.kind {
    id = "utility/meteor-controller",
    pool = "effect",
    state = { mode = "u8", drops = "u8", tick = "u8" },
    update = dimming_chips.phases {
        name = "Meteor",
        effect = function(me: Object)
            local s = me.state :: MeteorState
            if me.phase_init == 0 then
                s.drops = 0
                s.tick = 0
                battle.play_sound(APPEAR_SOUND)
                me.phase_init = 4
            end
            if s.tick % 8 == 0 and s.drops < 6 then
                s.drops += 1
                local tx, ty: number, number
                if s.mode == 3 then
                    tx, ty = enemy_pos(me.alliance)
                else
                    local area = regions.other_area[me.alliance]
                    local rx, ry = regions.random_panel(me.panel_x, me.panel_y, area, me.facing, 0, 0x8000)
                    if rx and ry then
                        tx, ty = rx, ry
                    else
                        tx, ty = enemy_pos(me.alliance)
                    end
                end
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 1, -- fire
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.drops == 6 then 3 else 1,
                })
                if s.mode == 2 then
                    field.break_panel(tx, ty)
                end
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BLAST_FX)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(1, 6)
            end
            s.tick += 1
            if s.drops >= 6 and s.tick >= 54 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function meteor_hook(mode: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(METEOR_CTRL, user, spec)
        if o then
            local st = o.state :: MeteorState
            st.mode = mode
        end
        return o
    end
end

-- =============================================================================
-- 6. LavaCan1-3, Volcano, IceBall, LavaBall
-- =============================================================================
local function make_lavacan_action(id: string): Action
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
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                local tx, ty = me.panel_x + me.facing * 2, me.panel_y
                local opp_side = bit32.bxor(me.alliance, 1)
                for _, opp in battle.alive_actors(opp_side) do
                    if opp.panel_y == me.panel_y and (opp.panel_x - me.panel_x) * me.facing > 0 and not opp:status("dead") then
                        tx = opp.panel_x
                        break
                    end
                end
                if field.valid(tx, ty) then
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = 1, -- fire
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.fire,
                        target = collision.hits_navis,
                        self_type = collision.attack,
                        damage = damage,
                        hit_mod = 3,
                    })
                    field.set_type(tx, ty, "lava")
                    local cx, cy = field.center(tx, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), FIRE_FX)
                    battle.play_sound(CANNON_SOUND)
                end
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

local VOLCANO_ACTION = new.action {
    id = "volcano/action",
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
            local erupted = false
            for py = 1, 3 do
                for px = 1, 6 do
                    local p = field.panel(px, py)
                    if p and p.kind == "lava" then
                        erupted = true
                        battle.hitbox(me, {
                            panel_x = px,
                            panel_y = py,
                            element = 1, -- fire
                            z = 0x10_0000,
                            region = regions.plus,
                            hit_effect = sparks.fire,
                            target = collision.hits_navis,
                            self_type = collision.piercing,
                            damage = damage,
                            hit_mod = 3,
                        })
                        battle.region_effects(px, py, regions.plus, me.alliance, FIRE_FX, 0x10_0000)
                    end
                end
            end
            if not erupted then
                local tx, ty = enemy_pos(me.alliance)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 1,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.fire,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = damage,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), FIRE_FX)
            end
            battle.play_sound(BLAST_SOUND)
            battle.shake_camera(2, 12)
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local function make_elem_ball_action(id: string, elem: number, panel_type: PanelType): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                battle.play_sound(THROW_SOUND)
                me.step_init = 4
            end
            if s.timer == 10 then
                local tx = me.panel_x + me.facing * 3
                local ty = me.panel_y
                if field.valid(tx, ty) then
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = elem,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = if elem == 2 then sparks.aqua else sparks.fire,
                        target = collision.hits_navis,
                        self_type = collision.thrown,
                        damage = damage,
                        hit_mod = 3,
                        status = if elem == 2 then statuses.freeze else nil,
                    })
                    field.set_type(tx, ty, panel_type)
                    local cx, cy = field.center(tx, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), if elem == 2 then AQUA_FX else FIRE_FX)
                    battle.play_sound(if elem == 2 then FREEZE_SOUND else BLAST_SOUND)
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
-- 7. AirStrm1-3: Fires 3 rapid wind gusts down the row!
-- =============================================================================
local function make_airstrm_action(id: string): Action
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
            if s.timer == 4 or s.timer == 10 or s.timer == 16 then
                local sx = me.panel_x + me.facing
                if field.valid(sx, me.panel_y) then
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    battle.hitbox(me, {
                        panel_x = sx,
                        panel_y = me.panel_y,
                        element = WIND_ELEM,
                        z = 0x10_0000,
                        region = regions.row,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = damage,
                        hit_mod = if s.timer == 16 then 3 else 1,
                    })
                    battle.region_effects(sx, me.panel_y, regions.row, me.alliance, WIND_FX, 0x10_0000)
                    battle.play_sound(WIND_SOUND)
                end
            end
            s.timer += 1
            if s.timer >= 24 then
                me:exit_attack()
            end
        end,
    }
end

-- =============================================================================
-- 8. Totem1-3: Fire pillar that heals user and fires flames at opponent!
-- =============================================================================
type TotemState = { life: number, tick: number, heal_amt: number }

local TOTEM_KIND = new.kind {
    id = "utility/totem",
    pool = "attack",
    state = { life = "u16", tick = "u8", heal_amt = "u16" },
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
            me.anim, me.anim_loaded = 2, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: TotemState
            s.life = 360
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: TotemState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick == 20 or s.tick == 90 or s.tick == 160 then
                local owner = battle.player(me.alliance)
                if owner and not owner:status("dead") then
                    owner:heal(s.heal_amt, true)
                end
                local tx, ty = enemy_pos(me.alliance)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 1, -- fire
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.fire,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), FIRE_FX)
                battle.play_sound(BLAST_SOUND)
            end
            s.tick += 1
            if s.tick >= 180 then
                s.life = 1
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local function make_totem_action(id: string, heal_amt: number): Action
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
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    local o = battle.spawn(TOTEM_KIND)
                    if o then
                        o.panel_x, o.panel_y = tx, me.panel_y
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 1
                        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(damage)
                        ;(o.state :: TotemState).heal_amt = heal_amt
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
-- 9. Spice1-3: Explodes ALL Grass panels on the battlefield!
-- =============================================================================
local SPICE_CTRL = new.kind {
    id = "utility/spice-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Spice",
        effect = function(me: Object)
            if me.phase_init == 0 then
                for py = 1, 3 do
                    for px = 1, 6 do
                        local p = field.panel(px, py)
                        if p and p.kind == "grass" then
                            dim_hitbox(me, {
                                panel_x = px,
                                panel_y = py,
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
                            local cx, cy = field.center(px, py)
                            battle.effect(Vec3.new(cx, cy, 0x10_0000), WOOD_FX)
                        end
                    end
                end
                battle.play_sound(BLAST_SOUND)
                me.timer = 22
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function spice_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(SPICE_CTRL, user, spec)
end

-- =============================================================================
-- 10. CrsShld1-3: Guard shield counter throwing a crushing cross on block!
-- =============================================================================
type CrsState = { timer: number, countered: boolean }

local function make_crsshld_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16", countered = "bool" },
        update = function(me: Object, s: CrsState)
            if me.step_init == 0 then
                me:set_animation(anims.idle(me))
                me:set_status("guard", true)
                me:set_status("using_action", true)
                s.timer = 0
                s.countered = false
                battle.play_sound(SHIELD_SOUND)
                battle.effect(me.pos, HEAL_FX)
                me.step_init = 4
                return
            end
            if not s.countered and me.collision.guard_dirs ~= 0 then
                s.countered = true
                local tx, ty = enemy_pos(me.alliance)
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = BREAK_ELEM,
                    z = 0x10_0000,
                    region = regions.plus,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = damage,
                    hit_mod = 3,
                })
                battle.region_effects(tx, ty, regions.plus, me.alliance, IMPACT_FX, 0x10_0000)
                battle.play_sound(GUARD_SOUND)
                battle.play_sound(BLAST_SOUND)
            end
            s.timer += 1
            if s.timer >= 42 then
                me:set_status("guard", false)
                me:exit_attack()
            end
        end,
    }
end

-- =============================================================================
-- 11. Geyser, Shake1-3, Condor, Burning, FireRatn, RockArm1-3
-- =============================================================================
local GEYSER_ACTION = new.action {
    id = "geyser/action",
    state = { timer = "u16" },
    update = function(me: Object, s: ActTimer)
        if me.step_init == 0 then
            me:set_animation(anims.buster_shoot(me))
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == 8 then
            local tx = me.panel_x + me.facing * 3
            local ty = me.panel_y
            if field.valid(tx, ty) then
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 2, -- aqua
                    z = 0x10_0000,
                    region = regions.square,
                    hit_effect = sparks.aqua,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = damage,
                    hit_mod = 3,
                })
                battle.region_effects(tx, ty, regions.square, me.alliance, AQUA_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 12)
            end
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local function make_shake_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                battle.play_sound(THROW_SOUND)
                me.step_init = 4
            end
            if s.timer == 10 then
                local tx = me.panel_x + me.facing * 3
                local ty = me.panel_y
                if field.valid(tx, ty) then
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.region_1d,
                        hit_effect = sparks.impact,
                        target = collision.hits_navis,
                        self_type = collision.thrown,
                        damage = damage,
                        hit_mod = 3,
                    })
                    battle.region_effects(tx, ty, regions.region_1d, me.alliance, IMPACT_FX, 0x10_0000)
                    battle.play_sound(BLAST_SOUND)
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

local CONDOR_ACTION = new.action {
    id = "condor/action",
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
                battle.hitbox(me, {
                    panel_x = sx,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.deep_fan,
                    hit_effect = sparks.impact,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = damage,
                    hit_mod = 3,
                })
                battle.region_effects(sx, me.panel_y, regions.deep_fan, me.alliance, WIND_FX, 0x10_0000)
                battle.play_sound(WIND_SOUND)
            end
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local BURNING_ACTION = new.action {
    id = "burning/action",
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
                region = regions.region_2a,
                hit_effect = sparks.fire,
                target = collision.hits_navis,
                self_type = collision.piercing,
                damage = damage,
                hit_mod = 3,
            })
            battle.region_effects(me.panel_x, me.panel_y, regions.region_2a, me.alliance, FIRE_FX, 0x10_0000)
            battle.play_sound(BLAST_SOUND)
            battle.shake_camera(2, 12)
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local FIRERATN_ACTION = new.action {
    id = "fireratn/action",
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
            local tx, ty = enemy_pos(me.alliance)
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            battle.hitbox(me, {
                panel_x = tx,
                panel_y = ty,
                element = 1, -- fire blast rat
                z = 0x10_0000,
                region = regions.plus,
                hit_effect = sparks.blast,
                target = collision.hits_navis,
                self_type = collision.piercing,
                damage = damage,
                hit_mod = 3,
            })
            battle.region_effects(tx, ty, regions.plus, me.alliance, BLAST_FX, 0x10_0000)
            battle.play_sound(BLAST_SOUND)
            battle.shake_camera(2, 12)
        end
        s.timer += 1
        if s.timer >= 20 then
            me:exit_attack()
        end
    end,
}

local function make_rockarm_action(id: string): Action
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
                    battle.hitbox(me, {
                        panel_x = sx,
                        panel_y = me.panel_y,
                        element = BREAK_ELEM,
                        z = 0x10_0000,
                        region = regions.long,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = damage,
                        hit_mod = 3,
                    })
                    battle.region_effects(sx, me.panel_y, regions.long, me.alliance, IMPACT_FX, 0x10_0000)
                    field.break_panel(sx, me.panel_y)
                    battle.play_sound(CRACK_SOUND)
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
-- 12. Needler1-3 & Team1 / Team2
-- =============================================================================
type NeedlerState = { bursts: number, tick: number }

local NEEDLER_CTRL = new.kind {
    id = "utility/needler-controller",
    pool = "effect",
    state = { bursts = "u8", tick = "u8" },
    update = dimming_chips.phases {
        name = "Needler",
        effect = function(me: Object)
            local s = me.state :: NeedlerState
            if me.phase_init == 0 then
                s.bursts = 0
                s.tick = 0
                battle.play_sound(APPEAR_SOUND)
                me.phase_init = 4
            end
            if s.tick % 5 == 0 and s.bursts < 5 then
                s.bursts += 1
                local tx, ty = enemy_pos(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.plain,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.bursts == 5 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), FLASH_FX)
                battle.play_sound(SHOT_SOUND)
            end
            s.tick += 1
            if s.bursts >= 5 and s.tick >= 30 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function needler_hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(NEEDLER_CTRL, user, spec)
end

type TeamState = { mode: number }

local TEAM_CTRL = new.kind {
    id = "utility/team-controller",
    pool = "effect",
    state = { mode = "u8" },
    update = dimming_chips.phases {
        name = "Team",
        effect = function(me: Object)
            local s = me.state :: TeamState
            if me.phase_init == 0 then
                local user = me.related1
                if s.mode == 1 then
                    -- Team1: allied Mettaur Guard sends a quick shockwave & shields user!
                    local tx, ty = enemy_pos(me.alliance)
                    dim_hitbox(me, {
                        panel_x = tx,
                        panel_y = ty,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = math.max(10, me.damage),
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    local cx, cy = field.center(tx, ty)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), IMPACT_FX)
                    battle.play_sound(CRACK_SOUND)
                else
                    -- Team2: allied support restores HP & grants brief invulnerability!
                    if user then
                        user:heal(50, true)
                        user:set_status("invulnerable", true)
                        user:set_status_timer("invulnerable", 120)
                        battle.effect(user.pos, HEAL_FX)
                    end
                    battle.play_sound(RECOVER_SOUND)
                end
                me.timer = 20
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function team_hook(mode: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(TEAM_CTRL, user, spec)
        if o then
            local st = o.state :: TeamState
            st.mode = mode
        end
        return o
    end
end

-- =============================================================================
-- 13. Virus-Breeder Standard Chips (9 chips -> completes 200/200 Standard Library!):
-- Mettaur, Bunny, Spikey, Swordy, Jelly, Mushy, Momogra, KillrEye, Scuttlst.
-- =============================================================================
-- kind: 1=Mettaur, 2=Bunny, 3=Spikey, 4=Swordy, 5=Jelly, 6=Mushy, 7=Momogra, 8=KillrEye, 9=Scuttlst
type VirusState = { vkind: number }

local VIRUS_CTRL = new.kind {
    id = "utility/virus-controller",
    pool = "effect",
    state = { vkind = "u8" },
    update = dimming_chips.phases {
        name = "Virus",
        effect = function(me: Object)
            local s = me.state :: VirusState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 8 then
                local tx, ty = enemy_pos(me.alliance)
                local vk = s.vkind
                local elem = if vk == 2 or vk == 8
                    then 3
                    elseif vk == 3 then 1
                    elseif vk == 6 then 4
                    else 0
                local region = if vk == 4
                    then regions.wide
                    elseif vk == 3 or vk == 8 then regions.row
                    elseif vk == 9 then regions.plus
                    else regions.single
                local spark = if elem == 3
                    then sparks.elec
                    elseif elem == 1 then sparks.fire
                    elseif elem == 4 then sparks.wood
                    else sparks.plain
                local fx = if elem == 3
                    then ELEC_FX
                    elseif elem == 1 then FIRE_FX
                    elseif elem == 4 then WOOD_FX
                    elseif vk == 4 then SLASH_FX
                    else IMPACT_FX
                local status: Status? = if vk == 2
                    then statuses.paralyze
                    elseif vk == 6 then statuses.confuse
                    else nil
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = elem,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = spark,
                    target = collision.hits_navis,
                    self_type = if vk == 4 then collision.slash else collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                    status = status,
                })
                battle.region_effects(tx, ty, region, me.alliance, fx, 0x10_0000)
                battle.play_sound(if vk == 4 then SWING_SOUND else BLAST_SOUND)
                battle.shake_camera(1, 8)
            end
            me.timer += 1
            if me.timer >= 24 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function virus_hook(vkind: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(VIRUS_CTRL, user, spec)
        if o then
            local st = o.state :: VirusState
            st.vkind = vkind
        end
        return o
    end
end

-- =============================================================================
-- Chip Definitions (52 chips, exact metadata from compat/chips.toml)
-- =============================================================================
local grabback: Chip = {
    codes = { "A", "E", "I", "K", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 10,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(GRAB_REVENGE_CTRL, user, spec)
    end,
}

local grabrvng: Chip = {
    codes = { "C", "G", "P", "R", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 30,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(GRAB_REVENGE_CTRL, user, spec)
    end,
}

local nrthwind: Chip = {
    codes = { "A", "C", "N", "S", "Z" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 3,
    mb = 43,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(NRTHWIND_CTRL, user, spec)
    end,
}

local fanfare: Chip = {
    codes = { "C", "E", "G", "L", "Y", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = inst_hook(1),
}

local discord: Chip = {
    codes = { "D", "F", "N", "T", "Z", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = inst_hook(2),
}

local timpani: Chip = {
    codes = { "A", "N", "Q", "U", "W", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = inst_hook(3),
}

local panic: Chip = {
    action = PANIC_ACTION,
    codes = { "A", "C", "L", "R", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

local rndmmetr: Chip = {
    codes = { "E", "I", "S", "V", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = meteor_hook(1),
}

local holemetr: Chip = {
    codes = { "C", "H", "J", "N", "Q" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = meteor_hook(2),
}

local shotmetr: Chip = {
    codes = { "A", "D", "F", "S", "Y" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = meteor_hook(3),
}

local lavacan1: Chip = {
    action = make_lavacan_action("lavacan1/action"),
    codes = { "A", "G", "S", "T", "V" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 34,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local lavacan2: Chip = {
    action = make_lavacan_action("lavacan2/action"),
    codes = { "B", "D", "F", "M", "O" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 42,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local lavacan3: Chip = {
    action = make_lavacan_action("lavacan3/action"),
    codes = { "E", "H", "J", "R", "W" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local volcano: Chip = {
    action = VOLCANO_ACTION,
    codes = { "A", "G", "J", "Y", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 75,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local iceball: Chip = {
    action = make_elem_ball_action("iceball/action", 2, "ice"),
    codes = { "F", "I", "M", "Q", "S", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local lavaball: Chip = {
    action = make_elem_ball_action("lavaball/action", 1, "lava"),
    codes = { "B", "C", "H", "N", "W", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local airstrm1: Chip = {
    action = make_airstrm_action("airstrm1/action"),
    codes = { "A", "F", "H", "O", "S" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 26,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local airstrm2: Chip = {
    action = make_airstrm_action("airstrm2/action"),
    codes = { "C", "G", "H", "M", "W" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local airstrm3: Chip = {
    action = make_airstrm_action("airstrm3/action"),
    codes = { "A", "G", "I", "N", "V" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 70,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local totem1: Chip = {
    action = make_totem_action("totem1/action", 40),
    codes = { "G", "H", "M", "O", "V" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 29,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local totem2: Chip = {
    action = make_totem_action("totem2/action", 60),
    codes = { "A", "C", "E", "T", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 38,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local totem3: Chip = {
    action = make_totem_action("totem3/action", 80),
    codes = { "D", "I", "K", "N", "Q" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 180,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local spice1: Chip = {
    codes = { "B", "D", "I", "S", "Z", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 22,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = spice_hook,
}

local spice2: Chip = {
    codes = { "C", "F", "I", "K", "R" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 34,
    damage = 110,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = spice_hook,
}

local spice3: Chip = {
    codes = { "D", "F", "J", "O", "Q" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 46,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = spice_hook,
}

local crsshld1: Chip = {
    action = make_crsshld_action("crsshld1/action"),
    codes = { "A", "C", "H", "L", "P", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 25,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local crsshld2: Chip = {
    action = make_crsshld_action("crsshld2/action"),
    codes = { "B", "C", "L", "S", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 33,
    damage = 210,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local crsshld3: Chip = {
    action = make_crsshld_action("crsshld3/action"),
    codes = { "C", "E", "L", "N", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 41,
    damage = 260,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local geyser: Chip = {
    action = GEYSER_ACTION,
    codes = { "B", "C", "L", "S", "W" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 22,
    damage = 200,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local shake1: Chip = {
    action = make_shake_action("shake1/action"),
    codes = { "E", "G", "R", "S", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 34,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local shake2: Chip = {
    action = make_shake_action("shake2/action"),
    codes = { "B", "F", "I", "L", "Q" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 110,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local shake3: Chip = {
    action = make_shake_action("shake3/action"),
    codes = { "D", "M", "R", "T", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local condor: Chip = {
    action = CONDOR_ACTION,
    codes = { "B", "I", "L", "S", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 44,
    damage = 180,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local burning: Chip = {
    action = BURNING_ACTION,
    codes = { "A", "F", "L", "R", "S" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 42,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local fireratn: Chip = {
    action = FIRERATN_ACTION,
    codes = { "B", "F", "H", "M", "Y" },
    element = "null",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 230,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rockarm1: Chip = {
    action = make_rockarm_action("rockarm1/action"),
    codes = { "D", "H", "J", "O", "P", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 23,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rockarm2: Chip = {
    action = make_rockarm_action("rockarm2/action"),
    codes = { "G", "M", "P", "S", "V", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local rockarm3: Chip = {
    action = make_rockarm_action("rockarm3/action"),
    codes = { "C", "I", "L", "T", "Z" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 200,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local needler1: Chip = {
    codes = { "I", "J", "M", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = needler_hook,
}

local needler2: Chip = {
    codes = { "F", "H", "M", "T", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 52,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = needler_hook,
}

local needler3: Chip = {
    codes = { "D", "L", "O", "U", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 72,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = needler_hook,
}

local team1: Chip = {
    codes = { "A", "C", "E", "T", "Z", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = team_hook(1),
}

local team2: Chip = {
    codes = { "D", "L", "P", "S", "Z", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = team_hook(2),
}

local mettaur: Chip = {
    codes = { "E", "L", "M", "O", "T" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 30,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(1),
}

local bunny: Chip = {
    codes = { "A", "B", "I", "R", "L" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 4,
    mb = 32,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(2),
}

local spikey: Chip = {
    codes = { "A", "E", "G", "R", "U" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 4,
    mb = 34,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(3),
}

local swordy: Chip = {
    codes = { "D", "I", "O", "S", "W" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 4,
    mb = 30,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(4),
}

local jelly: Chip = {
    codes = { "E", "J", "L", "R", "Y" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 46,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(5),
}

local mushy: Chip = {
    codes = { "C", "H", "M", "S", "Y" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 4,
    mb = 38,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(6),
}

local momogra: Chip = {
    codes = { "G", "M", "O", "R", "U" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 38,
    damage = 110,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(7),
}

local killreye: Chip = {
    codes = { "E", "I", "K", "L", "R" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 4,
    mb = 50,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(8),
}

local scuttlst: Chip = {
    codes = { "A", "D", "E", "M", "R" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 52,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = virus_hook(9),
}

return {
    grabback = grabback,
    grabrvng = grabrvng,
    nrthwind = nrthwind,
    fanfare = fanfare,
    discord = discord,
    timpani = timpani,
    panic = panic,
    rndmmetr = rndmmetr,
    holemetr = holemetr,
    shotmetr = shotmetr,
    lavacan1 = lavacan1,
    lavacan2 = lavacan2,
    lavacan3 = lavacan3,
    volcano = volcano,
    iceball = iceball,
    lavaball = lavaball,
    airstrm1 = airstrm1,
    airstrm2 = airstrm2,
    airstrm3 = airstrm3,
    totem1 = totem1,
    totem2 = totem2,
    totem3 = totem3,
    spice1 = spice1,
    spice2 = spice2,
    spice3 = spice3,
    crsshld1 = crsshld1,
    crsshld2 = crsshld2,
    crsshld3 = crsshld3,
    geyser = geyser,
    shake1 = shake1,
    shake2 = shake2,
    shake3 = shake3,
    condor = condor,
    burning = burning,
    fireratn = fireratn,
    rockarm1 = rockarm1,
    rockarm2 = rockarm2,
    rockarm3 = rockarm3,
    needler1 = needler1,
    needler2 = needler2,
    needler3 = needler3,
    team1 = team1,
    team2 = team2,
    mettaur = mettaur,
    bunny = bunny,
    spikey = spikey,
    swordy = swordy,
    jelly = jelly,
    mushy = mushy,
    momogra = momogra,
    killreye = killreye,
    scuttlst = scuttlst,
}
