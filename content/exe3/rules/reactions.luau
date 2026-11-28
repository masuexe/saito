--!strict
-- EXE3's reactions and Gen-1 max-index hit modifier table (A6BE 0x080AF3B0: indices 0..=5).

return {
    -- Gen-1 (EXE1-3): hit modifiers merge by max() and decode via lookup table (0x080AF3B0: [0, 1, 0/2, 3, 5, 7]).
    hit_modifier = {
        mode = "max_index_table",
        read_pushes = false,
        table = {
            -- 0: no flinch, no flash (buster)
            { flinch = false, flash = false },
            -- 1: flinch without mercy flash (multi-hit / light flinch)
            { flinch = true, flash = false },
            -- 2: mercy flash without flinch
            { flinch = false, flash = true },
            -- 3: flinch + mercy flash (Cannon, Sword, Bombs)
            { flinch = true, flash = true },
            -- 4: flinch ignoring SuperArmor (heavy stagger)
            { flinch = true, flash = false, ignores_super_armor = true },
            -- 5: flinch + mercy flash ignoring SuperArmor
            { flinch = true, flash = true, ignores_super_armor = true },
        },
    },
    push_reading = {
        reads = "final",
        bits = 6,
        drag_bit = 0x01,
        obstacle_rows = {
            { dx = -1, dy = 0, panels = 6 },
            { dx = 1, dy = 0, panels = 6 },
            { dx = -1, dy = 0, panels = 1 },
            { dx = 1, dy = 0, panels = 1 },
            { dx = 0, dy = -1, panels = 1 },
            { dx = 0, dy = 1, panels = 1 },
            { dx = 0, dy = 0, panels = 0 },
        },
        obstacle_move_end = true,
    },
    hit_test = {
        float_shoe_needs_self_bit = false,
        bubbled_as_submerged = false,
        elec_reaches_submerged = false,
        guard_breaks_to = 0x1002,
        guard_before_untouchable = true,
        guard_marks_direction = false,
    },
    move_direction = "by_side",
    obstacle_slide_bounds = false,
    request_clears = {
        attack = {},
        paralysis = { "anti_sword_triggered" },
        flinch = { "anti_sword_triggered" },
        drag = { "anti_sword_triggered" },
        cut_in = {},
    },
    status_end = {
        flags = { "paralyzed", "blind", "immobilized", "confused" },
        requests = 0x8068,
        timers = { "paralyze", "confuse", "blind", "immobilize" },
    },
    push = {
        { dx = 1, dy = 0, panels = 6 },
        { dx = -1, dy = 0, panels = 6 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
    },
    -- Slide direction vectors when stepping onto an Ice panel (`move_end = panel_types.ice_slides`),
    -- indexed by `by_side` move direction: none, up, down, back, forward, other.
    slide_rows = {
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = 0, dy = 0, panels = 0 },
    },
    bubble_bob = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 },
    slide_speed = { x = 0xA_0000, y = 0x8_0000 },
    overlay_restart = "reload",
    stance_counter = "next_tick",
    trap_counter = "next_tick",
    idle_entry_counter = false,
    dead_player = "freed",
    attack_end_lockout = "chip_lockout",
}
