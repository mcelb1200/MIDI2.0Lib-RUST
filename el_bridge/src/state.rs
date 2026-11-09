//! Bounded state tracking for MIDI 1.0 aggregation (14-bit CCs, RPN/NRPN, SysEx).

use heapless::Vec;

/// Maximum size for incoming MIDI 1.0 SysEx buffer before flushing/aborting.
pub const SYSEX_MAX_CAPACITY: usize = 256;

/// Aggregation state for 14-bit CC pairs (CC 0-31 MSB, CC 32-63 LSB).
#[derive(Copy, Clone, Debug, Default)]
pub struct Cc14BitState {
    pub msb: [Option<u8>; 32],
    pub lsb: [Option<u8>; 32],
}

impl Cc14BitState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            msb: [None; 32],
            lsb: [None; 32],
        }
    }

    pub fn reset(&mut self) {
        self.msb = [None; 32];
        self.lsb = [None; 32];
    }
}

/// Active parameter number type (RPN vs NRPN).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParameterType {
    Rpn,
    Nrpn,
}

/// Aggregation state for RPN and NRPN parameter sequences per channel.
/// Sequences:
/// RPN: CC 101 (MSB), CC 100 (LSB), CC 6 (Data MSB), CC 38 (Data LSB optional)
/// NRPN: CC 99 (MSB), CC 98 (LSB), CC 6 (Data MSB), CC 38 (Data LSB optional)
#[derive(Copy, Clone, Debug, Default)]
pub struct ParameterState {
    pub active_param_type: Option<ParameterType>,
    pub param_msb: Option<u8>,
    pub param_lsb: Option<u8>,
    pub data_msb: Option<u8>,
    pub data_lsb: Option<u8>,
}

impl ParameterState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_param_type: None,
            param_msb: None,
            param_lsb: None,
            data_msb: None,
            data_lsb: None,
        }
    }

    pub fn reset(&mut self) {
        self.active_param_type = None;
        self.param_msb = None;
        self.param_lsb = None;
        self.data_msb = None;
        self.data_lsb = None;
    }

    #[must_use]
    pub fn is_param_selected(&self) -> bool {
        self.active_param_type.is_some() && self.param_msb.is_some() && self.param_lsb.is_some()
    }
}

/// Aggregation state for incoming SysEx 7-bit bytes with bounded capacity.
#[derive(Clone, Debug)]
pub struct SysExAccumulator {
    pub active: bool,
    pub buffer: Vec<u8, SYSEX_MAX_CAPACITY>,
}

impl Default for SysExAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl SysExAccumulator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active: false,
            buffer: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.buffer.clear();
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.buffer.clear();
    }

    pub fn push(&mut self, byte: u8) -> Result<(), u8> {
        self.buffer.push(byte)
    }
}

/// Aggregate channel-level state tracker across all 16 MIDI channels.
#[derive(Clone, Debug, Default)]
pub struct ChannelStateTracker {
    pub cc14: [Cc14BitState; 16],
    pub param: [ParameterState; 16],
    pub sysex: SysExAccumulator,
}

impl ChannelStateTracker {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cc14: [const { Cc14BitState::new() }; 16],
            param: [const { ParameterState::new() }; 16],
            sysex: SysExAccumulator::new(),
        }
    }

    pub fn reset(&mut self) {
        for ch in 0..16 {
            self.cc14[ch].reset();
            self.param[ch].reset();
        }
        self.sysex.reset();
    }
}
