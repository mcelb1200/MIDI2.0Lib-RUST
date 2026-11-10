//! Embedded MIDI 2.0 Device Engine & High-Level Runtime (Session 7.1)
//!
//! Provides `Midi2Device<T, NUM_FB>`, a turnkey embedded coordinator integrating:
//! - UMP Endpoint Discovery and Notifications (MT=0xF)
//! - Function Block topology (up to `NUM_FB` function blocks)
//! - MIDI-CI State Machine (Initiator & Responder)
//! - Protocol switching (MIDI 1.0 <-> MIDI 2.0)
//! - Standard Embedded Profile Handlers (Session 7.2)
//! - Standard Property Exchange Resource Providers (Session 7.3)
//! - Zero-dynamic-allocation polling loop `poll()`

use crate::ci::{
    CiDeviceInfo, CiHeader, CiInitiator, CiProfile, CiProtocolNegotiation, CiResponder,
    DeviceIdentity, DeviceInquiry, ProfileId, MIDI_PROTOCOL_1, MIDI_PROTOCOL_2, S7DEVICE_INQUIRY,
    S7MIDICI, S7UNIVERSAL_NRT, SUBID2_IDENTITY_REQUEST,
};
use crate::ci_pe::{
    hash_resource, CiPe, PeError, PeSchemaValidator, SubscriptionManager, MIDICI_PE_GET,
    MIDICI_PE_GETREPLY, MIDICI_PE_SET, MIDICI_PE_SETREPLY, MIDICI_PE_SUB, MIDICI_PE_SUBREPLY,
};
use crate::profile::ProfileManager;
use crate::resource::{
    write_channel_list_json, write_device_state_json, write_external_sync_json,
    write_local_on_json, write_mode_json, write_paginated_controller_list_json,
    write_paginated_program_list_json, write_resource_list_json, ControllerDefinition,
    DeviceInfoResource, DeviceMode, DeviceStateResource, ProgramItem, StandardResource, SyncSource,
};
use crate::stream::{
    FunctionBlock, StreamConfigStateMachine, StreamFactory, StreamTextChunker,
    STATUS_ENDPOINT_DISCOVERY, STATUS_FUNCTION_BLOCK_DISCOVERY, STATUS_STREAM_CONFIG_NOTIFICATION,
    STATUS_STREAM_CONFIG_REQUEST,
};
use crate::sysex::{SysEx7Chunker, SysEx7Receiver, SysEx7ReceiverState};
use crate::transport::{Transport, TransportError};
use crate::ump::{MessageType, Ump};

/// Active device protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveProtocol {
    Midi1_0,
    Midi2_0,
}

/// Configuration options for initializing a `Midi2Device`.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<'a> {
    pub name: &'a str,
    pub product_instance_id: &'a str,
    pub manufacturer: &'a str,
    pub model: &'a str,
    pub serial_number: &'a str,
    pub software_revision: &'a str,
    pub local_muid: u32,
    pub ci_info: CiDeviceInfo,
}

/// Errors returned during device polling or configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceError {
    Transport(TransportError),
    Pe(PeError),
    BufferFull,
}

impl From<TransportError> for DeviceError {
    #[inline]
    fn from(e: TransportError) -> Self {
        DeviceError::Transport(e)
    }
}

/// Turnkey Embedded MIDI 2.0 Device Runtime.
pub struct Midi2Device<'a, T: Transport, const NUM_FB: usize> {
    transport: T,
    config: DeviceConfig<'a>,
    active_protocol: ActiveProtocol,
    stream_negotiator: StreamConfigStateMachine,
    function_blocks: [Option<FunctionBlock>; NUM_FB],
    function_blocks_count: usize,
    ci_initiator: CiInitiator,
    ci_responder: CiResponder,
    profile_manager: ProfileManager<8>,
    subscription_manager: SubscriptionManager<8>,
    sysex_receiver: SysEx7Receiver<512>,
    rx_jr_enabled: bool,
    tx_jr_enabled: bool,
    local_on: bool,
    mode: DeviceMode,
    sync_source: SyncSource,
    sync_active: bool,
    device_state_revision: u32,
}

