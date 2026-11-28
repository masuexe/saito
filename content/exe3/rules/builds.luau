--!strict
-- EXE3's builds: White and Blue versions as groups, facts kept at rules defaults, and the authentic 30-chip
-- starter folder that a new EXE3 save begins with (`saito-match`'s `build`). The simulation reads none of it.

return {
    -- The versions, as groups (docs/design/versions.md §2.2): White's five Giga chips (`navrcycl`, `bass`,
    -- `serenade`, `balance`, `alpharmsg`) and Blue's five Giga chips (`foldrbak`, `deltaray`, `darkaura`,
    -- `alpharmom`, `bass-137`). A group's `color` (0xRRGGBB) is drawn in build/ruleset lists; names live in
    -- locales (`[builds]`, `version.<id>`).
    groups = {
        { id = "white", color = 0xE6EEF8 },
        { id = "blue", color = 0x3D8BFF },
    },
    -- Pre-battle Style Change (save byte 0x1881: `(style_type << 3) | style_element`): Normal Style is always
    -- non-elemental (`null`); every Style Change pairs one of the 7 styles with one of the 4 elements.
    choices = {
        {
            id = "style_type",
            navi = "megaman",
            fact = "style_type",
            presets = {
                { id = "normal", set = { style_type = "normal" } },
                { id = "guts", set = { style_type = "guts" } },
                { id = "custom", set = { style_type = "custom" } },
                { id = "team", set = { style_type = "team" } },
                { id = "shield", set = { style_type = "shield" } },
                { id = "ground", set = { style_type = "ground" } },
                { id = "shadow", set = { style_type = "shadow" } },
                { id = "bug", set = { style_type = "bug" } },
            },
        },
        {
            id = "style_element",
            navi = "megaman",
            fact = "style_element",
            unless = { fact = "style_type", value = "normal" },
            presets = {
                { id = "heat", set = { style_element = "heat" } },
                { id = "aqua", set = { style_element = "aqua" } },
                { id = "elec", set = { style_element = "elec" } },
                { id = "wood", set = { style_element = "wood" } },
            },
        },
    },
    -- Facts every competitive build keeps at the rules' defaults: full HP Memory base HP (1000) and max Regular
    -- memory (50 MB).
    at_defaults = { "hp", "reg_up" },
    -- A new build starts with MegaMan in Normal Style and the starter folder copied on NEW GAME from the ROM's
    -- initial folder table (`0x0800CBDC` in White US `A6BE` and Blue US `A3XE`; `0x0800CB58` in Japanese `A6BJ`
    -- and `A3XJ` — identical across all four Legacy Collection Vol. 1 ROMs).
    new = {
        folder = {
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "B" },
            { chip = "cannon", code = "B" },
            { chip = "shotgun", code = "J" },
            { chip = "shotgun", code = "J" },
            { chip = "shotgun", code = "J" },
            { chip = "v-gun", code = "D" },
            { chip = "v-gun", code = "D" },
            { chip = "v-gun", code = "D" },
            { chip = "sidegun", code = "S" },
            { chip = "airshot1", code = "*" },
            { chip = "airshot1", code = "*" },
            { chip = "airshot1", code = "*" },
            { chip = "minibomb", code = "B" },
            { chip = "minibomb", code = "B" },
            { chip = "minibomb", code = "S" },
            { chip = "sword", code = "L" },
            { chip = "sword", code = "L" },
            { chip = "sword", code = "L" },
            { chip = "wideswrd", code = "L" },
            { chip = "panlout1", code = "B" },
            { chip = "panlout1", code = "B" },
            { chip = "areagrab", code = "L" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "L" },
            { chip = "recov10", code = "L" },
            { chip = "atk-10", code = "*" },
            { chip = "atk-10", code = "*" },
        },
    },
}
