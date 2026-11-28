--!strict
-- EXE3 Electric Chips: Sensor1-3 (0x065..0x067) and Plasma1-3 (0x05D..0x05F).

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local HIT_SPARKS = asset.sprite("hit-sparks")
local SENSOR_SPRITE = asset.sprite("sensor")
local PLASMA_SPRITE = asset.sprite("plasma")
local PLASMA_SPARK_SPRITE = asset.sprite("plasma-spark")
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local APPEAR_SOUND = asset.sound("appear")
local PLASMA_SPARK_SOUND = asset.sound("plasma-spark")
local BLAST_SOUND = asset.sound("hit-bomb")

local SOLID_PANEL, OCCUPIED_PANEL = 0x10, 0x0F880080

-- =============================================================================
-- Sensor1 / Sensor2 / Sensor3:
-- Places an electric turret ahead that scans forward; when an opponent enters
-- the beam's line of sight (rows based on tier), fires a paralyzing electric beam!
-- =============================================================================
type SensorState = { tier: number, life: number, cooldown: number }

local SENSOR_KIND = new.kind {
    id = "elec/sensor-turret",
    pool = "attack",
    state = { tier = "u8", life = "u16", cooldown = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            local s = me.state :: SensorState
            sprite:load(SENSOR_SPRITE)
            sprite.shadow = "hidden"
            sprite.palette = math.max(0, s.tier - 1) * 3
            sprite:set_flip(bit32.bxor(me.alliance, 1))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0)
            s.life = 480
            s.cooldown = 0
            battle.play_sound(APPEAR_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: SensorState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            else
                -- Scan opponent actors in sensor's trigger rows.
                local opp_side = bit32.bxor(me.alliance, 1)
                local triggered = false
                for _, opp in battle.alive_actors(opp_side) do
                    local dx = (opp.panel_x - me.panel_x) * me.facing
                    local dy = math.abs(opp.panel_y - me.panel_y)
                    if dx > 0 and (dy == 0 or (s.tier >= 2 and dy <= 1)) then
                        triggered = true
                        break
                    end
                end
                if triggered then
                    local bx = me.panel_x + me.facing
                    if field.valid(bx, me.panel_y) then
                        local region = if s.tier >= 3 then regions.deep_fan else regions.line
                        battle.hitbox(me, {
                            panel_x = bx,
                            panel_y = me.panel_y,
                            element = 3,
                            z = 0x10_0000,
                            region = region,
                            hit_effect = sparks.elec,
                            target = collision.hits_navis,
                            self_type = collision.piercing,
                            damage = me.damage,
                            hit_mod = 3,
                            status = statuses.paralyze,
                        })
                        battle.region_effects(bx, me.panel_y, region, me.alliance, ELEC_FX, 0x10_0000)
                        battle.play_sound(BLAST_SOUND)
                    end
                    s.life = 1
                end
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type SensorCtrlState = { tier: number }

local SENSOR_CTRL = new.kind {
    id = "elec/sensor-controller",
    pool = "effect",
    state = { tier = "u8" },
    update = dimming_chips.phases {
        name = "Sensor",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                local s = me.state :: SensorCtrlState
                if user then
                    local tx = user.panel_x + user.facing
                    local ty = user.panel_y
                    if field.valid(tx, ty) then
                        local o = battle.spawn(SENSOR_KIND)
                        if o then
                            o.panel_x, o.panel_y = tx, ty
                            o.alliance, o.flip = user.alliance, user.flip
                            o.element = 3
                            o:set_damage_word(me.damage)
                            ;(o.state :: SensorState).tier = s.tier
                        end
                    end
                end
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

local function sensor_hook(tier: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(SENSOR_CTRL, user, spec)
        if o then
            local st = o.state :: SensorCtrlState
            st.tier = tier
        end
        return o
    end
end

-- =============================================================================
-- Plasma1 / Plasma2 / Plasma3 (A6BE 0x080B2AD4, 0x080D938F, 0x080D92A8):
-- Places an EleBall (04-23, palette 0/1/2, HP 10/30/50) 1 panel ahead.
-- After 40 ticks, fires rotating opposite pairs of electric sparks (10-07 anim 1)
-- on the 8 surrounding panels (12x/15t, 15x/12t, 22x/8t).
-- =============================================================================
local PLASMA_OFFSETS = {
    { 0, -1 },
    { -1, -1 },
    { -1, 0 },
    { -1, 1 },
}

type PlasmaSparkState = { life: number }

local PLASMA_SPARK_KIND = new.kind {
    id = "elec/plasma-spark",
    pool = "attack",
    state = { life = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(PLASMA_SPARK_SPRITE)
            sprite.shadow = "hidden"
            sprite:set_flip(me.alliance)
            me.anim, me.anim_loaded = 1, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0x10_0000)
            ;(me.state :: PlasmaSparkState).life = 15
            battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = 3,
                z = 0x10_0000,
                region = regions.single,
                hit_effect = sparks.elec,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 1,
            })
            me:set_lifecycle("update")
            me:update_sprite()
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PlasmaSparkState
            if s.life <= 1 then
                me:set_lifecycle("destroy")
                return
            end
            s.life -= 1
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type PlasmaState = { tier: number, hp: number, timer: number, interval: number, remaining: number, step: number }

local function spawn_plasma_spark(owner: Object, px: number, py: number)
    if not field.valid(px, py) then
        return
    end
    local o = battle.spawn(PLASMA_SPARK_KIND)
    if o then
        o.panel_x, o.panel_y = px, py
        o.alliance, o.flip = owner.alliance, owner.flip
        o.element = 3
        o:set_damage_word(int.u32(bit32.bor(owner.damage, bit32.lshift(owner.stamina, 16))))
    end
end

local PLASMA_KIND = new.kind {
    id = "elec/plasma-ball",
    pool = "attack",
    state = { tier = "u8", hp = "u16", timer = "u16", interval = "u8", remaining = "u8", step = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            if not field.check(me.panel_x, me.panel_y, SOLID_PANEL, OCCUPIED_PANEL) then
                me:set_coordinates_from_panel()
                battle.effect(Vec3.new(me.pos.x, me.pos.y, 0x10_0000), PUFF_FX)
                me:free()
                return
            end
            local s = me.state :: PlasmaState
            me.visible = true
            local sprite = me.sprite
            sprite:load(PLASMA_SPRITE)
            sprite.shadow = "hidden"
            sprite.palette = s.tier
            sprite:set_flip(bit32.bxor(me.alliance, 1))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
            me.hp = s.hp
            me.max_hp = s.hp
            if me:create_collision() then
                me:setup_collision(collision.navi, collision.navi_target, 3)
                me.collision:set_region(regions.single)
            end
            s.timer = 40
            s.step = 0
            battle.effect(Vec3.new(me.pos.x, me.pos.y, 0x1E_0000), PUFF_FX)
            me:set_lifecycle("update")
            me:update_sprite()
        elseif l == "update" then
            if battle.over() or not field.solid(me.panel_x, me.panel_y) then
                battle.effect(Vec3.new(me.pos.x, me.pos.y, 0x1E_0000), PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if me.has_collision then
                me.collision:remove()
                if me:take_damage(0) < 0 or me.hp == 0 then
                    battle.play_sound(BLAST_SOUND)
                    battle.effect(Vec3.new(me.pos.x, me.pos.y, 0x1E_0000), PUFF_FX)
                    me:set_lifecycle("destroy")
                    return
                end
            end
            local s = me.state :: PlasmaState
            if s.timer > 0 then
                s.timer -= 1
            end
            if s.timer == 0 then
                if me.anim == 0 then
                    me.anim, me.anim_loaded = 1, 0xFF
                end
                local pair = PLASMA_OFFSETS[(s.step % 4) + 1]
                local dx = pair[1] * me.facing
                local dy = pair[2]
                spawn_plasma_spark(me, me.panel_x + dx, me.panel_y + dy)
                spawn_plasma_spark(me, me.panel_x - dx, me.panel_y - dy)
                battle.play_sound(PLASMA_SPARK_SOUND)
                s.remaining -= 1
                if s.remaining == 0 then
                    battle.effect(Vec3.new(me.pos.x, me.pos.y, 0x1E_0000), PUFF_FX)
                    me:set_lifecycle("destroy")
                    return
                end
                s.step = (s.step + 1) % 4
                s.timer = s.interval
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

type PlasmaActState = { timer: number }

local function make_plasma_action(id: string, tier: number, hp: number, pulses: number, interval: number): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: PlasmaActState)
            if me.step_init == 0 then
                me:set_animation(anims.FLAMETHROWER)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 2 then
                local tx = me.panel_x + me.facing
                local ty = me.panel_y
                if field.valid(tx, ty) then
                    local o = battle.spawn(PLASMA_KIND)
                    if o then
                        o.panel_x, o.panel_y = tx, ty
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 3
                        local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(dmg)
                        local ps = o.state :: PlasmaState
                        ps.tier = tier
                        ps.hp = hp
                        ps.remaining = pulses
                        ps.interval = interval
                    end
                    battle.play_sound(APPEAR_SOUND)
                end
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

local sensor1: Chip = {
    codes = { "C", "K", "L", "O", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 62,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = sensor_hook(1),
}

local sensor2: Chip = {
    codes = { "A", "G", "H", "N", "S" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 70,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = sensor_hook(2),
}

local sensor3: Chip = {
    codes = { "B", "E", "J", "O", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 72,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = sensor_hook(3),
}

local plasma1: Chip = {
    action = make_plasma_action("plasma1/action", 0, 10, 12, 15),
    codes = { "B", "D", "J", "R", "T" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local plasma2: Chip = {
    action = make_plasma_action("plasma2/action", 1, 30, 15, 12),
    codes = { "A", "G", "K", "M", "Q" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local plasma3: Chip = {
    action = make_plasma_action("plasma3/action", 2, 50, 22, 8),
    codes = { "F", "I", "N", "P", "S" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    sensor1 = sensor1,
    sensor2 = sensor2,
    sensor3 = sensor3,
    plasma1 = plasma1,
    plasma2 = plasma2,
    plasma3 = plasma3,
}