impl<'a, T: Transport, const NUM_FB: usize> Midi2Device<'a, T, NUM_FB> {
    /// Creates a new `Midi2Device` instance with the given transport and configuration.
    #[must_use]
    pub fn new(transport: T, config: DeviceConfig<'a>) -> Self {
        let ci_initiator = CiInitiator::new(config.local_muid, config.ci_info);
        let ci_responder = CiResponder::new(config.local_muid, config.ci_info);

        Self {
            transport,
            config,
            active_protocol: ActiveProtocol::Midi1_0, // starts default MIDI 1.0 until negotiated/configured
            stream_negotiator: StreamConfigStateMachine::new(),
            function_blocks: [None; NUM_FB],
            function_blocks_count: 0,
            ci_initiator,
            ci_responder,
            profile_manager: ProfileManager::<8>::new(),
            subscription_manager: SubscriptionManager::<8>::new(),
            sysex_receiver: SysEx7Receiver::<512>::new(),
            rx_jr_enabled: false,
            tx_jr_enabled: false,
            local_on: true,
            mode: DeviceMode::Standard,
            sync_source: SyncSource::Internal,
            sync_active: true,
            device_state_revision: 1,
        }
    }

    /// Registers a Function Block on this device.
    pub fn add_function_block(&mut self, fb: FunctionBlock) -> bool {
        if self.function_blocks_count < NUM_FB {
            self.function_blocks[self.function_blocks_count] = Some(fb);
            self.function_blocks_count += 1;
            true
        } else {
            false
        }
    }

    /// Registers a standard or custom profile supported by this device.
    pub fn register_profile(&mut self, profile_id: ProfileId) -> bool {
        self.ci_responder.add_supported_profile(profile_id);
        self.profile_manager.register_profile(profile_id)
    }

    /// Access mutable profile manager (e.g. for adjusting drawbar organ or synth parameters).
    #[inline]
    pub fn profile_manager_mut(&mut self) -> &mut ProfileManager<8> {
        &mut self.profile_manager
    }

    /// Access profile manager.
    #[must_use]
    #[inline]
    pub fn profile_manager(&self) -> &ProfileManager<8> {
        &self.profile_manager
    }

    /// Returns the currently active protocol.
    #[must_use]
    #[inline]
    pub fn active_protocol(&self) -> ActiveProtocol {
        self.active_protocol
    }

    /// Sets the active protocol explicitly.
    #[inline]
    pub fn set_active_protocol(&mut self, proto: ActiveProtocol) {
        self.active_protocol = proto;
        match proto {
            ActiveProtocol::Midi1_0 => {
                self.stream_negotiator.reset();
            }
            ActiveProtocol::Midi2_0 => {
                let notif =
                    StreamFactory::stream_config_notify(2, self.rx_jr_enabled, self.tx_jr_enabled);
                self.stream_negotiator.process_ump(&notif);
            }
        }
    }

    /// Access the Stream Configuration state machine.
    #[inline]
    pub fn stream_negotiator(&self) -> &StreamConfigStateMachine {
        &self.stream_negotiator
    }

    /// Access mutable Stream Configuration state machine.
    #[inline]
    pub fn stream_negotiator_mut(&mut self) -> &mut StreamConfigStateMachine {
        &mut self.stream_negotiator
    }

    /// Returns `true` if MIDI 2.0 messages (MT=0x4) are currently accepted.
    #[must_use]
    #[inline]
    pub fn can_process_midi2(&self) -> bool {
        self.active_protocol == ActiveProtocol::Midi2_0
            || self.stream_negotiator.can_process_midi2()
    }

    /// Returns the device's local MUID.
    #[must_use]
    #[inline]
    pub fn local_muid(&self) -> u32 {
        self.config.local_muid
    }

    /// Access CI Initiator state machine.
    #[inline]
    pub fn ci_initiator_mut(&mut self) -> &mut CiInitiator {
        &mut self.ci_initiator
    }

    /// Access CI Responder state machine.
    #[inline]
    pub fn ci_responder_mut(&mut self) -> &mut CiResponder {
        &mut self.ci_responder
    }

    /// Access underlying transport.
    #[inline]
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// High-level single polling step.
    ///
    /// Consumes an incoming UMP from the transport, processes Stream requests,
    /// reassembles SysEx7 / MIDI-CI packets, updates internal state machines,
    /// and emits any required responses directly back through the transport.
    pub fn poll(&mut self) -> Result<bool, DeviceError> {
        let maybe_ump = self.transport.recv()?;
        let Some(ump) = maybe_ump else {
            return Ok(false);
        };

        self.handle_incoming_ump(ump)?;
        Ok(true)
    }

