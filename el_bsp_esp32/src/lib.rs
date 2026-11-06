//! `el_bsp_esp32` - Board Support Package and Peripheral Drivers for ESP32 (Xtensa & RISC-V)
//!
//! Provides turnkey embedded MIDI 2.0 peripheral drivers for the Espressif ESP32 SoC family:
//! - Hardware UART FIFO driver supporting MIDI 1.0 (31,250 baud) and high-speed serial UMP.
//! - USB-OTG USB-MIDI 2.0 Peripheral driver with Group Terminal Block (GTB) descriptors.
//! - WiFi / UDP Network MIDI 2.0 (RTP-MIDI / AppleMIDI) transport adapter.
//!
//! Supports both:
//! - **Xtensa Architecture**: ESP32, ESP32-S2, ESP32-S3
//! - **RISC-V Architecture**: ESP32-C2, ESP32-C3, ESP32-C6, ESP32-H2

#![no_std]

pub mod uart;
pub mod usb;
pub mod wifi;

pub use uart::{Esp32UartDriver, Esp32UartPort, MidiBaudRate};
pub use usb::Esp32UsbMidiDriver;
pub use wifi::{Esp32NetworkConfig, Esp32WifiMidiAdapter};

/// Chip architecture target family indicator.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Esp32ArchFamily {
    /// Tensilica Xtensa dual-core / single-core (ESP32, ESP32-S2, ESP32-S3)
    Xtensa,
    /// 32-bit RISC-V core (ESP32-C3, ESP32-C6, ESP32-H2)
    RiscV,
    /// Generic or simulated host environment
    Generic,
}

impl Esp32ArchFamily {
    /// Detects the active microcontroller architecture at compile-time.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_arch = "xtensa")]
        {
            Self::Xtensa
        }
        #[cfg(target_arch = "riscv32")]
        {
            Self::RiscV
        }
        #[cfg(not(any(target_arch = "xtensa", target_arch = "riscv32")))]
        {
            Self::Generic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use el_midi2::messages::UmpFactory;
    use el_midi2::transport::network::SessionRole;
    use el_midi2::transport::{EmbeddedTransport, Transport};

    #[test]
    fn test_esp32_arch_detection() {
        let arch = Esp32ArchFamily::current();
        // On x86 host test runners, this falls back safely to Generic
        assert_eq!(arch, Esp32ArchFamily::Generic);
    }

    #[test]
    fn test_esp32_uart_driver_loopback() {
        let mut uart =
            Esp32UartDriver::<64, 64>::new(Esp32UartPort::Uart1, MidiBaudRate::Midi1_31250, 0);

        // Send a MIDI 1.0 Note On (Ch0, Note 60, Vel 100) -> converts to serial bytes in TX FIFO
        let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
        assert!(uart.send(&[note_on]).is_ok());
        assert!(uart.tx_pending() > 0);

        // Simulate loopback wire (TX -> RX)
        while let Some(byte) = uart.pop_tx_byte() {
            uart.push_rx_byte(byte).expect("push rx");
        }

        // Receive UMP from RX FIFO
        let received = uart.recv().expect("recv ok").expect("ump packet");
        assert_eq!(received.data[0], note_on.data[0]);
    }

    #[test]
    fn test_esp32_usb_driver_packets() {
        let mut usb = Esp32UsbMidiDriver::<64, 16>::new(1, 0, 1);
        assert_eq!(usb.gtb.block_id, 1);

        let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);
        let mut out_pkt = [0_u8; 64];
        let len = usb.fill_usb_in_packet(&[ump], &mut out_pkt).expect("fill in");
        assert_eq!(len, 8);

        // Feed to OUT endpoint packet handler
        let ingested = usb.handle_usb_out_packet(&out_pkt[..len]).expect("handle out");
        assert_eq!(ingested, 1);

        let rec = usb.read_ump().expect("read ok").expect("some ump");
        assert_eq!(rec.data[0], ump.data[0]);
        assert_eq!(rec.data[1], ump.data[1]);
    }

    #[test]
    fn test_esp32_wifi_adapter_roundtrip() {
        let config = Esp32NetworkConfig {
            local_name: "ESP32-S3 Synthesizer",
            local_ssrc: 0x99887766,
            control_port: 5004,
            midi_port: 5005,
        };

        let mut wifi_adapter = Esp32WifiMidiAdapter::<16>::new(config, SessionRole::Responder, 0);

        // Simulate incoming UDP MIDI datagram
        let ump = UmpFactory::timing_clock(0);
        let mut datagram = [0_u8; 32];
        let rtp_hdr = el_midi2::transport::network::RtpMidiHeader::new(1, 1000, 0x11223344);
        let datagram_len = el_midi2::transport::network::NetworkMidiPacket::pack_packet(
            &rtp_hdr,
            &[ump],
            &mut datagram,
        )
        .expect("pack");

        let count = wifi_adapter
            .handle_midi_datagram(&datagram[..datagram_len])
            .expect("handle");
        assert_eq!(count, 1);

        let rec = wifi_adapter.recv().expect("recv ok").expect("packet");
        assert_eq!(rec.data[0], ump.data[0]);
    }
}
