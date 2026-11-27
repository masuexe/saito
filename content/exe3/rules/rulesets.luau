--!strict
-- EXE3's rulesets: what a ruleset may hold back of a build, and ready-made tournament presets (`saito-match`'s
-- `ruleset`). The simulation reads none of it. EXE3 has no patch cards and MegaMan is the only playable Navi, so
-- rulesets regulate folder chips and NaviCust programs. Version groups (`white` / `blue` in `./builds`) partition
-- the version-exclusive Giga chips; names live in locales (`[rulesets]`, `category.<fact>`, `preset.<id>`).

return {
    categories = {
        { fact = "folder" },
        { fact = "navicust_programs" },
    },
    -- Masters Class: standard single-version cartridge rules (White or Blue per build, all chips and NaviCust
    -- programs allowed).
    -- Senior Class: single-version restricted tournament preset banning all Giga chips (including FoldrBak,
    -- NavRcycl, DarkAura, Bass/Bass+, Serenade, Balance, DeltaRay, AlphaArmΣ/Ω) and the HubBatc NaviCust program.
    presets = {
        { id = "masters", versions = "one_per_build" },
        {
            id = "senior",
            versions = "one_per_build",
            ban = {
                folder = {
                    -- White Giga chips
                    "navrcycl",
                    "bass",
                    "serenade",
                    "balance",
                    "alpharmsg",
                    -- Blue Giga chips
                    "foldrbak",
                    "deltaray",
                    "darkaura",
                    "alpharmom",
                    "bass-137",
                    -- Event / V5 Giga chips
                    "bassgs",
                    "gutsmanv5",
                    "protomnv5",
                    "flashmnv5",
                    "beastmnv5",
                    "bubblmnv5",
                    "desrtmnv5",
                    "plantmnv5",
                    "flammanv5",
                    "drillmnv5",
                    "metalmnv5",
                    "kingmanv5",
                    "mistmanv5",
                    "bowlmanv5",
                    "darkmanv5",
                    "japanmnv5",
                },
                navicust_programs = {
                    "hubbatc",
                },
            },
        },
    },
}
