//! Nintendo DS NitroSDK (`SND`) lookup tables for IMA-ADPCM, decibel volume
//! scaling, ADSR attack/decay rates, and LFO sine modulation.

/// Standard IMA-ADPCM 89-entry step size table.
pub const ADPCM_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
    449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
    9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

/// Standard IMA-ADPCM nibble index delta table (indexed by `nibble & 7`).
pub const ADPCM_INDEX_TABLE: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// NitroSDK decibel attenuation table (`0..=127`) in `0.1 dB` units (`0` = full
/// scale, `-723` = `-72.3 dB` silence).
///
/// Exact NitroSDK `sDecibelSquareTable` / `SND_CalcDecibel`:
/// `200 * log10(v / 127)` for `v >= 1`, `-723` for `0`.
pub const DECIBEL_TABLE: [i16; 128] = [
    -723, -421, -361, -325, -300, -281, -265, -252,
    -240, -230, -221, -212, -205, -198, -192, -186,
    -180, -175, -170, -165, -161, -156, -152, -148,
    -145, -141, -138, -134, -131, -128, -125, -122,
    -120, -117, -114, -112, -110, -107, -105, -103,
    -100,  -98,  -96,  -94,  -92,  -90,  -88,  -86,
     -85,  -83,  -81,  -79,  -78,  -76,  -74,  -73,
     -71,  -70,  -68,  -67,  -65,  -64,  -62,  -61,
     -60,  -58,  -57,  -56,  -54,  -53,  -52,  -51,
     -49,  -48,  -47,  -46,  -45,  -43,  -42,  -41,
     -40,  -39,  -38,  -37,  -36,  -35,  -34,  -33,
     -32,  -31,  -30,  -29,  -28,  -27,  -26,  -25,
     -24,  -23,  -23,  -22,  -21,  -20,  -19,  -18,
     -17,  -17,  -16,  -15,  -14,  -13,  -12,  -12,
     -11,  -10,   -9,   -9,   -8,   -7,   -6,   -6,
      -5,   -4,   -3,   -3,   -2,   -1,   -1,    0,
];

/// Silence floor for envelope accumulation (`-723 << 7 = -92544`).
pub const ENV_SILENCE: i32 = -723 << 7;

/// NitroSDK `SND_CalcAttackRate(attack)`: returns the multiplicative attack
/// coefficient (`0..=255`) applied as `env = (coeff * env) >> 8` each 192 Hz
/// driver tick. `0` reaches full volume immediately (`attack >= 127`).
pub fn attack_coeff(attack: u8) -> i32 {
    const HIGH: [u8; 19] = [
        0, 1, 5, 14, 26, 38, 51, 63, 73, 84, 92, 100, 109, 116, 123, 127, 132, 137, 143,
    ];
    let a = attack.min(127);
    if a < 109 {
        (255 - a) as i32
    } else {
        HIGH[(127 - a) as usize] as i32
    }
}

/// NitroSDK `SND_CalcDecayRate(decay)` (used for both decay and release):
/// returns the per-tick subtraction from `env` (in `1/128` of `0.1 dB`).
pub fn decay_rate(val: u8) -> i32 {
    let v = val.min(127);
    if v == 127 {
        0xFFFF
    } else if v == 126 {
        0x3C00
    } else if v < 50 {
        (v as i32 * 2 + 1) & 0xFFFF
    } else {
        0x1E00 / (126 - v as i32)
    }
}

/// Convert a decibel attenuation value in `0.1 dB` units (e.g. `0` = `0 dB`,
/// `-60` = `-6.0 dB`, `-723` = silence) to a linear amplitude `[0.0, 1.0]`.
#[inline]
pub fn db_to_gain(db_tenths: i32) -> f32 {
    if db_tenths <= -723 {
        return 0.0;
    }
    if db_tenths >= 0 {
        return 1.0;
    }
    10.0_f32.powf(db_tenths as f32 / 200.0)
}

/// Quarter-wave NitroComposer LFO sine (`phase` in `0..=127` for a full
/// period, returning `-127..=127`).
pub fn lfo_sin(phase: u8) -> i32 {
    const QUARTER: [u8; 33] = [
        0, 6, 12, 19, 25, 31, 37, 43, 49, 54, 60, 65, 71, 76, 81, 85, 90, 94, 98, 102, 106, 109,
        112, 115, 117, 120, 122, 123, 125, 126, 126, 127, 127,
    ];
    let p = phase & 0x7F;
    if p < 32 {
        QUARTER[p as usize] as i32
    } else if p < 64 {
        QUARTER[(64 - p) as usize] as i32
    } else if p < 96 {
        -(QUARTER[(p - 64) as usize] as i32)
    } else {
        -(QUARTER[(128 - p) as usize] as i32)
    }
}
