//! Standard Embedded Profile Handlers (Session 7.2)
//!
//! Provides zero-heap, embedded-friendly profile implementations for common MIDI-CI profiles
//! (M2-102: Common Rules for MIDI-CI Profiles):
//! - Default Profile
//! - Drawbar Organ Profile
//! - Analog Synth Profile
//! - General MIDI 1 & 2 Profiles
//!
//! Includes hooks for parameter updates and profile-specific data callbacks without heap allocation.

use crate::ci::ProfileId;

/// Standard Profile Definitions (M2-102).
/// Standard profile identifiers always start with `0x7E`.
pub const PROFILE_DEFAULT: ProfileId = ProfileId::standard(0x00, 0x01, 0x01, 0x00);
pub const PROFILE_DRAWBAR_ORGAN: ProfileId = ProfileId::standard(0x01, 0x01, 0x01, 0x00);
pub const PROFILE_ANALOG_SYNTH: ProfileId = ProfileId::standard(0x02, 0x01, 0x01, 0x00);
pub const PROFILE_GENERAL_MIDI_1: ProfileId = ProfileId::standard(0x03, 0x01, 0x01, 0x00);
pub const PROFILE_GENERAL_MIDI_2: ProfileId = ProfileId::standard(0x03, 0x02, 0x01, 0x00);

/// Enumeration of well-known standard profile kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardProfileKind {
    Default,
    DrawbarOrgan,
    AnalogSynth,
    GeneralMidi1,
    GeneralMidi2,
    Custom(ProfileId),
}

impl StandardProfileKind {
    /// Maps a `ProfileId` to a `StandardProfileKind`.
    #[must_use]
    #[inline]
    pub const fn from_profile_id(id: ProfileId) -> Self {
        if id.raw[0] == PROFILE_DEFAULT.raw[0]
            && id.raw[1] == PROFILE_DEFAULT.raw[1]
            && id.raw[2] == PROFILE_DEFAULT.raw[2]
            && id.raw[3] == PROFILE_DEFAULT.raw[3]
            && id.raw[4] == PROFILE_DEFAULT.raw[4]
        {
            StandardProfileKind::Default
        } else if id.raw[0] == PROFILE_DRAWBAR_ORGAN.raw[0]
            && id.raw[1] == PROFILE_DRAWBAR_ORGAN.raw[1]
            && id.raw[2] == PROFILE_DRAWBAR_ORGAN.raw[2]
            && id.raw[3] == PROFILE_DRAWBAR_ORGAN.raw[3]
            && id.raw[4] == PROFILE_DRAWBAR_ORGAN.raw[4]
        {
            StandardProfileKind::DrawbarOrgan
        } else if id.raw[0] == PROFILE_ANALOG_SYNTH.raw[0]
            && id.raw[1] == PROFILE_ANALOG_SYNTH.raw[1]
            && id.raw[2] == PROFILE_ANALOG_SYNTH.raw[2]
            && id.raw[3] == PROFILE_ANALOG_SYNTH.raw[3]
            && id.raw[4] == PROFILE_ANALOG_SYNTH.raw[4]
        {
            StandardProfileKind::AnalogSynth
        } else if id.raw[0] == PROFILE_GENERAL_MIDI_1.raw[0]
            && id.raw[1] == PROFILE_GENERAL_MIDI_1.raw[1]
            && id.raw[2] == PROFILE_GENERAL_MIDI_1.raw[2]
            && id.raw[3] == PROFILE_GENERAL_MIDI_1.raw[3]
            && id.raw[4] == PROFILE_GENERAL_MIDI_1.raw[4]
        {
            StandardProfileKind::GeneralMidi1
        } else if id.raw[0] == PROFILE_GENERAL_MIDI_2.raw[0]
            && id.raw[1] == PROFILE_GENERAL_MIDI_2.raw[1]
            && id.raw[2] == PROFILE_GENERAL_MIDI_2.raw[2]
            && id.raw[3] == PROFILE_GENERAL_MIDI_2.raw[3]
            && id.raw[4] == PROFILE_GENERAL_MIDI_2.raw[4]
        {
            StandardProfileKind::GeneralMidi2
        } else {
            StandardProfileKind::Custom(id)
        }
    }

