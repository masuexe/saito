--!strict
-- EXE3's 8 NetBattle stages (A6BE 0x0803EAB0: ACDC Net, SciLab Net, Yoka Net, Beach Net, Secret Area, Undernet,
-- Lan's HP, Hospital Comp).

local stages: { [string]: StageDef } = {
    ["acdc-net"] = {
        layout = {
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["scilab-net"] = {
        layout = {
            "normal cracked normal normal cracked normal",
            "normal normal normal normal normal normal",
            "normal cracked normal normal cracked normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["yoka-net"] = {
        layout = {
            "grass grass normal normal grass grass",
            "normal normal normal normal normal normal",
            "grass grass normal normal grass grass",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["beach-net"] = {
        layout = {
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["secret-area"] = {
        layout = {
            "missing normal normal normal normal missing",
            "normal normal cracked cracked normal normal",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["undernet"] = {
        layout = {
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["lans-hp"] = {
        layout = {
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    ["hospital-comp"] = {
        layout = {
            "normal grass grass grass grass normal",
            "normal normal normal normal normal normal",
            "normal grass grass grass grass normal",
        },
        panel_pattern = 0x38,
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
}

return stages
