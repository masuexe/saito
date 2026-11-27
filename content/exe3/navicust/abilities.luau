--!strict
-- EXE3 core NaviCust ability programs (non-plus, textured blocks; shapes from EXE3 ROM 0x08039420).

local barriers = require("../lib/barriers")
local guards = require("../navis/megaman/weapons/guards")
local programs = require("../rules/navicust/programs")

local sprarmor: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...##..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.super_armor() },
}

local custom1: NaviCustProgram = {
    colors = { "yellow", "blue" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(1) },
}

local custom2: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "....#..",
        "...##..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.custom(2) },
}

local megfldr1: NaviCustProgram = {
    colors = { "pink", "green" },
    shape = {
        ".......",
        ".......",
        ".......",
        "..##...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(1) },
}

local megfldr2: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.mega_folder(2) },
}

local gigfldr1: NaviCustProgram = {
    colors = { "purple" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "...#...",
        "..###..",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.giga_folder(1) },
}

local shield: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.back_special(guards.shield) },
}

local reflect: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "..#....",
        "..#....",
        "..###..",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.back_special(guards.reflect) },
}

local flotshoe: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "...#...",
        ".####..",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.float_shoes() },
}

local antidmg: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        "..###..",
        "...#...",
        "...#...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.back_special(guards.anti_dmg) },
}

local airshoes: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..#....",
        "..#.#..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.air_shoes() },
}

local undersht: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.undershirt() },
}

local fstgauge: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        "...##..",
        "...#...",
        "...#...",
        "...#...",
        "...#...",
        ".......",
    },
    bug = "custom",
    effects = { programs.fast_gauge() },
}

local bugstop: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        "..#....",
        "..#....",
        "..#....",
        "..###..",
        ".......",
        ".......",
    },
    effects = { programs.bug_stop() },
}

local bustrmax: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack_max(), programs.rapid_max(), programs.charge_max() },
}

local hubbatc: NaviCustProgram = {
    colors = { "orange", "white" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "hub",
    effects = {
        programs.outside("hub-batch"),
        programs.super_armor(),
        programs.custom(1),
        programs.mega_folder(1),
        programs.first_barrier(barriers.first_barrier),
        programs.back_special(guards.shield),
        programs.float_shoes(),
        programs.air_shoes(),
        programs.undershirt(),
    },
}

local rush: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.rush() },
}

local beat: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...##..",
        "..###..",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.beat() },
}

local tango: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        "..##...",
        "...#...",
        "...#...",
        "..##...",
        ".......",
        ".......",
    },
    bug = "support",
    effects = { programs.tango() },
}

-- Stage-setting NaviCust programs (SetGreen, SetIce, SetLava, SetSand, SetMetal, SetHoly)
local setgreen: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(1) },
}

local setice: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(2) },
}

local setlava: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(3) },
}

local setsand: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(4) },
}

local setmetal: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(5) },
}

local setholy: NaviCustProgram = {
    colors = { "green" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        "...#...",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.set_stage(6) },
}

-- Combat / Guard / Movement abilities
local brakbust: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..#....",
        "..###..",
        "....#..",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.brake_buster() },
}

local brakchrg: NaviCustProgram = {
    colors = { "orange", "red" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..#....",
        "..###..",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.brake_charge() },
}

local shdwshoe: NaviCustProgram = {
    colors = { "red" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        "...##..",
        ".......",
        ".......",
    },
    bug = "panel",
    effects = { programs.shadow_shoes() },
}

local block: NaviCustProgram = {
    colors = { "blue" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "..#....",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.back_special(guards.block) },
}

-- Utility & Overworld programs from compat/navicust.toml
local collect: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..#.#..",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.outside("collect") },
}

local oilbody: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        "...#...",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("oil-body") },
}

local fish: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..#....",
        "..##...",
        "..#....",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("fish") },
}

local battery: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("battery") },
}

local jungle: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("jungle") },
}

local sneakrun: NaviCustProgram = {
    colors = { "yellow" },
    shape = {
        ".......",
        ".......",
        "..##...",
        ".......",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("sneak-run") },
}

local press: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..#.#..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("press") },
}

local engychng: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.outside("energy-change") },
}

local humor: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..#....",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("humor") },
}

local blckmind: NaviCustProgram = {
    colors = { "white" },
    shape = {
        ".......",
        ".......",
        "...#...",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "move",
    effects = { programs.outside("black-mind") },
}

local darklcns: NaviCustProgram = {
    colors = { "dark" },
    shape = {
        ".......",
        ".......",
        "..###..",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.outside("dark-license") },
}

local alpha: NaviCustProgram = {
    colors = { "pink" },
    shape = {
        ".......",
        ".......",
        "..##...",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.outside("alpha") },
}

return {
    sprarmor = sprarmor,
    custom1 = custom1,
    custom2 = custom2,
    megfldr1 = megfldr1,
    megfldr2 = megfldr2,
    gigfldr1 = gigfldr1,
    shield = shield,
    block = block,
    reflect = reflect,
    flotshoe = flotshoe,
    shdwshoe = shdwshoe,
    antidmg = antidmg,
    airshoes = airshoes,
    undersht = undersht,
    fstgauge = fstgauge,
    bugstop = bugstop,
    bustrmax = bustrmax,
    hubbatc = hubbatc,
    rush = rush,
    beat = beat,
    tango = tango,
    setgreen = setgreen,
    setice = setice,
    setlava = setlava,
    setsand = setsand,
    setmetal = setmetal,
    setholy = setholy,
    brakbust = brakbust,
    brakchrg = brakchrg,
    collect = collect,
    oilbody = oilbody,
    fish = fish,
    battery = battery,
    jungle = jungle,
    sneakrun = sneakrun,
    press = press,
    engychng = engychng,
    humor = humor,
    blckmind = blckmind,
    darklcns = darklcns,
    alpha = alpha,
}
