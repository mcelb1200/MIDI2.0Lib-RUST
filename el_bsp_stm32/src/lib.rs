//! `el_bsp_stm32` - Board Support Package and Peripheral Drivers for STM32 Microcontrollers
//!
//! Provides turnkey embedded MIDI 2.0 peripheral drivers for STM32 microcontrollers (ARM Cortex-M):
//! - DMA-backed circular buffer UART driver for 31,250 baud MIDI 1.0 (with running status and real-time interleaving) and high-speed serial UMP.
//! - USB-MIDI 2.0 Native Device driver for STM32 USB OTG_FS / OTG_HS cores with Group Terminal Blocks (GTB) and endpoint packet framing.

#![no_std]

pub mod uart;
pub mod usb;

pub use uart::{Stm32DmaCircularBuffer, Stm32UartBaudConfig, Stm32UartDriver, Stm32UartPeripheral};
pub use usb::{Stm32UsbCore, Stm32UsbMidiDriver};

#[cfg(test)]
mod tests {
    use super::*;
    use el_midi2::messages::UmpFactory;
    use el_midi2::transport::{EmbeddedTransport, Transport};

    #[test]
    fn test_stm32_uart_baud_rate_calculations() {
        // Standard STM32 APB1 peripheral bus clock: 42 MHz (e.g. STM32F407)
        // 42,000,000 / (16 * 31,250) = 84.0 => mantissa 84, fraction 0
        let baud_42m = Stm32UartBaudConfig::from_clock_and_baud(42_000_000, 31_250);
        assert_eq!(baud_42m.mantissa, 84);
        assert_eq!(baud_42m.fraction, 0);
        assert_eq!(baud_42m.brr_val, (84 << 4));

        // 16 MHz internal HSI clock:
        // 16,000,000 / (16 * 31,250) = 32.0 => mantissa 32, fraction 0
        let baud_16m = Stm32UartBaudConfig::from_clock_and_baud(16_000_000, 31_250);
        assert_eq!(baud_16m.mantissa, 32);
        assert_eq!(baud_16m.fraction, 0);
        assert_eq!(baud_16m.brr_val, (32 << 4));

        // 84 MHz peripheral clock:
        // 84,000,000 / (16 * 31,250) = 168.0 => mantissa 168, fraction 0
        let baud_84m = Stm32UartBaudConfig::from_clock_and_baud(84_000_000, 31_250);
        assert_eq!(baud_84m.mantissa, 168);
        assert_eq!(baud_84m.fraction, 0);

        // Standard 115,200 baud on 42 MHz:
        // 42,000,000 / (16 * 115,200) = 22.7864 => mantissa 22, fraction round(0.7864 * 16) = 13
        let baud_115k = Stm32UartBaudConfig::from_clock_and_baud(42_000_000, 115_200);
        assert_eq!(baud_115k.mantissa, 22);
        assert_eq!(baud_115k.fraction, 13);
    }

    #[test]
    fn test_stm32_uart_dma_circular_buffer() {
        let mut dma_buf = Stm32DmaCircularBuffer::<16>::new();

        // Simulate DMA writing 5 bytes
        for i in 0..5 {
            dma_buf.write_dma_byte(i, 0xA0 + i as u8);
        }

        let mut read_bytes = heapless::Vec::<u8, 16>::new();
        let count = dma_buf.read_available(5, |b| {
            let _ = read_bytes.push(b);
        });

        assert_eq!(count, 5);
        assert_eq!(read_bytes.as_slice(), &[0xA0, 0xA1, 0xA2, 0xA3, 0xA4]);

        // Wrap-around write: write 14 more bytes
        for i in 5..19 {
            dma_buf.write_dma_byte(i, i as u8);
        }

        let mut read_wrapped = heapless::Vec::<u8, 16>::new();
        let count_wrap = dma_buf.read_available(19 % 16, |b| {
            let _ = read_wrapped.push(b);
        });

        assert_eq!(count_wrap, 14);
        assert_eq!(read_wrapped[0], 5);
        assert_eq!(read_wrapped[13], 18);
    }

    #[test]
    fn test_stm32_uart_driver_loopback() {
        let mut uart =
            Stm32UartDriver::<64, 64>::new(Stm32UartPeripheral::Usart1, 42_000_000, 31_250, 0);

        // Send MIDI 1.0 Note On (Ch0, Note 60, Vel 100) -> converts to UART TX bytes
        let note_on = UmpFactory::midi1_note_on(0, 0, 60, 100);
        assert!(uart.send(&[note_on]).is_ok());
        assert_eq!(uart.tx_pending(), 3);

        // Loopback TX bytes into RX DMA buffer
        let mut head = 0;
        while let Some(b) = uart.pop_tx_byte() {
            uart.dma_buffer_mut().write_dma_byte(head, b);
            head += 1;
        }

        // Poll DMA RX
        uart.poll_dma_rx(head);

        // Receive UMP from driver
        let rec = uart.recv().expect("recv ok").expect("ump message");
        assert_eq!(rec.data[0], note_on.data[0]);
    }

    #[test]
    fn test_stm32_usb_driver_packets() {
        let mut usb = Stm32UsbMidiDriver::<64, 64>::new(Stm32UsbCore::OtgFs, 1, 0, 1);
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
}
