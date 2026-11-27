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
    colors = { "white" },
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

return {
    sprarmor = sprarmor,
    custom1 = custom1,
    custom2 = custom2,
    megfldr1 = megfldr1,
    megfldr2 = megfldr2,
    gigfldr1 = gigfldr1,
    shield = shield,
    reflect = reflect,
    flotshoe = flotshoe,
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
}
