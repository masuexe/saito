--!strict
-- EXE3's 8 NetBattle random stage pool (A6BE 0x0803EAB0, uniform weights).

local stages = require("../stages")

local pool = {
    stages["acdc-net"],
    stages["scilab-net"],
    stages["yoka-net"],
    stages["beach-net"],
    stages["secret-area"],
    stages["undernet"],
    stages["lans-hp"],
    stages["hospital-comp"],
}

return {
    stages = pool,
    first_round_stages = #pool,
    match_stages = pool,
}
