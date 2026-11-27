--!strict
-- MegaMan's charged shots in EXE3: Normal buster charge plus elemental (Heat/Aqua/Elec/Wood) and Ground Style charge.

local attachment = require("../../../objects/attachment")
local collision = require("../../../rules/collision")
local projectile = require("../../../lib/projectile")
local shot = require("../../../objects/projectile")
local sparks = require("../../../lib/sparks")

local charged_shot = {}

local SHOOT, RECOVER = 0x0, 0x4
local SHOT_Z = 0x18_0000
local NAVI_NUMBER = 0
local SHOT_SOUND = asset.sound("buster-shot")
local CHARGE_TICKS = { 120, 100, 85, 70, 60, 60 }

local NULL_SHOT = shot.variant {
    id = "megaman/charged-shot/null",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}

local HEAT_SHOT = shot.variant {
    id = "megaman/charged-shot/heat",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "fire",
    spark = sparks.fire,
}

local AQUA_SHOT = shot.variant {
    id = "megaman/charged-shot/aqua",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "aqua",
    spark = sparks.aqua,
}

local ELEC_SHOT = shot.variant {
    id = "megaman/charged-shot/elec",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 1,
    element = "elec",
    spark = sparks.elec,
}

local WOOD_SHOT = shot.variant {
    id = "megaman/charged-shot/wood",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 1,
    element = "wood",
    spark = sparks.wood,
}

type State = { timer: number }

local function make_action(id: string, variant: ProjectileVariant, cracks: boolean?): Action
    return new.action {
        id = id,
        state = { timer = "i16" },
        update = function(me: Object, s: State)
            local step = me.step
            if step == SHOOT then
                if me.step_init == 0 then
                    me:set_animation(0x12)
                    me.step_init = 4
                    me:set_status("using_action", true)
                    s.timer = 0
                end
                if s.timer == 1 then
                    battle.play_sound(SHOT_SOUND)
                    me.attack_count = projectile.fire(me, { variant = variant, damage = me.attack_damage, z = SHOT_Z })
                    if cracks then
                        local tx = me.panel_x + me.facing * 2
                        if field.valid(tx, me.panel_y) then
                            field.crack(tx, me.panel_y)
                        end
                    end
                    attachment.attach(me, attachment.muzzle_flash, "related", NAVI_NUMBER)
                end
                s.timer += 1
                if s.timer > 4 then
                    me.step = RECOVER
                    me.step_init = 0
                end
            elseif step == RECOVER then
                if me.step_init == 0 then
                    s.timer = me.attack_count
                    me.step_init = 4
                end
                s.timer -= 1
                if s.timer < 0 then
                    me.related1 = nil
                    me:exit_attack(true)
                end
                if not me:can_move() then
                    return
                end
                local dir = me:held_direction()
                if dir == 0 or me:step_target(dir) == nil then
                    return
                end
                me.related1 = nil
                me:exit_attack()
                me:start_move(dir)
            else
                error(string.format("charged shot phase %#x out of range", step))
            end
        end,
    }
end

local NULL_ACTION = make_action("megaman/charged-shot/action", NULL_SHOT, false)
local HEAT_ACTION = make_action("megaman/charged-shot/heat-action", HEAT_SHOT, false)
local AQUA_ACTION = make_action("megaman/charged-shot/aqua-action", AQUA_SHOT, false)
local ELEC_ACTION = make_action("megaman/charged-shot/elec-action", ELEC_SHOT, false)
local WOOD_ACTION = make_action("megaman/charged-shot/wood-action", WOOD_SHOT, false)
local GROUND_ACTION = make_action("megaman/charged-shot/ground-action", NULL_SHOT, true)

local function make_weapon(id: string, action: Action, elem: number, base_damage: number, scale: number): Weapon
    return new.weapon {
        id = id,
        charge_ticks = CHARGE_TICKS,
        setup = function(navi: Object): Action
            local atk = battle.navi(navi.alliance).attack
            navi.attack_damage = base_damage + atk * scale
            navi.attack_element = elem
            navi.charged = 0
            navi.extra = 0
            navi.hit_param = 0
            navi.attack_lockout = 0
            return action
        end,
    }
end

charged_shot.weapon = make_weapon("megaman/charged-shot", NULL_ACTION, 0, 10, 10)
charged_shot.heat = make_weapon("megaman/charged-shot/heat", HEAT_ACTION, 1, 50, 10)
charged_shot.aqua = make_weapon("megaman/charged-shot/aqua", AQUA_ACTION, 2, 30, 10)
charged_shot.elec = make_weapon("megaman/charged-shot/elec", ELEC_ACTION, 3, 20, 10)
charged_shot.wood = make_weapon("megaman/charged-shot/wood", WOOD_ACTION, 4, 40, 10)
charged_shot.ground = make_weapon("megaman/charged-shot/ground", GROUND_ACTION, 0, 30, 10)

return charged_shot
