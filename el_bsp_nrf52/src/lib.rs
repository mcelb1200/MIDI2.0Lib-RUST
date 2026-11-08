//! `el_bsp_nrf52` - Board Support Package and Peripheral Drivers for Nordic nRF52 Series
//!
//! Provides turnkey embedded MIDI 2.0 peripheral drivers for Nordic Semiconductor nRF52 microcontrollers (ARM Cortex-M4F):
//! - EasyDMA UARTE driver for 31,250 baud MIDI 1.0 (with running status and real-time interleaving) and high-speed serial UMP.
//! - USB-MIDI 2.0 Native Device driver for nRF52 USBD cores (nRF52840, nRF52833) with Group Terminal Blocks (GTB) and endpoint packet framing.
//! - Bluetooth Low Energy (BLE) MIDI 2.0 driver with GATT characteristic packet framing and 13-bit timestamp encapsulation.

#![no_std]

pub mod ble;
pub mod uart;
pub mod usb;

pub use ble::{BleMidiPacket, Nrf52BleMidiDriver};
pub use uart::{Nrf52BaudRate, Nrf52UartDriver, Nrf52UarteInstance};
pub use usb::Nrf52UsbMidiDriver;

/// Nordic nRF52 SoC variant detection
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Nrf52ChipVariant {
    /// nRF52832 (BLE, UARTE, No USB)
    Nrf52832,
    /// nRF52840 (BLE, UARTE, USBD Full-Speed)
    Nrf52840,
    /// nRF52833 (BLE, UARTE, USBD Full-Speed)
    Nrf52833,
    /// Generic or simulated host environment
    Generic,
}

impl Nrf52ChipVariant {
    /// Detects chip target variant based on compile-time cfg flags or defaults to Generic.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(feature = "nrf52840")]
        {
            Self::Nrf52840
        }
        #[cfg(all(feature = "nrf52833", not(feature = "nrf52840")))]
        {
            Self::Nrf52833
        }
        #[cfg(all(
            feature = "nrf52832",
            not(any(feature = "nrf52840", feature = "nrf52833"))
        ))]
        {
            Self::Nrf52832
        }
        #[cfg(not(any(feature = "nrf52840", feature = "nrf52833", feature = "nrf52832")))]
        {
            Self::Generic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use el_midi2::messages::UmpFactory;
    use el_midi2::transport::{EmbeddedTransport, Transport};

    #[test]
    fn test_nrf52_chip_variant() {
        let variant = Nrf52ChipVariant::current();
        assert_eq!(variant, Nrf52ChipVariant::Generic);
    }

    #[test]
    fn test_nrf52_baud_rate_constants_and_calculation() {
        assert_eq!(Nrf52BaudRate::Baud31250.register_value(), 0x0080_0000);
        assert_eq!(Nrf52BaudRate::Baud115200.register_value(), 0x01D7_E000);
        assert_eq!(Nrf52BaudRate::Baud230400.register_value(), 0x03AF_B000);
        assert_eq!(Nrf52BaudRate::Baud1M.register_value(), 0x1000_0000);

        let calc_31250 = Nrf52BaudRate::from_baud(31_250);
        assert_eq!(calc_31250.register_value(), 0x0080_0000);

        let calc_115200 = Nrf52BaudRate::from_baud(115_200);
        assert_eq!(calc_115200.register_value(), 0x01D7_E000);

        let calc_custom = Nrf52BaudRate::from_baud(250_000);
        assert_eq!(calc_custom.register_value(), 0x0400_0000);
    }

    #[test]
    fn test_nrf52_uart_driver_loopback() {
        let mut uart =
            Nrf52UartDriver::<64, 64>::new(Nrf52UarteInstance::Uarte0, Nrf52BaudRate::Baud31250, 0);

        // Send MIDI 1.0 Note On (Ch0, Note 60, Vel 100) -> converts to UART TX bytes
        let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
        assert!(uart.send(&[note_on]).is_ok());
        assert_eq!(uart.tx_pending(), 3);

        // Simulate EasyDMA TX buffer fill
        let mut dma_tx_buf = [0_u8; 16];
        let bytes_filled = uart.fill_easydma_tx(&mut dma_tx_buf);
        assert_eq!(bytes_filled, 3);
        assert_eq!(uart.tx_pending(), 0);

        // Loopback DMA bytes into EasyDMA RX handler
        uart.handle_easydma_rx(&dma_tx_buf[..bytes_filled]);

        // Receive UMP from driver
        let rec = uart.recv().expect("recv ok").expect("ump message");
        assert_eq!(rec.data[0], note_on.data[0]);
    }

    #[test]
    fn test_nrf52_usb_driver_packets() {
        let mut usb = Nrf52UsbMidiDriver::<64, 64>::new(1, 0, 1);
        assert_eq!(usb.gtb.block_id, 1);

        // Create a 64-bit MIDI 2.0 Note On UMP
        let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);
        let mut out_pkt = [0_u8; 64];
        let len = usb
            .fill_usb_in_packet(&[ump], &mut out_pkt)
            .expect("fill in packet");
        assert_eq!(len, 8);

        // Ingest into OUT endpoint
        let ingested = usb
            .handle_usb_out_packet(&out_pkt[..len])
            .expect("handle out packet");
        assert_eq!(ingested, 2); // 2 words (64-bit)

        let rec = usb.read_ump().expect("read ok").expect("ump message");
        assert_eq!(rec.data[0], ump.data[0]);
        assert_eq!(rec.data[1], ump.data[1]);
    }

    #[test]
    fn test_nrf52_ble_midi_driver_roundtrip() {
        let mut ble = Nrf52BleMidiDriver::<64, 64>::new();
        ble.set_timestamp(1234); // 13-bit timestamp: 1234 ms

        let note_on = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);
        let mut ble_packet = [0_u8; 64];
        let len = ble
            .fill_gatt_tx(&[note_on], &mut ble_packet)
            .expect("fill gatt tx");
        assert_eq!(len, 10); // 2 bytes header/timestamp + 8 bytes (2 words UMP)

        // Verify header and timestamp low byte
        assert_eq!(ble_packet[0] & 0x80, 0x80);
        assert_eq!(ble_packet[1] & 0x80, 0x80);

        // Ingest into RX driver
        let mut ble_rx = Nrf52BleMidiDriver::<64, 64>::new();
        let ts_parsed = ble_rx
            .handle_gatt_rx(&ble_packet[..len])
            .expect("handle gatt rx");
        assert_eq!(ts_parsed, 1234);

        let rec = ble_rx.read_ump().expect("read ok").expect("ump packet");
        assert_eq!(rec.data[0], note_on.data[0]);
        assert_eq!(rec.data[1], note_on.data[1]);
    }
}
