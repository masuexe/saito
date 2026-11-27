--!strict
-- EXE3 Electric Chips: Sensor1-3 (0x065..0x067) and Plasma1-3 (0x05D..0x05F).

local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local HIT_SPARKS = asset.sprite("hit-sparks")
local ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local APPEAR_SOUND = asset.sound("appear")
local SHOT_SOUND = asset.sound("buster-shot")
local BLAST_SOUND = asset.sound("hit-bomb")

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
            sprite:load(HIT_SPARKS)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 4, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0x08_0000)
            local s = me.state :: SensorState
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
-- Plasma1 / Plasma2 / Plasma3:
-- Spawns an electric plasma ball 2 panels ahead that strikes with rapid elec pulses!
-- =============================================================================
type PlasmaState = { tick: number, pulses: number, max_pulses: number, wide: boolean }

local PLASMA_KIND = new.kind {
    id = "elec/plasma-ball",
    pool = "attack",
    state = { tick = "u16", pulses = "u8", max_pulses = "u8", wide = "bool" },
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
            me.anim, me.anim_loaded = 4, 0xFF
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0x10_0000)
            local s = me.state :: PlasmaState
            s.tick = 0
            s.pulses = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PlasmaState
            if s.tick % 8 == 0 and s.pulses < s.max_pulses then
                s.pulses += 1
                local region = if s.wide then regions.plus else regions.single
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 3,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.elec,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    hit_mod = if s.pulses == s.max_pulses then 3 else 1,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, ELEC_FX, 0x10_0000)
                battle.play_sound(SHOT_SOUND)
            end
            s.tick += 1
            if s.tick >= s.max_pulses * 8 + 4 then
                me:set_lifecycle("destroy")
                return
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type PlasmaActState = { timer: number }

local function make_plasma_action(id: string, max_pulses: number, wide: boolean): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: PlasmaActState)
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 2 then
                local tx = me.panel_x + me.facing * 2
                local ty = me.panel_y
                -- If opponent is in front in this row, lock onto their X!
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and opp.panel_y == me.panel_y and (opp.panel_x - me.panel_x) * me.facing > 0 then
                    tx = opp.panel_x
                end
                if field.valid(tx, ty) then
                    local o = battle.spawn(PLASMA_KIND)
                    if o then
                        o.panel_x, o.panel_y = tx, ty
                        o.alliance, o.flip = me.alliance, me.flip
                        o.element = 3
                        local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                        o:set_damage_word(dmg)
                        local ps = o.state :: PlasmaState
                        ps.max_pulses = max_pulses
                        ps.wide = wide
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
    action = make_plasma_action("plasma1/action", 3, false),
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
    action = make_plasma_action("plasma2/action", 3, true),
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
    action = make_plasma_action("plasma3/action", 4, true),
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
