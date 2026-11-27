--!strict
-- EXE3 Complete Navi Chips (79 chips: all V1-V5 Navis + Roll V1-V3 + Punk).

local merge = require("@exelib/merge")

return merge {
    require("@self/part1"),
    require("@self/part2"),
}
