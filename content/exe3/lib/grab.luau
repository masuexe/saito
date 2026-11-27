--!strict
-- EXE3's AreaGrab and PanlGrab dimming controller over @exelib/grab.

local collision = require("../rules/collision")
local controller = require("@exelib/grab/controller")
local panels = require("@exelib/panels")
local shot = require("@exelib/grab/shot")
local grab_shot_sprite = asset.sprite("grab-shot")

local shot_kind = shot.make_kind {
    id = "grab/shot",
    sprite = grab_shot_sprite,
    fall_sound = asset.sound("grab-shot-fall"),
    land_sound = asset.sound("grab-shot-land"),
    landing_steps = 20,
    target = collision.hits_navis,
    self_type = collision.thrown,
    spares = "back_column",
}

local OWN = { [0] = { require = 0, forbid = 0x20 }, [1] = { require = 0x20, forbid = 0 } }

local function closest_of_side(side: number, y: number): number
    local own = OWN[side]
    return panels.closest_in_row(side, y, own.require, own.forbid)
end

local function column(me: Object): number?
    local other = bit32.bxor(me.alliance, 1)
    local own = OWN[other]
    local x = panels.front_most(other, own.require, own.forbid)
    return if x ~= 0 then x else nil
end

local function panel(me: Object): (number?, number?)
    local y = me.panel_y
    local x = closest_of_side(me.alliance, y)
    if x == 0 then
        return nil, nil
    end
    x += me.facing
    if not field.valid(x, y) then
        return nil, nil
    end
    return x, y
end

return controller.make { id = "grab/controller", shot = shot_kind, column = column, panel = panel }
