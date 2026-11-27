--!strict
-- EXE3 B+Left specials: Shield (Shield Style / Shield program), Reflect (Reflect program), and AntiDmg/Shadow Invis.

local collision = require("../../../rules/collision")
local guard = require("../../../lib/guard")
local projectile = require("../../../objects/projectile")
local shield = require("../../../objects/shield")
local sparks = require("../../../lib/sparks")

local guards = {}

local TICKS = 20
local REFLECT_COOLDOWN = 40
local CHARGE = { 60, 60, 60, 60, 60, 60 }

local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
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

-- Shadow Style / AntiDmg B+Left quick invisibility dodge (16 ticks of invisible + brief cooldown).
type ShadowState = { timer: number }
local SHADOW_INVIS_SOUND = asset.sound("invisible")
local SHADOW_ACTION = new.action {
    id = "megaman/anti-dmg/action",
    state = { timer = "u16" },
    update = function(me: Object, s: ShadowState)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_status("invisible", true)
            me:set_status_timer("invulnerable", 90)
            battle.play_sound(SHADOW_INVIS_SOUND)
            me.step_init = 4
            s.timer = 14
        end
        s.timer -= 1
        if s.timer == 0 then
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
        return SHADOW_ACTION
    end,
}

return guards
