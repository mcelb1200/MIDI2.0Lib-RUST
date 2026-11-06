//! `el_bsp_rp2040` - Board Support Package and Peripheral Drivers for Raspberry Pi RP2040
//!
//! Provides turnkey embedded MIDI 2.0 peripheral drivers for the Raspberry Pi RP2040 microcontroller:
//! - Direct Programmable I/O (PIO) state-machine UART driver for jitter-free 31,250 baud MIDI 1.0 and high-speed serial UMP.
//! - USB-MIDI 2.0 Native Device driver with Group Terminal Blocks (GTB) and endpoint packet framing.

#![no_std]

pub mod pio;
pub mod usb;

pub use pio::{PioBlock, PioClockDiv, PioStateMachine, Rp2040PioDriver};
pub use usb::Rp2040UsbMidiDriver;

#[cfg(test)]
mod tests {
    use super::*;
    use el_midi2::messages::UmpFactory;
    use el_midi2::transport::{EmbeddedTransport, Transport};

    #[test]
    fn test_rp2040_pio_clock_div_calculation() {
        let div_midi = PioClockDiv::from_baud(31_250);
        assert_eq!(div_midi.int_div, 500);
        assert_eq!(div_midi.frac_div, 0);

        let div_115k = PioClockDiv::from_baud(115_200);
        assert_eq!(div_115k.int_div, 135);
        assert_eq!(div_115k.frac_div, 162);

        let div_1m = PioClockDiv::from_baud(1_000_000);
        assert_eq!(div_1m.int_div, 15);
        assert_eq!(div_1m.frac_div, 160);
    }

    #[test]
    fn test_rp2040_pio_driver_loopback() {
        let mut pio_drv =
            Rp2040PioDriver::<64, 64>::new(PioBlock::Pio0, PioStateMachine::Sm0, 31_250, 0);

        // Queue MIDI 1.0 Note On (Ch0, Note 60, Vel 100) -> serial bytes in TX FIFO
        let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
        assert!(pio_drv.send(&[note_on]).is_ok());
        assert!(pio_drv.tx_pending() > 0);

        // Simulate loopback wire (TX -> RX)
        while let Some(b) = pio_drv.pop_tx_byte() {
            pio_drv.push_rx_byte(b).expect("push rx");
        }

        // Receive UMP from RX FIFO
        let received = pio_drv.recv().expect("recv ok").expect("ump packet");
        assert_eq!(received.data[0], note_on.data[0]);
    }

    #[test]
    fn test_rp2040_usb_driver_packets() {
        let mut usb = Rp2040UsbMidiDriver::<64, 16>::new(1, 0, 1);
        assert_eq!(usb.gtb.block_id, 1);

        let ump = UmpFactory::midi2_note_on(0, 0, 60, 0, 0x8000, 0);
        let mut out_pkt = [0_u8; 64];
        let len = usb
            .fill_usb_in_packet(&[ump], &mut out_pkt)
            .expect("fill in");
        assert_eq!(len, 8);

        // Feed to OUT endpoint handler
        let ingested = usb
            .handle_usb_out_packet(&out_pkt[..len])
            .expect("handle out");
        assert_eq!(ingested, 1);

        let rec = usb.read_ump().expect("read ok").expect("some ump");
        assert_eq!(rec.data[0], ump.data[0]);
        assert_eq!(rec.data[1], ump.data[1]);
    }
}
