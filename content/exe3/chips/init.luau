--!strict
-- EXE3 chips index.

local merge = require("@exelib/merge")

return merge {
    require("@self/airshot"),
    require("@self/barrier"),
    require("@self/bomb"),
    require("@self/cannon"),
    require("@self/gauge"),
    require("@self/geddon"),
    require("@self/grab"),
    require("@self/invalid"),
    require("@self/invis"),
    require("@self/pa"),
    require("@self/panlout"),
    require("@self/plus"),
    require("@self/recov"),
    require("@self/spreader"),
    require("@self/sword"),
    require("@self/zeroed"),
}
