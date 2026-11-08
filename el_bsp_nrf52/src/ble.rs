//! Nordic nRF52 BLE MIDI 2.0 Driver / Packet Framing
//!
//! Implements Bluetooth Low Energy (BLE) MIDI encapsulation over GATT characteristics.
//! Supports:
//! - BLE MIDI packet header with 13-bit millisecond timestamp encapsulation.
//! - Packing and unpacking UMP packets into BLE GATT MTU payloads without dynamic allocation.

use el_midi2::transport::{EmbeddedTransport, Transport, TransportError};
use el_midi2::ump::Ump;
use heapless::spsc::Queue;

/// BLE MIDI packet builder / parser helper.
///
/// In BLE MIDI, packets sent over the BLE MIDI Characteristic have:
/// - Header Byte: `[1, 0, timestamp_high (6 bits)]` (bit 7 is 1, bit 6 is 0)
/// - Followed by timestamp low / status and payload bytes, or UMP words.
pub struct BleMidiPacket;

impl BleMidiPacket {
    /// Builds a BLE MIDI packet containing UMP words with a 13-bit timestamp.
    /// Returns total bytes written to `dest`.
    pub fn pack_ble_ump_packet(timestamp_ms: u16, umps: &[Ump], dest: &mut [u8]) -> Option<usize> {
        let ts_13 = timestamp_ms & 0x1FFF;
        let ts_high = ((ts_13 >> 7) & 0x3F) as u8;
        let ts_low = (ts_13 & 0x7F) as u8;

        // Header byte: bit 7 = 1, bit 6 = 0, bits 5..0 = ts_high
        let header = 0x80 | ts_high;
        // Timestamp low byte: bit 7 = 1, bits 6..0 = ts_low
        let ts_low_byte = 0x80 | ts_low;

        let total_words: usize = umps.iter().map(|u| u.word_count()).sum();
        let payload_len = 2 + (total_words * 4);

        if dest.len() < payload_len {
            return None;
        }

        dest[0] = header;
        dest[1] = ts_low_byte;

        let mut offset = 2;
        for ump in umps {
            let wc = ump.word_count();
            for i in 0..wc {
                let w = ump.data[i];
                let bytes = w.to_le_bytes();
                dest[offset..offset + 4].copy_from_slice(&bytes);
                offset += 4;
            }
        }

        Some(offset)
    }

    /// Unpacks a BLE MIDI packet into UMP words, returning the extracted 13-bit timestamp
    /// and the count of parsed UMPs.
    pub fn unpack_ble_ump_packet<F: FnMut(Ump)>(
        src: &[u8],
        mut on_ump: F,
    ) -> Result<u16, TransportError> {
        if src.len() < 2 {
            return Err(TransportError::HardwareError);
        }

        let header = src[0];
        let ts_low_byte = src[1];

        if (header & 0x80) == 0 || (ts_low_byte & 0x80) == 0 {
            return Err(TransportError::HardwareError);
        }

        let ts_high = (header & 0x3F) as u16;
        let ts_low = (ts_low_byte & 0x7F) as u16;
        let timestamp = (ts_high << 7) | ts_low;

        let mut offset = 2;
        while offset + 4 <= src.len() {
            let w0 = u32::from_le_bytes([
                src[offset],
                src[offset + 1],
                src[offset + 2],
                src[offset + 3],
            ]);
            let mt = (w0 >> 28) & 0xF;
            let wc = match mt {
                0x0 | 0x1 | 0x2 | 0x6 | 0x7 => 1,
                0x3 | 0x4 | 0x8 | 0x9 | 0xA => 2,
                0xB | 0xC => 3,
                0x5 | 0xD | 0xE | 0xF => 4,
                _ => 1,
            };

            if offset + (wc * 4) > src.len() {
                break;
            }

            let mut data = [0_u32; 4];
            data[0] = w0;
            for (idx, slot) in data.iter_mut().enumerate().take(wc).skip(1) {
                let word_off = offset + (idx * 4);
                *slot = u32::from_le_bytes([
                    src[word_off],
                    src[word_off + 1],
                    src[word_off + 2],
                    src[word_off + 3],
                ]);
            }

            on_ump(Ump { data });
            offset += wc * 4;
        }

        Ok(timestamp)
    }
}

/// Nordic nRF52 BLE MIDI 2.0 Driver
pub struct Nrf52BleMidiDriver<const RX_CAP: usize = 64, const TX_CAP: usize = 64> {
    rx_queue: Queue<Ump, RX_CAP>,
    tx_queue: Queue<Ump, TX_CAP>,
    current_timestamp: u16,
    connected: bool,
}

impl<const RX_CAP: usize, const TX_CAP: usize> Default for Nrf52BleMidiDriver<RX_CAP, TX_CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Nrf52BleMidiDriver<RX_CAP, TX_CAP> {
    pub fn new() -> Self {
        Self {
            rx_queue: Queue::new(),
            tx_queue: Queue::new(),
            current_timestamp: 0,
            connected: true,
        }
    }

    /// Sets connection status of the BLE peripheral.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Sets the current local 13-bit millisecond timestamp.
    pub fn set_timestamp(&mut self, timestamp_ms: u16) {
        self.current_timestamp = timestamp_ms & 0x1FFF;
    }

    /// Handles an incoming BLE GATT Characteristic write buffer.
    pub fn handle_gatt_rx(&mut self, packet: &[u8]) -> Result<u16, TransportError> {
        let rx = &mut self.rx_queue;
        BleMidiPacket::unpack_ble_ump_packet(packet, |ump| {
            let _ = rx.enqueue(ump);
        })
    }

    /// Formats pending TX UMPs into a BLE GATT Characteristic notification buffer.
    pub fn fill_gatt_tx(
        &mut self,
        packets: &[Ump],
        dest: &mut [u8],
    ) -> Result<usize, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        BleMidiPacket::pack_ble_ump_packet(self.current_timestamp, packets, dest)
            .ok_or(TransportError::TxBufferFull)
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> Transport for Nrf52BleMidiDriver<RX_CAP, TX_CAP> {
    fn send(&mut self, packets: &[Ump]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        for ump in packets {
            self.tx_queue
                .enqueue(*ump)
                .map_err(|_| TransportError::TxBufferFull)?;
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Ump>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        Ok(self.rx_queue.dequeue())
    }
}

impl<const RX_CAP: usize, const TX_CAP: usize> EmbeddedTransport
    for Nrf52BleMidiDriver<RX_CAP, TX_CAP>
{
    type Error = TransportError;

    fn read_ump(&mut self) -> Result<Option<Ump>, Self::Error> {
        self.recv()
    }

    fn write_ump(&mut self, ump: &Ump) -> Result<(), Self::Error> {
        self.send(core::slice::from_ref(ump))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
