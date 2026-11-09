//! `el_bsp_rpi3` - Board Support Package and Peripheral Drivers for Bare Metal Raspberry Pi 3B+
//!
//! Provides turnkey embedded MIDI 2.0 peripheral drivers for the Broadcom BCM2837B0 SoC (ARM Cortex-A53 quad-core, AArch64):
//! - 1 MHz System Timer driver for UMP Jitter Reduction (JR) timestamps and AppleMIDI 100-microsecond clock synchronization.
//! - VideoCore Mailbox 0 interface for clock locking (48 MHz UART reference) and hardware discovery.
//! - GPIO pin multiplexing for PL011 UART0 (ALT0 on GPIO 14/15) and MIDI activity indicators.
//! - PL011 UART0 driver with exact fractional baud generation (31,250 baud MIDI 1.0, 1M/3M baud high-speed UMP), running status tracking, and real-time interleaving.
//! - Synopsys DWC2 USB OTG driver with 13-byte USB-MIDI 2.0 Group Terminal Block (GTB) descriptors and Little-Endian packet framing.
//! - LAN7515 Ethernet UDP Network MIDI 2.0 (AppleMIDI / RTP-MIDI) transport adapter.

#![no_std]

pub mod gpio;
pub mod mailbox;
pub mod network;
pub mod timer;
pub mod uart;
pub mod usb;

pub use gpio::{GpioFunction, GpioPin, GpioPull, Rpi3Gpio};
pub use mailbox::{MailboxChannel, MailboxClockId, MailboxTag, Rpi3Mailbox};
pub use network::{Rpi3EthernetConfig, Rpi3EthernetMidiAdapter};
pub use timer::{Rpi3SystemTimer, SYSTEM_TIMER_FREQ_HZ};
pub use uart::{Rpi3BaudDivisor, Rpi3BaudRate, Rpi3UartDriver, UART_REF_CLOCK_HZ};
pub use usb::Rpi3UsbMidiDriver;

/// BCM2837B0 MMIO Base Address on Raspberry Pi 3B+ in AArch64 mode
pub const MMIO_BASE: usize = 0x3F00_0000;

/// BCM2837B0 Peripheral Physical Base Addresses
pub mod mmio {
    use super::MMIO_BASE;

    /// System Timer (1 MHz 64-bit hardware counter)
    pub const SYSTEM_TIMER_BASE: usize = MMIO_BASE + 0x0000_3000;

    /// VideoCore Mailbox 0 (ARM <- VC)
    pub const MAILBOX0_BASE: usize = MMIO_BASE + 0x0000_B880;

    /// ARM Interrupt Controller
    pub const ARM_IRQ_BASE: usize = MMIO_BASE + 0x0000_B200;

    /// GPIO Controller
    pub const GPIO_BASE: usize = MMIO_BASE + 0x0020_0000;

    /// PL011 UART0 (Primary high-precision MIDI transceiver)
    pub const UART0_BASE: usize = MMIO_BASE + 0x0020_1000;

    /// Mini UART (AUX / UART1)
    pub const MINI_UART_BASE: usize = MMIO_BASE + 0x0021_5000;

    /// Synopsys DWC2 USB OTG Controller
    pub const DWC2_USB_BASE: usize = MMIO_BASE + 0x0098_0000;
}

#[cfg(test)]
mod tests {
    use super::*;
    use el_midi2::messages::UmpFactory;
    use el_midi2::transport::network::SessionRole;
    use el_midi2::transport::{EmbeddedTransport, Transport};

    /// Test 1: Baud Divisor Precision (`test_rpi3_baud_divisors`)
    /// Assert integer (IBRD) and fractional (FBRD) registers evaluate to exact theoretical values
    /// for 31,250 baud (96, 0), 1 MBaud (3, 0), 3 MBaud (1, 0), and 115,200 baud (26, 3) under 48 MHz clock.
    #[test]
    fn test_rpi3_baud_divisors() {
        // Standard MIDI 1.0 (31,250 baud):
        // 48_000_000 / (16 * 31,250) = 96.0000 -> IBRD=96, FBRD=0
        let div_midi1 = Rpi3BaudDivisor::calculate(UART_REF_CLOCK_HZ, 31_250);
        assert_eq!(div_midi1.ibrd, 96);
        assert_eq!(div_midi1.fbrd, 0);

        let driver_midi1 = Rpi3UartDriver::<64, 64>::new(Rpi3BaudRate::Midi1_31250, 0);
        assert_eq!(driver_midi1.baud_divisor(), div_midi1);

        // High-Speed Serial UMP (1,000,000 baud / 1 MBaud):
        // 48_000_000 / (16 * 1_000_000) = 3.0000 -> IBRD=3, FBRD=0
        let div_1m = Rpi3BaudDivisor::calculate(UART_REF_CLOCK_HZ, 1_000_000);
        assert_eq!(div_1m.ibrd, 3);
        assert_eq!(div_1m.fbrd, 0);

        let driver_1m = Rpi3UartDriver::<64, 64>::new(Rpi3BaudRate::Ump1M, 0);
        assert_eq!(driver_1m.baud_divisor(), div_1m);

        // High-Speed Serial UMP (3,000,000 baud / 3 MBaud):
        // 48_000_000 / (16 * 3_000_000) = 1.0000 -> IBRD=1, FBRD=0
        let div_3m = Rpi3BaudDivisor::calculate(UART_REF_CLOCK_HZ, 3_000_000);
        assert_eq!(div_3m.ibrd, 1);
        assert_eq!(div_3m.fbrd, 0);

        let driver_3m = Rpi3UartDriver::<64, 64>::new(Rpi3BaudRate::Ump3M, 0);
        assert_eq!(driver_3m.baud_divisor(), div_3m);

        // Standard Diagnostic / Host Link (115,200 baud):
        // 48_000_000 / (16 * 115_200) = 26.041666 -> IBRD=26, FBRD=round(0.041666 * 64)=3
        let div_115k = Rpi3BaudDivisor::calculate(UART_REF_CLOCK_HZ, 115_200);
        assert_eq!(div_115k.ibrd, 26);
        assert_eq!(div_115k.fbrd, 3);

        let driver_115k = Rpi3UartDriver::<64, 64>::new(Rpi3BaudRate::Standard115200, 0);
        assert_eq!(driver_115k.baud_divisor(), div_115k);
    }

