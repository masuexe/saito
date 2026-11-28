--!strict
-- EXE3 Trap, Bomb, Chess, Guard, Snake, Hole & Repair chips:
-- TimeBomb (0x063), Mine (0x064), BlkBomb1-3 (0x01B..0x01D), Prism (0x08E),
-- Rook (0x099), Pawn (0x097), Knight (0x098), Guard (0x076), Snake (0x083),
-- Hole (0x0A1), Repair (0x09F).

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local common_debris = require("@exelib/rock/debris")
local common_rock = require("@exelib/rock")
local dimming_chips = require("@exelib/dimming")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local BOMB_SPRITE = asset.sprite("bomb")
local BLKBOMB_SPRITE = asset.sprite("black-bomb")
local CUBE_SPRITE = asset.sprite("rockcube")
local SHIELD_SPRITE = asset.sprite("reflector-shield")
local HIT_SPARKS = asset.sprite("hit-sparks")
local SMALL_PUFF_SPRITE = asset.sprite("small-puff")
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 20 }
local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 16 }
local PUFF_FX = new.effect { sprite = SMALL_PUFF_SPRITE, anim = 1, palette = 0, steps = 12 }
local DUST_FX = new.effect { sprite = SMALL_PUFF_SPRITE, anim = 1, palette = 0, steps = 14 }
local SLASH_FX = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0, steps = 12 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 12 }
local HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 14 }

local APPEAR_SOUND = asset.sound("appear")
local BLAST_SOUND = asset.sound("hit-bomb")
local THROW_SOUND = asset.sound("minibomb-throw")
local CRACK_SOUND = asset.sound("panel-crack")
local GUARD_SOUND = asset.sound("guard")
local SHIELD_SOUND = asset.sound("shield")
local SWING_SOUND = asset.sound("sword-swing")
local TRAP_SOUND = asset.sound("trap-mark")
local RECOVER_SOUND = asset.sound("sfx-recover")

local BREAK_ELEM = element.byte("null", { "break" })

local function dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

-- =============================================================================
-- 1. TimeBomb: Countdown bomb placed on enemy area; explodes across enemy field!
-- =============================================================================
type TimeBombState = { ticks: number }

