--!strict
-- EXE3 builds: White and Blue versions, Style presets, and starter 30-chip folder.

return {
    groups = {
        { id = "white", color = 0xE6EEF8 },
        { id = "blue", color = 0x3D8BFF },
    },
    at_defaults = { "reg_up" },
    new = {
        folder = {
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "B" },
            { chip = "cannon", code = "C" },
            { chip = "cannon", code = "*" },
            { chip = "airshot1", code = "*" },
            { chip = "airshot1", code = "*" },
            { chip = "airshot1", code = "*" },
            { chip = "shotgun", code = "*" },
            { chip = "shotgun", code = "*" },
            { chip = "shotgun", code = "*" },
            { chip = "v-gun", code = "*" },
            { chip = "v-gun", code = "*" },
            { chip = "spreader", code = "*" },
            { chip = "spreader", code = "*" },
            { chip = "minibomb", code = "*" },
            { chip = "minibomb", code = "*" },
            { chip = "minibomb", code = "*" },
            { chip = "sword", code = "L" },
            { chip = "sword", code = "L" },
            { chip = "sword", code = "S" },
            { chip = "wideswrd", code = "L" },
            { chip = "wideswrd", code = "L" },
            { chip = "longswrd", code = "L" },
            { chip = "panlout1", code = "*" },
            { chip = "panlout1", code = "*" },
            { chip = "recov30", code = "*" },
            { chip = "recov30", code = "*" },
            { chip = "areagrab", code = "*" },
            { chip = "invis", code = "*" },
            { chip = "atk-10", code = "*" },
        },
    },
}