    /// Returns the associated `ProfileId`.
    #[must_use]
    #[inline]
    pub const fn profile_id(&self) -> ProfileId {
        match *self {
            StandardProfileKind::Default => PROFILE_DEFAULT,
            StandardProfileKind::DrawbarOrgan => PROFILE_DRAWBAR_ORGAN,
            StandardProfileKind::AnalogSynth => PROFILE_ANALOG_SYNTH,
            StandardProfileKind::GeneralMidi1 => PROFILE_GENERAL_MIDI_1,
            StandardProfileKind::GeneralMidi2 => PROFILE_GENERAL_MIDI_2,
            StandardProfileKind::Custom(id) => id,
        }
    }
}

/// Bounded drawbar settings for Drawbar Organ profile (9 drawbars, 0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawbarOrganState {
    /// 9 standard drawbar positions: 16', 5 1/3', 8', 4', 2 2/3', 2', 1 3/5', 1 1/3', 1'
    pub drawbars: [u8; 9],
    /// Percussion enabled
    pub percussion: bool,
    /// Vibrato/Chorus setting (0 = off, 1-3 = V1-V3, 4-6 = C1-C3)
    pub vibrato_chorus: u8,
    /// Rotary speaker speed (0 = off/slow, 1 = fast)
    pub rotary_fast: bool,
}

impl Default for DrawbarOrganState {
    fn default() -> Self {
        Self {
            drawbars: [8, 8, 8, 0, 0, 0, 0, 0, 0], // standard 888000000 jazz registration
            percussion: false,
            vibrato_chorus: 0,
            rotary_fast: false,
        }
    }
}

/// Standard analog synth parameters for Analog Synth profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalogSynthState {
    pub filter_cutoff: u16,
    pub filter_resonance: u16,
    pub attack_time: u16,
    pub decay_time: u16,
    pub sustain_level: u16,
    pub release_time: u16,
    pub lfo_rate: u16,
    pub lfo_depth: u16,
}

impl Default for AnalogSynthState {
    fn default() -> Self {
        Self {
            filter_cutoff: 0x8000,
            filter_resonance: 0x2000,
            attack_time: 0x0400,
            decay_time: 0x4000,
            sustain_level: 0x8000,
            release_time: 0x4000,
            lfo_rate: 0x2000,
            lfo_depth: 0x0000,
        }
    }
}

/// Trait defining the lifecycle and control hooks for an embedded profile handler.
pub trait ProfileHandler {
    /// Returns the Profile ID associated with this handler.
    fn profile_id(&self) -> ProfileId;

    /// Called when the profile is enabled on a target channel / function block.
    fn on_enable(&mut self, channel: u8) -> bool {
        let _ = channel;
        true
    }

    /// Called when the profile is disabled on a target channel / function block.
    fn on_disable(&mut self, channel: u8) {
        let _ = channel;
    }

    /// Called when profile-specific data is received via MIDI-CI Profile Specific Data (0x2F).
    ///
    /// Returns an optional reply payload length if a response is needed, or 0.
    fn on_specific_data(
        &mut self,
        channel: u8,
        data: &[u8],
        reply_buffer: &mut [u8],
    ) -> Option<usize> {
        let _ = (channel, data, reply_buffer);
        None
    }
}

/// An embedded profile descriptor tracking state across up to 16 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileState {
    pub profile_id: ProfileId,
    pub enabled_channel_mask: u16,
}

impl ProfileState {
    /// Creates a new inactive profile state for `profile_id`.
    #[must_use]
    #[inline]
    pub const fn new(profile_id: ProfileId) -> Self {
        Self {
            profile_id,
            enabled_channel_mask: 0,
        }
    }

    /// Checks if the profile is enabled on `channel` (0..15).
    #[must_use]
    #[inline]
    pub fn is_enabled_on_channel(&self, channel: u8) -> bool {
        if channel < 16 {
            (self.enabled_channel_mask & (1 << channel)) != 0
        } else {
            false
        }
    }

