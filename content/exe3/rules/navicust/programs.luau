--!strict
-- EXE3 NaviCust program effect constructors and bug groups.

local effects = require("@exelib/navicust/effects")

local programs = {}

export type Effect = NaviCustEffect

programs.super_armor = effects.super_armor
programs.custom = effects.custom
programs.mega_folder = effects.mega_folder
programs.giga_folder = effects.giga_folder
programs.first_barrier = effects.first_barrier
programs.back_special = effects.back_special
programs.float_shoes = effects.float_shoes
programs.air_shoes = effects.air_shoes
programs.undershirt = effects.undershirt
programs.outside = effects.outside
programs.attack = effects.attack
programs.rapid = effects.rapid
programs.charge = effects.charge
programs.attack_max = effects.attack_max
programs.rapid_max = effects.rapid_max
programs.charge_max = effects.charge_max
programs.bug_stop = effects.bug_stop
programs.rush = effects.rush
programs.beat = effects.beat
programs.tango = effects.tango
programs.hp = effects.hp

function programs.fast_gauge(): Effect
    return { kind = "fast_gauge" }
end

function programs.reg_up(n: number): Effect
    return { kind = "reg_up", amount = n }
end

programs.BUGS = {
    "move",
    "hp",
    "custom",
    "panel",
    "buster",
    "support",
    "hub",
}

return programs
