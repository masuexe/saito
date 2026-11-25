--!strict
-- EXE4 build legality data: items that exist in ROM tables (so replays and `validate` still accept them)
-- but cannot be obtained through normal gameplay.

return {
    unobtainable = {
        navicust_colors = {
            megfldr1 = { "pink" },
            attckmax = { "yellow", "pink" },
            speedmax = { "yellow", "pink" },
            chargmax = { "white", "yellow" },
            ["hp-500"] = { "yellow" },
        },
    },
}