    /// Ticks the device clock with `current_time` to check for stream negotiation timeouts.
    pub fn tick(&mut self, current_time: u32) {
        if self.stream_negotiator.tick(current_time) {
            self.active_protocol = ActiveProtocol::Midi1_0;
        }
    }

    /// Dispatches an incoming UMP packet.
    fn handle_incoming_ump(&mut self, ump: Ump) -> Result<(), DeviceError> {
        match ump.message_type() {
            MessageType::Stream => self.handle_stream_message(ump),
            MessageType::SysEx7 => self.handle_sysex7_message(ump),
            _ => Ok(()),
        }
    }

    /// Handles incoming UMP Stream messages (MT=0xF).
    fn handle_stream_message(&mut self, ump: Ump) -> Result<(), DeviceError> {
        let status = ((ump.data[0] >> 16) & 0x3FF) as u16;

        match status {
            STATUS_ENDPOINT_DISCOVERY => {
                let filter_bitmap = ((ump.data[1] >> 24) & 0xFF) as u8;
                self.send_endpoint_discovery_replies(filter_bitmap)?;
            }
            STATUS_STREAM_CONFIG_REQUEST => {
                let protocol_code = ((ump.data[0] >> 8) & 0xFF) as u8;
                let rxjr = (ump.data[0] & 0x2) != 0;
                let txjr = (ump.data[0] & 0x1) != 0;

                if protocol_code == 2 {
                    self.active_protocol = ActiveProtocol::Midi2_0;
                } else if protocol_code == 1 {
                    self.active_protocol = ActiveProtocol::Midi1_0;
                }
                self.rx_jr_enabled = rxjr;
                self.tx_jr_enabled = txjr;

                let notify = StreamFactory::stream_config_notify(protocol_code, rxjr, txjr);
                self.stream_negotiator.process_ump(&notify);
                self.transport.send(&[notify])?;
            }
            STATUS_STREAM_CONFIG_NOTIFICATION => {
                self.stream_negotiator.process_ump(&ump);
                if self.stream_negotiator.active_protocol() == 2 {
                    self.active_protocol = ActiveProtocol::Midi2_0;
                } else {
                    self.active_protocol = ActiveProtocol::Midi1_0;
                }
                self.rx_jr_enabled = self.stream_negotiator.rx_jr_enabled();
                self.tx_jr_enabled = self.stream_negotiator.tx_jr_enabled();
            }
            STATUS_FUNCTION_BLOCK_DISCOVERY => {
                let fb_idx = ((ump.data[0] >> 8) & 0xFF) as u8;
                let filter_bitmap = (ump.data[0] & 0xFF) as u8;
                self.send_function_block_discovery_replies(fb_idx, filter_bitmap)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Sends Endpoint Discovery response packets according to `filter_bitmap`.
    fn send_endpoint_discovery_replies(&mut self, filter: u8) -> Result<(), DeviceError> {
        // Bit 0: Endpoint Info
        if (filter & 0x01) != 0 {
            let info = StreamFactory::endpoint_info_notify(
                1,
                1,
                self.function_blocks_count as u8,
                true, // MIDI 2.0 supported
                true, // MIDI 1.0 supported
                self.rx_jr_enabled,
                self.tx_jr_enabled,
            );
            self.transport.send(&[info])?;
        }

        // Bit 1: Device Identity
        if (filter & 0x02) != 0 {
            let dev_id = StreamFactory::endpoint_device_info_notify(
                self.config.ci_info.manufacturer_id,
                self.config.ci_info.family_id,
                self.config.ci_info.model_id,
                self.config.ci_info.version,
            );
            self.transport.send(&[dev_id])?;
        }

        // Bit 2: Endpoint Name
        if (filter & 0x04) != 0 {
            let chunker = StreamTextChunker::endpoint_name(self.config.name.as_bytes());
            for pkt in chunker {
                self.transport.send(&[pkt])?;
            }
        }

        // Bit 3: Product Instance ID
        if (filter & 0x08) != 0 {
            let chunker =
                StreamTextChunker::product_instance_id(self.config.product_instance_id.as_bytes());
            for pkt in chunker {
                self.transport.send(&[pkt])?;
            }
        }

        Ok(())
    }

    /// Sends Function Block Discovery responses according to `fb_idx` and `filter_bitmap`.
    fn send_function_block_discovery_replies(
        &mut self,
        fb_idx: u8,
        filter: u8,
    ) -> Result<(), DeviceError> {
        for i in 0..self.function_blocks_count {
            if let Some(fb) = self.function_blocks[i] {
                if fb_idx == 0xFF || fb_idx == fb.fb_idx {
                    // Bit 0: Info Notification
                    if (filter & 0x01) != 0 {
                        let info_ump = fb.to_info_notification();
                        self.transport.send(&[info_ump])?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles incoming SysEx7 UMP packets (accumulates and routes MIDI-CI).
    fn handle_sysex7_message(&mut self, ump: Ump) -> Result<(), DeviceError> {
        let group = ump.group();
        let is_complete = matches!(
            self.sysex_receiver.process_ump(&ump),
            Ok(SysEx7ReceiverState::Complete)
        );

        if is_complete {
            let mut temp_payload = [0_u8; 512];
            let len = self.sysex_receiver.payload().len().min(512);
            temp_payload[..len].copy_from_slice(&self.sysex_receiver.payload()[..len]);
            self.sysex_receiver.reset();
            self.handle_completed_sysex(group, &temp_payload[..len])?;
        }
        Ok(())
    }

    /// Handles a complete, reassembled SysEx payload.
    fn handle_completed_sysex(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        if payload.is_empty() {
            return Ok(());
        }

        // Check Universal Non-Real-Time SysEx header
        if payload[0] != S7UNIVERSAL_NRT {
            return Ok(());
        }

        // Handle General Information / Device Identity Request (Sub-ID 1 = 0x06, Sub-ID 2 = 0x01)
        if payload.len() >= 4
            && payload[2] == S7DEVICE_INQUIRY
            && payload[3] == SUBID2_IDENTITY_REQUEST
        {
            let dev_identity = DeviceIdentity {
                device_id: payload[1] & 0x7F,
                manufacturer_id: self.config.ci_info.manufacturer_id,
                family_id: self.config.ci_info.family_id,
                model_id: self.config.ci_info.model_id,
                version: self.config.ci_info.version,
            };
            let reply = DeviceInquiry::build_identity_reply(&dev_identity);
            self.send_sysex7_payload(group, &reply)?;
            return Ok(());
        }

        if payload.len() < 13 {
            return Ok(());
        }

        // Check MIDI-CI Sub-ID
        if payload[2] != S7MIDICI {
            return Ok(());
        }

        let Some(hdr) = CiHeader::parse(payload) else {
            return Ok(());
        };

        // Dispatch based on CI Type
        match hdr.ci_type {
            crate::ci::MIDICI_DISCOVERY => {
                if let Some((reply_buf, len)) = self.ci_responder.handle_discovery_inquiry(payload)
                {
                    self.send_sysex7_payload(group, &reply_buf[..len])?;
                }
            }
            crate::ci::MIDICI_PROTOCOL_NEGOTIATION => {
                if let Some((reply_buf, len)) = self.ci_responder.handle_protocol_inquiry(payload) {
                    self.send_sysex7_payload(group, &reply_buf[..len])?;
                }
            }
            crate::ci::MIDICI_PROTOCOL_SET => {
                if payload.len() >= 19 {
                    let mut proto = [0_u8; 5];
                    proto.copy_from_slice(&payload[14..19]);
                    if proto == MIDI_PROTOCOL_2 {
                        self.active_protocol = ActiveProtocol::Midi2_0;
                    } else if proto == MIDI_PROTOCOL_1 {
                        self.active_protocol = ActiveProtocol::Midi1_0;
                    }
                    let confirm = CiProtocolNegotiation::build_protocol_confirm_payload(
                        self.config.local_muid,
                        hdr.source_muid,
                        payload[13],
                    );
                    self.send_sysex7_payload(group, &confirm)?;
                }
            }
            crate::ci::MIDICI_PROFILE_INQUIRY => {
                let mut en_profiles = [ProfileId::new([0; 5]); 8];
                let mut dis_profiles = [ProfileId::new([0; 5]); 8];
                let (en_count, dis_count) = self
                    .profile_manager
                    .collect_profiles(&mut en_profiles, &mut dis_profiles);

                let (reply_buf, len) = CiProfile::build_inquiry_reply_payload(
                    self.config.local_muid,
                    hdr.source_muid,
                    hdr.device_id,
                    hdr.ci_version,
                    &en_profiles[..en_count],
                    &dis_profiles[..dis_count],
                );
                self.send_sysex7_payload(group, &reply_buf[..len])?;
            }
            crate::ci::MIDICI_PROFILE_SETON => {
                if let Some((_, profile, num_channels)) = CiProfile::parse_profile_message(payload)
                {
                    let channel = hdr.device_id & 0xF;
                    self.profile_manager.enable_profile(profile, channel);
                    let (rep, len) = CiProfile::build_profile_enabled_report(
                        self.config.local_muid,
                        hdr.source_muid,
                        hdr.device_id,
                        hdr.ci_version,
                        profile,
                        num_channels,
                    );
                    self.send_sysex7_payload(group, &rep[..len])?;
                }
            }
            crate::ci::MIDICI_PROFILE_SETOFF => {
                if let Some((_, profile, num_channels)) = CiProfile::parse_profile_message(payload)
                {
                    let channel = hdr.device_id & 0xF;
                    self.profile_manager.disable_profile(profile, channel);
                    let (rep, len) = CiProfile::build_profile_disabled_report(
                        self.config.local_muid,
                        hdr.source_muid,
                        hdr.device_id,
                        hdr.ci_version,
                        profile,
                        num_channels,
                    );
                    self.send_sysex7_payload(group, &rep[..len])?;
                }
            }
            crate::ci::MIDICI_PROFILE_DETAILS_INQUIRY => {
                if let Some((_, profile, target)) =
                    CiProfile::parse_profile_details_inquiry(payload)
                {
                    let mut details_buf = [0_u8; 32];
                    let details_len = if profile == crate::profile::PROFILE_DRAWBAR_ORGAN {
                        details_buf[0] = 9; // 9 drawbars supported
                        1
                    } else if profile == crate::profile::PROFILE_ANALOG_SYNTH {
                        details_buf[0] = 2; // 2 oscillators
                        details_buf[1] = 1; // 1 LFO
                        2
                    } else {
                        0
                    };

                    let (rep, len) = CiProfile::build_profile_details_reply(
                        self.config.local_muid,
                        hdr.source_muid,
                        hdr.device_id,
                        hdr.ci_version,
                        profile,
                        target,
                        &details_buf[..details_len],
                    );
                    self.send_sysex7_payload(group, &rep[..len])?;
                }
            }
            crate::ci::MIDICI_PROFILE_ADD => {
                if let Some((_, profile, _)) = CiProfile::parse_profile_message(payload) {
                    self.register_profile(profile);
                }
            }
            crate::ci::MIDICI_PROFILE_REMOVE => {
                if let Some((_, profile, _)) = CiProfile::parse_profile_message(payload) {
                    self.profile_manager.disable_profile(profile, 0);
                }
            }
            MIDICI_PE_GET => {
                self.handle_pe_get(group, payload)?;
            }
            MIDICI_PE_SET => {
                self.handle_pe_set(group, payload)?;
            }
            MIDICI_PE_SUB => {
                self.handle_pe_subscribe(group, payload)?;
            }
            _ => {
                // Pass to initiator if it's waiting for replies
                let _ = self.ci_initiator.process_message(payload);
            }
        }

        Ok(())
    }

    /// Handles a PE Set request with Schema Validation.
    fn handle_pe_set(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        let Some((hdr, pe_msg)) = CiPe::parse_pe_message(payload) else {
            return Ok(());
        };

        let resource_name = pe_msg.header.resource.unwrap_or("");
        let standard_res = StandardResource::parse_name(resource_name);
        let body = pe_msg.body.unwrap_or(b"");

        let status_code: u16 = match standard_res {
            Some(StandardResource::LocalOn) => {
                if PeSchemaValidator::validate_local_on(body).is_ok() {
                    let tok = crate::ci_pe::JsonTokenizer::new(body);
                    for t in tok {
                        if let crate::ci_pe::JsonTokenType::BoolValue(b) = t {
                            self.local_on = b;
                            break;
                        }
                    }
                    200
                } else {
                    400 // Bad request / Schema validation failure
                }
            }
            Some(StandardResource::Mode) => {
                if PeSchemaValidator::validate_mode(body).is_ok() {
                    let tok = crate::ci_pe::JsonTokenizer::new(body);
                    for t in tok {
                        if let crate::ci_pe::JsonTokenType::StringValue(s) = t {
                            match s {
                                "Standard" => self.mode = DeviceMode::Standard,
                                "MPE" => self.mode = DeviceMode::Mpe,
                                "MultiChannel" => self.mode = DeviceMode::MultiChannel,
                                _ => {}
                            }
                            break;
                        }
                    }
                    200
                } else {
                    400
                }
            }
            Some(StandardResource::ExternalSync) => {
                if PeSchemaValidator::validate_external_sync(body).is_ok() {
                    let tok = crate::ci_pe::JsonTokenizer::new(body);
                    let mut cur_key: Option<&str> = None;
                    for t in tok {
                        match t {
                            crate::ci_pe::JsonTokenType::Key(k) => cur_key = Some(k),
                            crate::ci_pe::JsonTokenType::StringValue(s) => {
                                if cur_key == Some("source") {
                                    match s {
                                        "Internal" => self.sync_source = SyncSource::Internal,
                                        "MIDI" => self.sync_source = SyncSource::Midi,
                                        "SMPTE" => self.sync_source = SyncSource::Smpte,
                                        "Audio" => self.sync_source = SyncSource::Audio,
                                        _ => {}
                                    }
                                }
                                cur_key = None;
                            }
                            crate::ci_pe::JsonTokenType::BoolValue(b) => {
                                if cur_key == Some("active") {
                                    self.sync_active = b;
                                }
                                cur_key = None;
                            }
                            _ => {}
                        }
                    }
                    200
                } else {
                    400
                }
            }
            Some(StandardResource::DeviceState) => {
                if PeSchemaValidator::validate_device_state(body).is_ok() {
                    let tok = crate::ci_pe::JsonTokenizer::new(body);
                    let mut cur_key: Option<&str> = None;
                    for t in tok {
                        match t {
                            crate::ci_pe::JsonTokenType::Key(k) => cur_key = Some(k),
                            crate::ci_pe::JsonTokenType::NumberValue(n) => {
                                if cur_key == Some("revision") && n >= 0 {
                                    self.device_state_revision = n as u32;
                                }
                                cur_key = None;
                            }
                            _ => {}
                        }
                    }
                    200
                } else {
                    400
                }
            }
            _ => 404, // Unsupported resource for Set
        };

        let mut pe_out = [0_u8; 128];
        let json_hdr = match status_code {
            200 => b"{\"status\":200}",
            400 => b"{\"status\":400}",
            _ => b"{\"status\":404}",
        };

        let pe_len = CiPe::serialize_pe_header_only(
            &mut pe_out,
            MIDICI_PE_SETREPLY,
            hdr.ci_version,
            self.config.local_muid,
            hdr.source_muid,
            pe_msg.request_id,
            json_hdr,
        )
        .map_err(DeviceError::Pe)?;

        self.send_sysex7_payload(group, &pe_out[..pe_len])?;
        Ok(())
    }

    /// Handles a PE Get request (Session 7.3).
    fn handle_pe_get(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        let Some((hdr, pe_msg)) = CiPe::parse_pe_message(payload) else {
            return Ok(());
        };

        let resource_name = pe_msg.header.resource.unwrap_or("");
        let standard_res = StandardResource::parse_name(resource_name);
        let offset = pe_msg.header.offset.unwrap_or(0) as usize;
        let limit = pe_msg.header.limit.unwrap_or(128) as usize;

        let mut body_buf = [0_u8; 256];
        let body_len_opt = match standard_res {
            Some(StandardResource::DeviceInfo) => {
                let dev_res = DeviceInfoResource::new(
                    self.config.manufacturer,
                    self.config.model,
                    self.config.serial_number,
                    self.config.software_revision,
                    self.config.ci_info,
                );
                dev_res.write_json(&mut body_buf)
            }
            Some(StandardResource::ResourceList) => {
                let list = [
                    "DeviceInfo",
                    "ResourceList",
                    "ChannelList",
                    "Mode",
                    "ProgramList",
                    "LocalOn",
                    "DeviceState",
                    "ExternalSync",
                    "ControllerList",
                ];
                write_resource_list_json(&list, &mut body_buf)
            }
            Some(StandardResource::ChannelList) => {
                // Return channels active across function blocks
                let mut mask = 0_u16;
                for fb in self.function_blocks.iter().flatten() {
                    for g in 0..fb.group_length {
                        mask |= 1 << ((fb.first_group + g) % 16);
                    }
                }
                if mask == 0 {
                    mask = 0x0001;
                }
                write_channel_list_json(mask, &mut body_buf)
            }
            Some(StandardResource::Mode) => write_mode_json(self.mode, &mut body_buf),
            Some(StandardResource::ProgramList) => {
                let default_programs = [
                    ProgramItem {
                        program: 0,
                        bank_msb: 0,
                        bank_lsb: 0,
                        name: "Default Preset",
                    },
                    ProgramItem {
                        program: 1,
                        bank_msb: 0,
                        bank_lsb: 0,
                        name: "Acoustic Piano",
                    },
                    ProgramItem {
                        program: 2,
                        bank_msb: 0,
                        bank_lsb: 0,
                        name: "Drawbar Organ",
                    },
                ];
                write_paginated_program_list_json(&default_programs, offset, limit, &mut body_buf)
            }
            Some(StandardResource::LocalOn) => write_local_on_json(self.local_on, &mut body_buf),
            Some(StandardResource::DeviceState) => {
                let state_res = DeviceStateResource {
                    state_title: "Active Device Profile",
                    revision: self.device_state_revision,
                    memory_used_bytes: 4096,
                    total_memory_bytes: 65536,
                };
                write_device_state_json(&state_res, &mut body_buf)
            }
            Some(StandardResource::ExternalSync) => {
                write_external_sync_json(self.sync_source, self.sync_active, &mut body_buf)
            }
            Some(StandardResource::ControllerList) => {
                let default_controllers = [
                    ControllerDefinition {
                        index: 1,
                        name: "Modulation Wheel",
                        default_val: 0,
                        unit: "ratio",
                    },
                    ControllerDefinition {
                        index: 7,
                        name: "Channel Volume",
                        default_val: 100,
                        unit: "ratio",
                    },
                    ControllerDefinition {
                        index: 10,
                        name: "Pan",
                        default_val: 64,
                        unit: "ratio",
                    },
                ];
                write_paginated_controller_list_json(
                    &default_controllers,
                    offset,
                    limit,
                    &mut body_buf,
                )
            }
            _ => None,
        };

        let mut pe_out = [0_u8; 384];
        let pe_len = if let Some(body_len) = body_len_opt {
            let json_hdr = b"{\"status\":200}";
            CiPe::serialize_pe_with_body(
                &mut pe_out,
                MIDICI_PE_GETREPLY,
                hdr.ci_version,
                self.config.local_muid,
                hdr.source_muid,
                pe_msg.request_id,
                json_hdr,
                1,
                1,
                &body_buf[..body_len],
            )
            .map_err(DeviceError::Pe)?
        } else {
            let json_hdr = b"{\"status\":404}";
            CiPe::serialize_pe_header_only(
                &mut pe_out,
                MIDICI_PE_GETREPLY,
                hdr.ci_version,
                self.config.local_muid,
                hdr.source_muid,
                pe_msg.request_id,
                json_hdr,
            )
            .map_err(DeviceError::Pe)?
        };

        self.send_sysex7_payload(group, &pe_out[..pe_len])?;
        Ok(())
    }

    /// Handles a PE Subscribe request.
    fn handle_pe_subscribe(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        let Some((hdr, pe_msg)) = CiPe::parse_pe_message(payload) else {
            return Ok(());
        };

        let resource_name = pe_msg.header.resource.unwrap_or("");
        let h = hash_resource(resource_name);
        let ok = self
            .subscription_manager
            .add_subscription(hdr.source_muid, h);

        let mut pe_out = [0_u8; 128];
        let json_hdr = if ok {
            b"{\"status\":200}"
        } else {
            b"{\"status\":404}"
        };

        let pe_len = CiPe::serialize_pe_header_only(
            &mut pe_out,
            MIDICI_PE_SUBREPLY,
            hdr.ci_version,
            self.config.local_muid,
            hdr.source_muid,
            pe_msg.request_id,
            json_hdr,
        )
        .map_err(DeviceError::Pe)?;

        self.send_sysex7_payload(group, &pe_out[..pe_len])?;
        Ok(())
    }

    /// Chunks a complete payload into SysEx7 UMP packets and transmits them.
    fn send_sysex7_payload(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        let chunker = SysEx7Chunker::new(group, payload);
        for ump in chunker {
            self.transport.send(&[ump])?;
        }
        Ok(())
    }
}
