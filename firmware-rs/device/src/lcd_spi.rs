//! ST7789V2 on the Cardputer-Adv: SPI, panel 240x135. The command sequence and the window come from core's `lcd`
//! module, as on the box; this only sends them over SPI. The frame is a plain buffer on the heap, sent in chunks:
//! at 40 MHz a full 64.8 KB frame takes about 13 ms, so no DMA is needed here.

use alloc::vec;
use alloc::vec::Vec;

use esp_hal::Blocking;
use esp_hal::delay::Delay;
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use vibebuddy_firmware_core::canvas::FRAME_BYTES;
use vibebuddy_firmware_core::lcd::{INIT_SEQUENCE, RAMWR, WINDOW};

/// Bytes per SPI write, so the driver never sees one very long buffer.
const CHUNK: usize = 4092;

pub struct Lcd {
    spi: Spi<'static, Blocking>,
    /// Data/command: low for a command byte, high for its parameters and the pixels.
    dc: Output<'static>,
    cs: Output<'static>,
    /// Reset is active low. The panel has the pin wired, so the reset is hardware, as in the Adv's factory firmware.
    reset: Output<'static>,
    frame: Vec<u8>,
}

impl Lcd {
    pub fn new(spi: Spi<'static, Blocking>, dc: Output<'static>, cs: Output<'static>, reset: Output<'static>) -> Self {
        Self { spi, dc, cs, reset, frame: vec![0u8; FRAME_BYTES] }
    }

    /// One command byte, then its parameters if there are any.
    fn send(&mut self, command: u8, parameters: &[u8]) -> Result<(), ()> {
        self.cs.set_low();
        self.dc.set_low();
        let result = self.spi.write(&[command]);
        if result.is_ok() && !parameters.is_empty() {
            self.dc.set_high();
            let result = self.spi.write(parameters);
            self.cs.set_high();
            return result.map_err(|_| ());
        }
        self.cs.set_high();
        result.map_err(|_| ())
    }

    /// Hardware reset, then the init sequence the C firmware sends, with its waits.
    pub fn init(&mut self) -> Result<(), ()> {
        let delay = Delay::new();
        self.reset.set_low();
        delay.delay_millis(20);
        self.reset.set_high();
        delay.delay_millis(120);
        for step in INIT_SEQUENCE {
            self.send(step.command, step.parameters)?;
            if step.delay_ms > 0 {
                delay.delay_millis(step.delay_ms);
            }
        }
        Ok(())
    }

    pub fn frame(&mut self) -> &mut [u8] {
        &mut self.frame[..FRAME_BYTES]
    }

    /// Full-screen refresh: the window, then RAMWR with the pixels in wire order (see core's canvas.rs).
    pub fn present(&mut self) -> Result<(), ()> {
        for step in &WINDOW {
            self.send(step.command, step.parameters)?;
        }
        self.send(RAMWR, &[])?;
        self.cs.set_low();
        self.dc.set_high();
        let mut result = Ok(());
        for chunk in self.frame[..FRAME_BYTES].chunks(CHUNK) {
            if self.spi.write(chunk).is_err() {
                result = Err(());
                break;
            }
        }
        self.cs.set_high();
        result
    }
}
