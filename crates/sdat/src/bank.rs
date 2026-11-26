//! Typed data extracted from a Nintendo DS `SDAT` sound archive: sequences
//! (`SSEQ`), instrument banks (`SBNK`), and decoded wave archives (`SWAR`).

/// Index of a sequence (`SSEQ`) inside an SDAT sound bank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SongId(pub u16);

/// One parsed `SSEQ` song record from an SDAT archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Song {
    /// Raw SSEQ bytecode stream (starting after the `DATA` header offset).
    pub data: Vec<u8>,
    /// Index of the instrument bank (`SBNK`) inside [`SoundBank::banks`].
    pub bank: u16,
    /// Sequence master volume from SDAT `INFO` (`0..=127`).
    pub volume: u8,
    /// Channel priority (`0..=127`).
    pub channel_priority: u8,
    /// Player priority (`0..=127`).
    pub player_priority: u8,
    /// Player slot (`0..=31`).
    pub player: u8,
}

/// How an instrument note produces sound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteKind {
    #[default]
    Empty,
    /// PCM / IMA-ADPCM sample inside wave archive slot `swar_slot` (`0..4`).
    Pcm { swav_id: u16, swar_slot: u8 },
    /// Hardware PSG pulse channel with duty cycle `0..=7`.
    Psg { duty: u8 },
    /// Hardware white-noise LFSR channel.
    Noise,
}

/// Parameters for a single instrument note definition (`SBNK`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoteDef {
    pub kind: NoteKind,
    /// Root MIDI key (`0..=127`, typically 60).
    pub base_note: u8,
    /// ADSR attack rate (`0..=127`).
    pub attack: u8,
    /// ADSR decay rate (`0..=127`).
    pub decay: u8,
    /// ADSR sustain level (`0..=127`).
    pub sustain: u8,
    /// ADSR release rate (`0..=127`).
    pub release: u8,
    /// Default instrument pan (`0..=127`, 64 = center).
    pub pan: u8,
}

/// One program entry in an `SBNK` instrument bank.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Instrument {
    #[default]
    Empty,
    Single(NoteDef),
    DrumSet {
        min_note: u8,
        max_note: u8,
        entries: Vec<NoteDef>,
    },
    KeySplit {
        /// Inclusive upper key for each region (`entries[i]` plays for `key <= regions[i]`).
        regions: Vec<u8>,
        entries: Vec<NoteDef>,
    },
}

impl Instrument {
    /// Resolve the [`NoteDef`] played for MIDI key `key`.
    pub fn resolve(&self, key: u8) -> Option<NoteDef> {
        match self {
            Instrument::Empty => None,
            Instrument::Single(def) => (def.kind != NoteKind::Empty).then_some(*def),
            Instrument::DrumSet { min_note, max_note, entries } => {
                if key < *min_note || key > *max_note {
                    return None;
                }
                entries
                    .get((key - *min_note) as usize)
                    .copied()
                    .filter(|d| d.kind != NoteKind::Empty)
            }
            Instrument::KeySplit { regions, entries } => {
                let idx = regions.iter().position(|&max_k| key <= max_k)?;
                entries.get(idx).copied().filter(|d| d.kind != NoteKind::Empty)
            }
        }
    }
}

/// One `SBNK` instrument bank and its up to 4 referenced `SWAR` indices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstrumentBank {
    /// Indices into [`SoundBank::wave_arcs`] (`None` when `0xFFFF`).
    pub wave_arcs: [Option<u16>; 4],
    pub instruments: Vec<Instrument>,
}

/// One decoded `SWAV` waveform sample (`PCM8`, `PCM16`, or `IMA-ADPCM` decoded to `i16`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wave {
    pub samples: Vec<i16>,
    pub sample_rate: u16,
    pub looped: bool,
    pub loop_start: usize,
    pub loop_end: usize,
}

/// One `SWAR` wave archive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaveArchive {
    pub waves: Vec<Wave>,
}

/// Parsed subset (or full set) of an NDS `SDAT` archive ready for playback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SoundBank {
    /// Sequences indexed by `SongId`.
    pub songs: Vec<Option<Song>>,
    /// Instrument banks indexed by `Song::bank`.
    pub banks: Vec<Option<InstrumentBank>>,
    /// Wave archives indexed by `InstrumentBank::wave_arcs`.
    pub wave_arcs: Vec<Option<WaveArchive>>,
}

impl SoundBank {
    pub fn song(&self, id: SongId) -> Option<&Song> {
        self.songs.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub fn bank(&self, id: u16) -> Option<&InstrumentBank> {
        self.banks.get(id as usize).and_then(Option::as_ref)
    }

    pub fn wave(&self, bank: &InstrumentBank, swar_slot: u8, swav_id: u16) -> Option<&Wave> {
        let war_id = (*bank.wave_arcs.get(swar_slot as usize)?)?;
        let war = self.wave_arcs.get(war_id as usize)?.as_ref()?;
        war.waves.get(swav_id as usize)
    }
}
