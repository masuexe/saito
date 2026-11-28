--!strict
-- EXE3's battle statuses and status rules:
--   - Emotion Window disabled (`emotion = { enabled = false }`)
--   - Counter Hits disabled (`counter_hits = false`)
--   - Data-driven barrier rules (`barrier.types`), resolving EXE3's ID 8 FirstBarrier (plain 100 HP) vs BubbleWrap (ID 9)

local status = {
    paralyze = new.status {
        id = "paralyze",
        requests = 0x00000008,
        duration = 120,
        timer = "paralyze",
    },
    freeze = new.status {
        id = "freeze",
        requests = 0x00000010,
        duration = 180,
        timer = "freeze",
        cancels_flinch = true,
    },
    bubble = new.status {
        id = "bubble",
        requests = 0x00000020,
        duration = 180,
        timer = "bubble",
        survives_counter = true,
    },
    confuse = new.status {
        id = "confuse",
        requests = 0x00000080,
        duration = 480,
        timer = "confuse",
    },
    blind = new.status {
        id = "blind",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    rules = {
        hp_drain = { periods = "stat", stops_while_paused = false },
        custom_drain = { periods = "stat", status = 0x1 },
        form_tick = false,
        form_keeps_back_special = true,
        flash_hides_on_clear = true,
        missing_collision_status = 0xE3A02004,
        reactions = "flash_timer_first",
        reaction_actions = "plain",
        drag = { poses = { otherwise = 8 }, ending = "keeps_pose" },
        bugs_before_drain = true,
        no_charge_drive = false,
        hp_loss = "hit_drains_gauge",
        hit_sound = "by_navi",
        -- Data-driven barrier rules for EXE3:
        --   ID 1/2/3 = Barrier / Barr100 / Barr200 (plain HP barriers)
        --   ID 6/7   = Aura (100) / LifeAura (200) (threshold auras)
        --   ID 8     = FirstBarrier (0x080B048C: plain 100-HP barrier, NOT BubbleWrap!)
        --   ID 9     = BubbleWrap (1 HP, elec pop, 240-tick regrow)
        --   ID 0x10  = Popped state
        barrier = {
            stops_while_paused = false,
            wind = "takes_away",
            types = {
                { id = 1, wind_pops = true, threshold_aura = false },
                { id = 2, wind_pops = true, threshold_aura = false },
                { id = 3, wind_pops = true, threshold_aura = false },
                { id = 4, wind_pops = true, threshold_aura = false },
                { id = 6, wind_pops = true, threshold_aura = true },
                { id = 7, wind_pops = true, threshold_aura = true },
                { id = 10, wind_pops = true, threshold_aura = true },
                { id = 8, wind_pops = true, bubble = false, elec_pops = false, regrow = false, threshold_aura = false },
                {
                    id = 9,
                    wind_pops = true,
                    bubble = true,
                    elec_pops = true,
                    regrow = true,
                    regrow_ticks = 240,
                    regrow_hp = 1,
                    regrow_blocked_by_status = true,
                    threshold_aura = false,
                },
                { id = 0x10, popped = true },
            },
        },
        anti_traps = { damage_min = 1, armed_swallows_below = false, sword_spares = 0, drops_requests = 0 },
        chip_destruction = "hand",
        -- Gen-1 (EXE1-3): no Emotion Window and no Counter Hits.
        emotion = { enabled = false },
        counter_hits = false,
        form_break = "any_form",
        weakness_hit_breaks_form = false,
        weakness_mark = "weak_element_damage",
        paused_navi = "stops_at_control",
        timers_while_paused = true,
        idle_stands = true,
        idle_asks_controller = false,
    },
}

return status
