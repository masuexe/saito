--!strict
-- EXE3 rulesets: Masters (standard PvP) and Senior (no Giga/HubBatc restricted preset).

return {
    categories = {
        { fact = "folder" },
        { fact = "navicust_programs" },
    },
    presets = {
        { id = "masters", versions = "one_per_build" },
        { id = "senior", versions = "mixed" },
    },
}