    /// Test 2: System Timer Timestamp Precision (`test_rpi3_system_timer_math`)
    /// Verify conversion of 1 MHz hardware counter ticks to 100-microsecond AppleMIDI network clock units
    /// and 16-bit Jitter Reduction timestamps.
    #[test]
    fn test_rpi3_system_timer_math() {
        // 1,000,000 ticks = 1 second = 10,000 AppleMIDI 100-microsecond units
        let ticks_1s: u64 = 1_000_000;
        let apple_ts = Rpi3SystemTimer::ticks_to_apple_midi_timestamp(ticks_1s);
        assert_eq!(apple_ts, 10_000);
        assert_eq!(
            Rpi3SystemTimer::apple_midi_timestamp_to_ticks(apple_ts),
            ticks_1s
        );

        // 350 microseconds = 3 AppleMIDI units (integer division truncated)
        assert_eq!(Rpi3SystemTimer::ticks_to_apple_midi_timestamp(350), 3);

        // JR Timestamp precision: 16-bit clock tick at 31.25 µs resolution (1/32000 sec).
        // For 31.25 µs (or 125 µs = 4 JR ticks):
        assert_eq!(Rpi3SystemTimer::ticks_to_jr_timestamp(125), 4);
        assert_eq!(Rpi3SystemTimer::ticks_to_jr_timestamp(1_000_000), 32_000);

        // 16-bit wrapping verification (e.g. 65,536 JR ticks = 2,048,000 µs wraps to 0)
        assert_eq!(Rpi3SystemTimer::ticks_to_jr_timestamp(2_048_000), 0);
    }

    /// Test 3: UART0 Loopback Simulation (`test_rpi3_uart_loopback`)
    /// Stream MIDI 1.0 Note On/Off bytes and high-speed UMP frames through the PL011 driver FIFO buffer,
    /// verifying zero-copy byte-to-UMP translation and running status reconstruction.
    #[test]
    fn test_rpi3_uart_loopback() {
        let mut uart = Rpi3UartDriver::<64, 64>::new(Rpi3BaudRate::Midi1_31250, 0);

        // 1. Send MIDI 1.0 Note On (Ch0, Note 60, Vel 100) -> converts to 3 serial bytes: 0x90, 0x3C, 0x64
        let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
        assert!(uart.send(&[note_on]).is_ok());
        assert_eq!(uart.tx_pending(), 3);

        // Send MIDI 1.0 Note Off (Ch0, Note 60, Vel 64) -> converts to 3 serial bytes: 0x80, 0x3C, 0x40
        let note_off = UmpFactory::midi1_note_off(0, 0, 60, 64);
        assert!(uart.send(&[note_off]).is_ok());
        assert_eq!(uart.tx_pending(), 6);

        // Loopback simulated physical wire: pop from TX FIFO into RX
        while let Some(byte) = uart.pop_tx_byte() {
            uart.push_rx_byte(byte);
        }
        assert_eq!(uart.tx_pending(), 0);

        // Ingested UMPs should match original Note On and Note Off
        let r1 = uart.recv().expect("recv ok").expect("note on ump");
        assert_eq!(r1.data[0], note_on.data[0]);

        let r2 = uart.recv().expect("recv ok").expect("note off ump");
        assert_eq!(r2.data[0], note_off.data[0]);

        // Running status verification: send status byte once, then multiple data byte pairs
        // 0x90 0x40 0x7F (Note On note 64 vel 127)
        // 0x41 0x7F (Running status Note On note 65 vel 127)
        uart.push_rx_byte(0x90);
        uart.push_rx_byte(0x40);
        uart.push_rx_byte(0x7F);

        uart.push_rx_byte(0x41);
        uart.push_rx_byte(0x7F);

        let rs1 = uart.read_ump().expect("read ok").expect("running status 1");
        let status_nibble_1 = ((rs1.data[0] >> 20) & 0x0F) as u8;
        let note_number_1 = ((rs1.data[0] >> 8) & 0xFF) as u8;
        assert_eq!(status_nibble_1, 0x9);
        assert_eq!(note_number_1, 0x40);

        let rs2 = uart.read_ump().expect("read ok").expect("running status 2");
        let status_nibble_2 = ((rs2.data[0] >> 20) & 0x0F) as u8;
        let note_number_2 = ((rs2.data[0] >> 8) & 0xFF) as u8;
        assert_eq!(status_nibble_2, 0x9);
        assert_eq!(note_number_2, 0x41);
    }

