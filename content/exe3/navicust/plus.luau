--!strict
-- EXE3 NaviCust plus programs (smooth blocks: must NOT sit on the Command Line; shapes from EXE3 ROM 0x08039420).

local programs = require("../rules/navicust/programs")

local hp_100: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "...##..",
        "..##...",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(100) },
}

local hp_200: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..###..",
        "...##..",
        ".......",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(200) },
}

local hp_300: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
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
    effects = { programs.hp(300) },
}

local hp_500: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        ".#####.",
        ".#.###.",
        ".......",
        ".......",
    },
    bug = "hp",
    effects = { programs.hp(500) },
}

local atk_1: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        "..##...",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.attack(1) },
}

local speed_1: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.rapid(1) },
}

local charge_1: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "...#...",
        ".......",
        ".......",
        ".......",
    },
    bug = "buster",
    effects = { programs.charge(1) },
}

local reg_5: NaviCustProgram = {
    colors = { "white", "pink", "yellow" },
    plus = true,
    shape = {
        ".......",
        ".......",
        ".......",
        "..###..",
        ".......",
        ".......",
        ".......",
    },
    bug = "custom",
    effects = { programs.reg_up(5) },
}

return {
    ["hp-100"] = hp_100,
    ["hp-200"] = hp_200,
    ["hp-300"] = hp_300,
    ["hp-500"] = hp_500,
    ["atk-1"] = atk_1,
    ["speed-1"] = speed_1,
    ["charge-1"] = charge_1,
    ["reg-5"] = reg_5,
}
