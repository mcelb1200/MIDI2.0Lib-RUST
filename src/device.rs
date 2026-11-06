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
    CiDeviceInfo, CiHeader, CiInitiator, CiProfile, CiProtocolNegotiation, CiResponder, ProfileId,
    MIDI_PROTOCOL_1, MIDI_PROTOCOL_2, S7MIDICI, S7UNIVERSAL_NRT,
};
use crate::ci_pe::{
    hash_resource, CiPe, PeError, SubscriptionManager, MIDICI_PE_GET, MIDICI_PE_GETREPLY,
    MIDICI_PE_SUB, MIDICI_PE_SUBREPLY,
};
use crate::profile::ProfileManager;
use crate::resource::{
    write_channel_list_json, write_resource_list_json, DeviceInfoResource, StandardResource,
};
use crate::stream::{
    FunctionBlock, StreamFactory, StreamTextChunker, STATUS_ENDPOINT_DISCOVERY,
    STATUS_FUNCTION_BLOCK_DISCOVERY, STATUS_STREAM_CONFIG_REQUEST,
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
    function_blocks: [Option<FunctionBlock>; NUM_FB],
    function_blocks_count: usize,
    ci_initiator: CiInitiator,
    ci_responder: CiResponder,
    profile_manager: ProfileManager<8>,
    subscription_manager: SubscriptionManager<8>,
    sysex_receiver: SysEx7Receiver<512>,
    rx_jr_enabled: bool,
    tx_jr_enabled: bool,
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
            function_blocks: [None; NUM_FB],
            function_blocks_count: 0,
            ci_initiator,
            ci_responder,
            profile_manager: ProfileManager::<8>::new(),
            subscription_manager: SubscriptionManager::<8>::new(),
            sysex_receiver: SysEx7Receiver::<512>::new(),
            rx_jr_enabled: false,
            tx_jr_enabled: false,
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
                self.transport.send(&[notify])?;
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
        if payload.len() < 13 {
            return Ok(());
        }

        // Check Universal Non-Real-Time SysEx header & MIDI-CI Sub-ID
        if payload[0] != S7UNIVERSAL_NRT || payload[2] != S7MIDICI {
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
            MIDICI_PE_GET => {
                self.handle_pe_get(group, payload)?;
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

    /// Handles a PE Get request (Session 7.3).
    fn handle_pe_get(&mut self, group: u8, payload: &[u8]) -> Result<(), DeviceError> {
        let Some((hdr, pe_msg)) = CiPe::parse_pe_message(payload) else {
            return Ok(());
        };

        let resource_name = pe_msg.header.resource.unwrap_or("");
        let standard_res = StandardResource::parse_name(resource_name);

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
                let list = ["DeviceInfo", "ResourceList", "ChannelList"];
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
