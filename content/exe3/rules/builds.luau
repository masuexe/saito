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
