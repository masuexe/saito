--!strict
-- EXE3 B+Left specials: Shield (Shield Style / Shield program), Block, Reflect, and AntiDmg (Shadow Style / AntiDmg NCP).
-- AntiDmg enters a counter stance and, upon absorbing an attack, vanishes in a puff, grants temporary invisibility,
-- and throws a diving shuriken onto the opponent's panel for 100 damage.

local api = require("../../../rules/api")
local collision = require("../../../rules/collision")
local guard = require("../../../lib/guard")
local projectile = require("../../../objects/projectile")
local shield = require("../../../objects/shield")
local shuriken = require("../../../objects/shuriken")
local sparks = require("../../../lib/sparks")

local guards = {}

local TICKS = 20
local BLOCK_TICKS = 16
local ANTIDMG_STANCE_TICKS = 24
local REFLECT_COOLDOWN = 40
local CHARGE = { 60, 60, 60, 60, 60, 60 }

local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
    shield = shield.look { anim = 0 },
    counter = nil,
}

local BLOCK = guard.action {
    id = "megaman/block/action",
    ticks = BLOCK_TICKS,
    shield = shield.look { anim = 0 },
    counter = nil,
}

local REFLECTED = projectile.variant {
    id = "megaman/reflect/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.charged,
}

local REFLECTED_Z = 0x14_0000
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    shield = shield.look { anim = 0 },
    counter = function(me: Object, x: number, y: number, damage: number)
        projectile.spawn(me, x, y, { variant = REFLECTED, damage = damage, z = REFLECTED_Z })
    end,
}

-- Shadow Style / AntiDmg B+Left counter stance:
-- Arms AntiDmg stance for 24 ticks (caught via `rules/init.luau` `hooks.hit_guard` OR guard_dirs).
-- When triggered: vanishes in a puff, grants temporary invisibility (90 ticks), and throws a diving shuriken!
type AntiDmgState = { timer: number, countered: boolean }

local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local INVIS_SOUND = asset.sound("invisible")

function guards.trigger_anti_dmg_counter(me: Object, damage: number)
    api.clear_anti_dmg_stance(me.alliance)
    me:set_status("guard", false)
    me:set_status_timer("flash", 90)
    me:set_status("invisible", true)
    battle.effect(me.pos, PUFF_FX)
    battle.play_sound(INVIS_SOUND)
    local tx, ty = shuriken.target_enemy(me)
    if tx and ty then
        shuriken.spawn(me, tx, ty, 0, 3, damage)
    end
end

local ANTIDMG_ACTION = new.action {
    id = "megaman/anti-dmg/action",
    state = { timer = "u16", countered = "bool" },
    update = function(me: Object, s: AntiDmgState)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me.anim = 0
            api.arm_anti_dmg_stance(me.alliance, ANTIDMG_STANCE_TICKS)
            s.timer = ANTIDMG_STANCE_TICKS
            s.countered = false
            me.step_init = 4
            return
        end
        -- If `hooks.hit_guard` cleared the stance (counter triggered!), finish cleanly.
        if api.state(me.alliance).anti_dmg_stance == 0 and not s.countered then
            s.countered = true
            me:exit_attack()
            return
        end
        s.timer -= 1
        if s.timer <= 0 then
            api.clear_anti_dmg_stance(me.alliance)
            me:exit_attack()
        end
    end,
}

local function raise(navi: Object, variant: number, damage: number)
    navi.attack_element = 0
    navi.attack_variant = variant
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
end

guards.shield = new.weapon {
    id = "megaman/shield",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 1, 0)
        return SHIELD
    end,
}

guards.block = new.weapon {
    id = "megaman/block",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 1, 0)
        return BLOCK
    end,
}

guards.reflect = new.weapon {
    id = "megaman/reflect",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 2, 100)
        navi.back_special_cooldown = REFLECT_COOLDOWN
        return REFLECT
    end,
}

guards.anti_dmg = new.weapon {
    id = "megaman/anti-dmg",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 3, 100)
        navi.back_special_cooldown = 45
        return ANTIDMG_ACTION
    end,
}

return guards
