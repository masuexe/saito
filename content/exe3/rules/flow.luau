--!strict
-- EXE3's battle flow rules: no Final Turn / Damage Judge, no transformation sequencers, joypad custom request.

return {
    result_words = 37,
    sequencer_before_custom = false,
    escape_check = false,
    result_wait = { normal = 102, special = 65 },
    low_hp_music = false,
    navi_win_banner = "never",
    intro_steps_on_init = true,
    sequencer_at_turn_start = false,
    custom_request = "joypads",
    fight_cycles = { 10, 2 },
    final_turn = false,
}