local TIMEBOMB_KIND = new.kind {
    id = "trap/timebomb",
    pool = "attack",
    state = { ticks = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(BOMB_SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            me.hp = 100
            me.max_hp = 100
            if me:create_collision() then
                me:setup_collision(collision.navi, collision.navi_target, 0)
                me.collision:set_region(regions.single)
            end
            ;(me.state :: TimeBombState).ticks = 180
            battle.play_sound(APPEAR_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            if me.has_collision then
                me.collision:remove()
                if me:take_damage(0) < 0 then
                    battle.effect(me.pos, PUFF_FX)
                    me:set_lifecycle("destroy")
                    return
                end
            end
            local s = me.state :: TimeBombState
            s.ticks -= 1
            if s.ticks <= 0 then
                local area = regions.other_area[me.alliance]
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = area,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(me.panel_x, me.panel_y, area, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 18)
                me:set_lifecycle("destroy")
                return
            end
            if s.ticks % 60 == 0 then
                battle.play_sound(TRAP_SOUND)
            end
            if me.has_collision then
                me.collision:present()
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local TIMEBOMB_CTRL = new.kind {
    id = "trap/timebomb-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "TimeBomb",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local tx = if me.alliance == 0 then 5 else 2
                local ty = 2
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and not opp:status("dead") then
                    tx = opp.panel_x
                    ty = if opp.panel_y == 2 then 1 else 2
                end
                local o = battle.spawn(TIMEBOMB_KIND)
                if o then
                    o.panel_x, o.panel_y = tx, ty
                    o.alliance, o.flip = me.alliance, me.flip
                    o:set_damage_word(me.damage)
                end
                me.timer = 18
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
-- 2. Mine: Hides an invisible 300-dmg mine on a random enemy panel!
-- =============================================================================
type MineState = { life: number }

local MINE_KIND = new.kind {
    id = "trap/mine",
    pool = "attack",
    state = { life = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            -- Visible only to its owner's side!
            me.visible = false
            me:set("visible", true, me.alliance)
            local sprite = me.sprite
            sprite:load(BOMB_SPRITE)
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            ;(me.state :: MineState).life = 1200
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: MineState
            s.life -= 1
            if s.life <= 0 then
                me:set_lifecycle("destroy")
                return
            end
            local opp_side = bit32.bxor(me.alliance, 1)
            for _, opp in battle.alive_actors(opp_side) do
                if opp.panel_x == me.panel_x and opp.panel_y == me.panel_y and not opp:status("dead") and not opp:status("air_shoe") then
                    battle.hitbox(me, {
                        panel_x = me.panel_x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.blast,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    battle.effect(me.pos, BLAST_FX)
                    battle.play_sound(BLAST_SOUND)
                    battle.shake_camera(2, 14)
                    me:set_lifecycle("destroy")
                    return
                end
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local MINE_CTRL = new.kind {
    id = "trap/mine-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Mine",
        hidden = true,
        effect = function(me: Object)
            if me.phase_init == 0 then
                local area = regions.other_area[me.alliance]
                local rx, ry = regions.random_panel(me.panel_x, me.panel_y, area, me.facing, 0x10, 0x600000)
                if rx == nil or ry == nil then
                    rx = if me.alliance == 0 then 5 else 2
                    ry = 2
                end
                local o = battle.spawn(MINE_KIND)
                if o then
                    o.panel_x, o.panel_y = rx :: number, ry :: number
                    o.alliance, o.flip = me.alliance, me.flip
                    o:set_damage_word(me.damage)
                end
                battle.play_sound(TRAP_SOUND)
                me.timer = 18
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
-- 3. BlkBomb1 / BlkBomb2 / BlkBomb3:
-- Throws a black bomb 3 panels ahead; detonates into a huge 3x3 fire blast
-- immediately if hit by fire or after fuse expires!
-- =============================================================================
type BlkBombState = { fuse: number }

local BLKBOMB_KIND = new.kind {
    id = "trap/blkbomb",
    pool = "attack",
    state = { fuse = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(BLKBOMB_SPRITE)
            sprite.shadow = "with_sprite"
            sprite.palette = 0
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            me.hp = 100
            me.max_hp = 100
            if me:create_collision() then
                me:setup_collision(collision.navi, collision.navi_target, 0)
                me.collision:set_region(regions.single)
            end
            ;(me.state :: BlkBombState).fuse = 150
            battle.play_sound(THROW_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local detonate = false
            if me.has_collision then
                me.collision:remove()
                -- Detonate immediately when struck by Fire (element 1) or lethal damage!
                if me.collision:element_damage(1) > 0 then
                    detonate = true
                elseif me:take_damage(0) < 0 then
                    battle.effect(me.pos, PUFF_FX)
                    me:set_lifecycle("destroy")
                    return
                end
            end
            local s = me.state :: BlkBombState
            s.fuse -= 1
            if detonate or s.fuse <= 0 then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 1, -- fire
                    z = 0x10_0000,
                    region = regions.square,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 18)
                me:set_lifecycle("destroy")
                return
            end
            if me.has_collision then
                me.collision:present()
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type ActTimer = { timer: number }

local function make_blkbomb_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: ActTimer)
            if me.step_init == 0 then
                me:set_animation(anims.BOMB_THROW)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 6 then
                local tx = me.panel_x + me.facing * 3
                if field.valid(tx, me.panel_y) then
                    local o = battle.spawn(BLKBOMB_KIND)
                    if o then
                        o.panel_x, o.panel_y = tx, me.panel_y
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 1
                        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(damage)
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
-- 4. Prism: Crystal obstacle that refracts any hit onto all 8 surrounding panels!
-- =============================================================================
type PrismState = { life: number }

local PRISM_KIND = new.kind {
    id = "trap/prism",
    pool = "attack",
    state = { life = "u16" },
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
            me.hp = 999
            me.max_hp = 999
            if me:create_collision() then
                me:setup_collision(collision.navi, collision.navi_target, 0)
                me.collision:set_region(regions.single)
            end
            ;(me.state :: PrismState).life = 360
            battle.play_sound(APPEAR_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PrismState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if me.has_collision then
                me.collision:remove()
                local dmg = me.collision.final_damage
                if dmg > 0 then
                    -- Refract received damage across all 8 surrounding panels!
                    battle.hitbox(me, {
                        panel_x = me.panel_x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.ring,
                        hit_effect = sparks.charged,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = dmg,
                        hit_mod = 1,
                    })
                    battle.region_effects(me.panel_x, me.panel_y, regions.ring, me.alliance, HEAL_FX, 0x10_0000)
                    battle.play_sound(GUARD_SOUND)
                end
                me.collision:present()
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local PRISM_ACTION = new.action {
    id = "prism/action",
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
            local tx = me.panel_x + me.facing * 3
            if field.valid(tx, me.panel_y) then
                local o = battle.spawn(PRISM_KIND)
                if o then
                    o.panel_x, o.panel_y = tx, me.panel_y
                    o.alliance, o.flip = me.alliance, me.flip
                end
            end
        end
        s.timer += 1
        if s.timer >= 18 then
            me:exit_attack()
        end
    end,
}

-- =============================================================================
-- 5. Chess pieces: Rook (durable obstacle), Pawn (slashes ahead), Knight (leaps!)
-- =============================================================================
local ROOK_DEBRIS = common_debris.make_kind("trap/rook-debris", asset.sprite("dust"), DUST_FX)
local ROOK_ROCK = common_rock.make {
    id = "trap/rook-obstacle",
    sprite = CUBE_SPRITE,
    reloads_anim = false,
    debris = ROOK_DEBRIS,
    effects = { puff = PUFF_FX, dust = DUST_FX },
    spark = sparks.guard,
    is = collision.navi,
    hits = collision.navi_target,
    leaves_by_hp = true,
    crush = "destroys",
}
local ROOK_VARIANT = common_rock.variant {
    anim = 9,
    hp = 600,
    element = "null",
    debris_palette = 0,
    break_sound = CRACK_SOUND,
    identity = nil,
}

local ROOK_CTRL = new.kind {
    id = "trap/rook-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Rook",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                if user then
                    local tx = user.panel_x + user.facing
                    if field.valid(tx, user.panel_y) then
                        ROOK_ROCK.spawn(tx, user.panel_y, user.alliance, { variant = ROOK_VARIANT, class = 0, entrance = "instant" }, 0)
                        battle.play_sound(APPEAR_SOUND)
                    end
                end
                me.timer = 18
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

type PawnState = { life: number, tick: number }

local PAWN_KIND = new.kind {
    id = "trap/pawn",
    pool = "attack",
    state = { life = "u16", tick = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(CUBE_SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 4, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: PawnState
            s.life = 420
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PawnState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick % 45 == 15 then
                local sx = me.panel_x + me.facing
                if field.valid(sx, me.panel_y) then
                    battle.hitbox(me, {
                        panel_x = sx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.long,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.slash,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 3,
                    })
                    battle.region_effects(sx, me.panel_y, regions.long, me.alliance, SLASH_FX, 0x10_0000)
                    battle.play_sound(SWING_SOUND)
                end
            end
            s.tick += 1
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local PAWN_CTRL = new.kind {
    id = "trap/pawn-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Pawn",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                if user then
                    local tx = user.panel_x + user.facing
                    if field.valid(tx, user.panel_y) then
                        local o = battle.spawn(PAWN_KIND)
                        if o then
                            o.panel_x, o.panel_y = tx, user.panel_y
                            o.alliance, o.flip = user.alliance, user.flip
                            o:set_damage_word(me.damage)
                        end
                    end
                end
                me.timer = 18
                me.phase_init = 4
            end
            me.timer -= 1
            if me.timer < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local KNIGHT_CTRL = new.kind {
    id = "trap/knight-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Knight",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 10 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                local tx = if opp and not opp:status("dead") then opp.panel_x else (if me.alliance == 0 then 5 else 2)
                local ty = if opp and not opp:status("dead") then opp.panel_y else 2
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = BREAK_ELEM,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x08_0000), IMPACT_FX)
                field.crack(tx, ty)
                battle.play_sound(CRACK_SOUND)
                battle.shake_camera(2, 12)
            end
            me.timer += 1
            if me.timer >= 28 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 6. Guard: Brief shield reflection; blocking any hit fires a 40-dmg shockwave!
-- =============================================================================
type GuardWaveState = { tick: number }

local GUARD_WAVE = new.kind {
    id = "trap/guard-wave",
    pool = "attack",
    state = { tick = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(SHIELD_SPRITE)
            sprite.shadow = "hidden"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            ;(me.state :: GuardWaveState).tick = 0
            battle.play_sound(GUARD_SOUND)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: GuardWaveState
            if s.tick == 0 then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x0C_0000,
                    region = regions.single,
                    hit_effect = sparks.plain,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.effect(me.pos, IMPACT_FX)
            end
            s.tick += 1
            if s.tick >= 5 then
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

type GuardActState = { timer: number, countered: boolean }

local GUARD_ACTION = new.action {
    id = "guard/action",
    state = { timer = "u16", countered = "bool" },
    update = function(me: Object, s: GuardActState)
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
            local sx = me.panel_x + me.facing
            if field.valid(sx, me.panel_y) then
                local o = battle.spawn(GUARD_WAVE)
                if o then
                    o.panel_x, o.panel_y = sx, me.panel_y
                    o.alliance, o.flip = me.alliance, me.flip
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    o:set_damage_word(damage)
                end
            end
        end
        s.timer += 1
        if s.timer >= 36 then
            me:set_status("guard", false)
            me:exit_attack()
        end
    end,
}

-- =============================================================================
-- 7. Snake: Fires a Wood snake for every hole on user's area!
-- =============================================================================
type SnakeState = { remaining: number, tick: number }

local SNAKE_CTRL = new.kind {
    id = "trap/snake-controller",
    pool = "effect",
    state = { remaining = "u8", tick = "u8" },
    update = dimming_chips.phases {
        name = "Snake",
        effect = function(me: Object)
            local s = me.state :: SnakeState
            if me.phase_init == 0 then
                local holes = 0
                for py = 1, 3 do
                    for px = 1, 6 do
                        local p = field.panel(px, py)
                        if p and p.alliance == me.alliance and (p.kind == "missing" or p.kind == "broken") then
                            holes += 1
                        end
                    end
                end
                -- At least 1 snake even with 0 holes so the chip always has feedback.
                s.remaining = math.max(1, holes)
                s.tick = 0
                battle.play_sound(APPEAR_SOUND)
                me.phase_init = 4
            end
            if s.tick % 7 == 0 and s.remaining > 0 then
                s.remaining -= 1
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                local tx = if opp and not opp:status("dead") then opp.panel_x else (if me.alliance == 0 then 5 else 2)
                local ty = if opp and not opp:status("dead") then opp.panel_y else 2
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 4, -- wood
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.wood,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.remaining == 0 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), WOOD_FX)
                battle.play_sound(SWING_SOUND)
            end
            s.tick += 1
            if s.remaining <= 0 and s.tick >= 16 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- 8. Hole: Opens a black-hole hazard breaking empty enemy panels!
-- =============================================================================
local HOLE_CTRL = new.kind {
    id = "trap/hole-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Hole",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                local tx = if opp and not opp:status("dead") then opp.panel_x else (if me.alliance == 0 then 5 else 2)
                local ty = if opp and not opp:status("dead") then opp.panel_y else 2
                for py = 1, 3 do
                    for px = 1, 6 do
                        local p = field.panel(px, py)
                        if p and p.alliance ~= me.alliance and math.abs(px - tx) <= 1 and py == ty then
                            field.break_empty(px, py)
                        end
                    end
                end
                -- Also remove any enemy field obstacles into the void!
                for _, obj in field.objects(bit32.bxor(me.alliance, 1)) do
                    if obj.active and obstacle.present(obj) then
                        obstacle.vanish(obj)
                    end
                end
                battle.play_sound(CRACK_SOUND)
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
-- 9. Repair: Repairs all broken & cracked panels in the user's area!
-- =============================================================================
local REPAIR_CTRL = new.kind {
    id = "trap/repair-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Repair",
        effect = function(me: Object)
            if me.phase_init == 0 then
                for py = 1, 3 do
                    for px = 1, 6 do
                        local p = field.panel(px, py)
                        if p and p.alliance == me.alliance and (p.kind == "broken" or p.kind == "cracked" or p.kind == "missing") then
                            field.set_type(px, py, "normal")
                            local cx, cy = field.center(px, py)
                            battle.effect(Vec3.new(cx, cy, 0), HEAL_FX)
                        end
                    end
                end
                battle.play_sound(RECOVER_SOUND)
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

local timebomb: Chip = {
    codes = { "J", "K", "L", "M", "N" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(TIMEBOMB_CTRL, user, spec)
    end,
}

local mine: Chip = {
    codes = { "A", "D", "L", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(MINE_CTRL, user, spec, true)
    end,
}

local blkbomb1: Chip = {
    action = make_blkbomb_action("blkbomb1/action"),
    codes = { "F", "L", "N", "P", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 60,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local blkbomb2: Chip = {
    action = make_blkbomb_action("blkbomb2/action"),
    codes = { "D", "I", "K", "Q", "S" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 70,
    damage = 170,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local blkbomb3: Chip = {
    action = make_blkbomb_action("blkbomb3/action"),
    codes = { "C", "G", "L", "U", "Y" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 80,
    damage = 220,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local prism: Chip = {
    action = PRISM_ACTION,
    codes = { "H", "J", "K", "Q", "W" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

local rook: Chip = {
    codes = { "D", "F", "N", "Q", "U", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(ROOK_CTRL, user, spec)
    end,
}

local pawn: Chip = {
    codes = { "B", "E", "N", "R", "Y" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(PAWN_CTRL, user, spec)
    end,
}

local knight: Chip = {
    codes = { "C", "H", "M", "U", "V" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 64,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(KNIGHT_CTRL, user, spec)
    end,
}

local guard: Chip = {
    action = GUARD_ACTION,
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local snake: Chip = {
    codes = { "D", "E", "I", "R", "Y" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(SNAKE_CTRL, user, spec)
    end,
}

local hole: Chip = {
    codes = { "A", "B", "S", "T", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 55,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(HOLE_CTRL, user, spec)
    end,
}

local repair: Chip = {
    codes = { "A", "C", "D", "F", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(REPAIR_CTRL, user, spec)
    end,
}

return {
    timebomb = timebomb,
    mine = mine,
    blkbomb1 = blkbomb1,
    blkbomb2 = blkbomb2,
    blkbomb3 = blkbomb3,
    prism = prism,
    rook = rook,
    pawn = pawn,
    knight = knight,
    guard = guard,
    snake = snake,
    hole = hole,
    repair = repair,
}
