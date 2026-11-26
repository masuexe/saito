--!strict
-- EXE5 build legality data:
-- - `unobtainable`: cannot be obtained in either GBA or Double Team DS normal play.
-- - `ds_only`: unobtainable in GBA, legitimately obtainable in Double Team DS (Lotto Numbers, BN3 Slot-2 merchant,
--   or native DS features such as Party Battle / NavChg / Spport). Controlled by the ruleset's `ds` (`allow_ds`) flag.

return {
    unobtainable = {
        chip_codes = {
            drksonic = { "*" },
        },
        navicust_colors = {
            megfldr1 = { "pink" },
            fstbarr = { "red", "green", "white" },
            hubbatc = { "white" },
            attckmax = { "white", "pink" },
            speedmax = { "white", "yellow" },
            chargmax = { "pink", "yellow" },
        },
    },
    ds_only = {
        chip_codes = {
            elemrage = { "*" },
            lavaseed = { "*" },
            sword = { "*" },
            wavepit = { "*" },
            redwave = { "*" },
            mudwave = { "*" },
            numbrbl = { "*" },
            drkrecov = { "*" },
        },
        navicust_colors = {
            ["speed-1"] = { "yellow" },
            ["charge-1"] = { "pink" },
            ["hp-500"] = { "yellow" },
        },
        navicust_programs = {
            "navchg_1",
            "navchg_2",
            "spport",
        },
        patch_cards = {
            "sol-cross-megaman",
        },
        facts = {
            "party",
            "party_programs",
        },
    },
}
