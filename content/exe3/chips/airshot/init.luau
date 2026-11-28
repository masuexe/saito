--!strict
-- EXE3 AirShot1, AirShot2, AirShot3 (compat/chips.toml IDs 0x004..0x006).
-- Fires a blast of compressed air down the row that damages and knocks the target back 1 panel!

local anims = require("../lib/anims")
local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local SHOOTER_LOOK = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0 }
local WIND_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local WIND_SOUND = asset.sound("spreader")
local WIND_ELEM = element.byte("null", { "wind" })

type AirShotState = { timer: number, recovery: number }

local function push_back_target(me: Object, tx: number, ty: number)
    local opp_side = bit32.bxor(me.alliance, 1)
    for _, opp in battle.alive_actors(opp_side) do
        if opp.panel_x == tx and opp.panel_y == ty and not opp:status("dead") and not opp:status("super_armor") then
            local nx = opp.panel_x + me.facing
            if field.valid(nx, ty) then
                local p = field.panel(nx, ty)
                if p and p.alliance == opp.alliance and p.kind ~= "missing" and p.kind ~= "broken" and opp:can_move() then
                    opp.panel_x = nx
                    opp:set_coordinates_from_panel()
                    opp:update_collision_panels()
                end
            end
        end
    end
end

local function make_airshot_action(id: string, recovery: number): Action
    return new.action {
        id = id,
        state = { timer = "u16", recovery = "u8" },
        update = function(me: Object, s: AirShotState)
            if me.step_init == 0 then
                me:set_animation(anims.AIRSHOT)
                me:open_counter_window()
                me:set_status("using_action", true)
                attachment.attach(me, SHOOTER_LOOK, "related", 0)
                battle.play_sound(WIND_SOUND)
                s.timer = 0
                s.recovery = recovery
                me.step_init = 4
                return
            end
            if s.timer == 5 then
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                local opp_side = bit32.bxor(me.alliance, 1)
                local hit_x: number? = nil
                local x = me.panel_x + me.facing
                while field.valid(x, me.panel_y) do
                    local found = false
                    for _, opp in battle.alive_actors(opp_side) do
                        if opp.panel_x == x and opp.panel_y == me.panel_y and not opp:status("dead") then
                            found = true
                            break
                        end
                    end
                    if not found then
                        for _, obj in field.all_objects() do
                            if obj.active and obj.panel_x == x and obj.panel_y == me.panel_y then
                                found = true
                                break
                            end
                        end
                    end
                    if found then
                        hit_x = x
                        break
                    end
                    x += me.facing
                end
                local tx = hit_x or (me.panel_x + me.facing)
                if field.valid(tx, me.panel_y) then
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = WIND_ELEM,
                        z = 0x14_0000,
                        region = regions.single,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.attack,
                        damage = damage,
                        hit_mod = 1,
                    })
                    local cx, cy = field.center(tx, me.panel_y)
                    battle.effect(Vec3.new(cx, cy, 0x14_0000), WIND_FX)
                    if hit_x ~= nil then
                        push_back_target(me, tx, me.panel_y)
                    end
                end
            end
            s.timer += 1
            if s.timer >= 10 + s.recovery then
                me.related1 = nil
                me.overlay = nil
                me:exit_attack()
            end
        end,
    }
end

local airshot1: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = make_airshot_action("airshot1/action", 10),
}

local airshot2: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = make_airshot_action("airshot2/action", 10),
}

local airshot3: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = make_airshot_action("airshot3/action", 10),
}

return {
    airshot1 = airshot1,
    airshot2 = airshot2,
    airshot3 = airshot3,
}
