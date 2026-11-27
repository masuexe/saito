--!strict
-- EXE3's MegaMan forms (base Normal Style + 7 Style Change forms).

local merge = require("@exelib/merge")

return merge {
    require("@self/base"),
    require("@self/styles"),
}