    /// Test 4: USB-MIDI 2.0 GTB Descriptor Serialization (`test_rpi3_usb_gtb_descriptors`)
    /// Verify byte-accurate USB-MIDI 2.0 Group Terminal Block descriptors and Little-Endian packet framing.
    #[test]
    fn test_rpi3_usb_gtb_descriptors() {
        let mut usb = Rpi3UsbMidiDriver::<64, 16>::new(1, 0, 16);
        assert_eq!(usb.gtb.block_id, 1);
        assert_eq!(usb.gtb.group_first, 0);
        assert_eq!(usb.gtb.group_count, 16);

        // Verify standard 13-byte GTB descriptor serialization
        let mut gtb_desc = [0_u8; 13];
        usb.gtb.serialize(&mut gtb_desc);
        assert_eq!(gtb_desc[0], 13); // bLength
        assert_eq!(gtb_desc[1], 0x24); // CS_INTERFACE
        assert_eq!(gtb_desc[2], 0x03); // MS_DESCRIPTOR_SUBTYPE = GROUP_TERMINAL_BLOCK
        assert_eq!(gtb_desc[3], 1); // bGroupTrmBlkID
        assert_eq!(gtb_desc[4], 0x00); // bBlockType (0=Bidirectional)
        assert_eq!(gtb_desc[5], 0); // bGroupFirst
        assert_eq!(gtb_desc[6], 16); // bGroupLength
        assert_eq!(gtb_desc[8], 0x02); // bMidiProtocol = MIDI 2.0

        // Test Little-Endian UMP packet framing over USB
        let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x9000, 0);
        let mut usb_pkt = [0_u8; 64];
        let written = usb
            .fill_usb_in_packet(&[ump], &mut usb_pkt)
            .expect("fill in packet");
        assert_eq!(written, 8); // 64-bit UMP = 8 bytes

        // Ingest into OUT endpoint
        let ingested = usb
            .handle_usb_out_packet(&usb_pkt[..written])
            .expect("handle out packet");
        assert_eq!(ingested, 1);

        let rec = usb.recv().expect("recv ok").expect("ump");
        assert_eq!(rec.data[0], ump.data[0]);
        assert_eq!(rec.data[1], ump.data[1]);
    }

    /// Test 5: LAN7515 Ethernet Network MIDI 2.0 Adapter & Mailbox Property Buffer
    #[test]
    fn test_rpi3_network_ethernet_adapter() {
        let config = Rpi3EthernetConfig {
            local_name: "RPi3 MIDI Synth",
            local_ssrc: 0xB827EB99,
            control_port: 5004,
            midi_port: 5005,
            mac_address: [0xB8, 0x27, 0xEB, 0x99, 0x88, 0x77],
        };

        let mut eth_adapter =
            Rpi3EthernetMidiAdapter::<16>::new(config, SessionRole::Responder, 0x11223344);

        // Test incoming UDP MIDI packet
        let clock = UmpFactory::timing_clock(0);
        let mut datagram = [0_u8; 32];
        let rtp_hdr = el_midi2::transport::network::RtpMidiHeader::new(1, 2000, 0x55667788);
        let len = el_midi2::transport::network::NetworkMidiPacket::pack_packet(
            &rtp_hdr,
            &[clock],
            &mut datagram,
        )
        .expect("pack");

        let ingested = eth_adapter
            .handle_midi_datagram(&datagram[..len])
            .expect("handle");
        assert_eq!(ingested, 1);

        let rec = eth_adapter.recv().expect("recv ok").expect("clock ump");
        assert_eq!(rec.data[0], clock.data[0]);

        // Test Mailbox buffer formatting
        let mut mb_buf = [0_u32; 12];
        let words = Rpi3Mailbox::build_set_clock_rate_buffer(
            &mut mb_buf,
            MailboxClockId::Uart,
            UART_REF_CLOCK_HZ,
            false,
        );
        assert_eq!(words, 12);
        assert_eq!(mb_buf[2], MailboxTag::TAG_SET_CLOCK_RATE);
        assert_eq!(mb_buf[5], MailboxClockId::Uart as u32);
        assert_eq!(mb_buf[6], UART_REF_CLOCK_HZ);

        // Test GPIO function select calculation
        let (offset, shift) = Rpi3Gpio::function_select_offset_and_shift(14);
        assert_eq!(offset, 4); // GPFSEL1
        assert_eq!(shift, 12); // pin 14: (14 % 10) * 3 = 4 * 3 = 12
    }
}