    /// Enables the profile on `channel` (0..15).
    #[inline]
    pub fn enable_channel(&mut self, channel: u8) {
        if channel < 16 {
            self.enabled_channel_mask |= 1 << channel;
        }
    }

    /// Disables the profile on `channel` (0..15).
    #[inline]
    pub fn disable_channel(&mut self, channel: u8) {
        if channel < 16 {
            self.enabled_channel_mask &= !(1 << channel);
        }
    }

    /// Returns `true` if this profile is enabled on at least one channel.
    #[must_use]
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled_channel_mask != 0
    }
}

/// A zero-allocation profile manager managing up to `MAX_PROFILES` profiles for an embedded device.
#[derive(Debug)]
pub struct ProfileManager<const MAX_PROFILES: usize> {
    profiles: [Option<ProfileState>; MAX_PROFILES],
    organ_state: DrawbarOrganState,
    synth_state: AnalogSynthState,
}

impl<const MAX_PROFILES: usize> Default for ProfileManager<MAX_PROFILES> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_PROFILES: usize> ProfileManager<MAX_PROFILES> {
    /// Creates a new `ProfileManager` initialized with default standard profile states.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            profiles: [None; MAX_PROFILES],
            organ_state: DrawbarOrganState {
                drawbars: [8, 8, 8, 0, 0, 0, 0, 0, 0],
                percussion: false,
                vibrato_chorus: 0,
                rotary_fast: false,
            },
            synth_state: AnalogSynthState {
                filter_cutoff: 0x8000,
                filter_resonance: 0x2000,
                attack_time: 0x0400,
                decay_time: 0x4000,
                sustain_level: 0x8000,
                release_time: 0x4000,
                lfo_rate: 0x2000,
                lfo_depth: 0x0000,
            },
        }
    }

    /// Registers a supported profile ID. Returns `true` if registered or already registered.
    pub fn register_profile(&mut self, profile_id: ProfileId) -> bool {
        for p in self.profiles.iter_mut() {
            if let Some(existing) = p {
                if existing.profile_id == profile_id {
                    return true;
                }
            } else {
                *p = Some(ProfileState::new(profile_id));
                return true;
            }
        }
        false
    }

    /// Enables a profile on a channel. Returns `true` if profile is supported.
    pub fn enable_profile(&mut self, profile_id: ProfileId, channel: u8) -> bool {
        for p in self.profiles.iter_mut().flatten() {
            if p.profile_id == profile_id {
                p.enable_channel(channel);
                return true;
            }
        }
        false
    }

    /// Disables a profile on a channel.
    pub fn disable_profile(&mut self, profile_id: ProfileId, channel: u8) -> bool {
        for p in self.profiles.iter_mut().flatten() {
            if p.profile_id == profile_id {
                p.disable_channel(channel);
                return true;
            }
        }
        false
    }

    /// Populates slices with enabled and disabled `ProfileId`s for Profile Inquiry Reply.
    pub fn collect_profiles(
        &self,
        enabled_out: &mut [ProfileId],
        disabled_out: &mut [ProfileId],
    ) -> (usize, usize) {
        let mut en_count = 0;
        let mut dis_count = 0;

        for p in self.profiles.iter().flatten() {
            if p.is_active() {
                if en_count < enabled_out.len() {
                    enabled_out[en_count] = p.profile_id;
                    en_count += 1;
                }
            } else if dis_count < disabled_out.len() {
                disabled_out[dis_count] = p.profile_id;
                dis_count += 1;
            }
        }

        (en_count, dis_count)
    }

    /// Access mutable drawbar organ state.
    #[inline]
    pub fn organ_mut(&mut self) -> &mut DrawbarOrganState {
        &mut self.organ_state
    }

    /// Access drawbar organ state.
    #[must_use]
    #[inline]
    pub fn organ(&self) -> &DrawbarOrganState {
        &self.organ_state
    }

    /// Access mutable analog synth state.
    #[inline]
    pub fn synth_mut(&mut self) -> &mut AnalogSynthState {
        &mut self.synth_state
    }

    /// Access analog synth state.
    #[must_use]
    #[inline]
    pub fn synth(&self) -> &AnalogSynthState {
        &self.synth_state
    }
}
