//! Display check for the Cardputer-Adv: the firmware-core screens on the ST7789V2 panel, one per press of BOOT (G0).
//!
//! Pins (Stamp-S3A, from docs/references/cardputer-adv/extracts.md): backlight G38, reset G33, D/C G34, SDA/MOSI G35,
//! SCK G36, CS G37. BOOT is G0, active low. The first screen is a calibration pattern: a one-pixel frame at the
//! panel's edges, so any cut-off edge is visible. The rest are the compact screens of firmware-core.
//! Provisional: the panel offsets come from `firmware-core/src/board.rs` and are what this check is for.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::{Instant, Rate};
use vibebuddy_firmware_core::canvas::{Canvas, FRAME_BYTES, HEIGHT, WIDTH};
use vibebuddy_firmware_core::display::{Display, Mode, PanelKind, Scene, Screen, State, TaskInput};
use vibebuddy_firmware_core::display::{Panel, Tone};
use vibebuddy_firmware_core::lcd::{INIT_SEQUENCE, RAMWR, WINDOW};
use vibebuddy_firmware_core::leisure::{Leisure, Skit};
use vibebuddy_firmware_core::pomodoro::{Pomodoro, Tally};

esp_bootloader_esp_idf::esp_app_desc!();

/// The panel is written in chunks, so the SPI driver never sees one very long buffer.
const CHUNK: usize = 4092;

struct Lcd {
    spi: Spi<'static, esp_hal::Blocking>,
    dc: Output<'static>,
    cs: Output<'static>,
    backlight: Output<'static>,
    frame: Vec<u8>,
}

impl Lcd {
    fn command(&mut self, command: u8, parameters: &[u8]) {
        self.cs.set_low();
        self.dc.set_low();
        let _ = self.spi.write(&[command]);
        if !parameters.is_empty() {
            self.dc.set_high();
            let _ = self.spi.write(parameters);
        }
        self.cs.set_high();
    }

    fn init(&mut self) {
        let delay = Delay::new();
        for step in INIT_SEQUENCE {
            self.command(step.command, step.parameters);
            if step.delay_ms > 0 {
                delay.delay_millis(step.delay_ms);
            }
        }
    }

    /// Sends the whole frame: the window, then RAMWR, then the pixels in wire order (see canvas.rs).
    fn flush(&mut self) {
        for step in WINDOW.iter() {
            self.command(step.command, step.parameters);
        }
        self.cs.set_low();
        self.dc.set_low();
        let _ = self.spi.write(&[RAMWR]);
        self.dc.set_high();
        for chunk in self.frame.chunks(CHUNK) {
            let _ = self.spi.write(chunk);
        }
        self.cs.set_high();
    }
}

impl Screen for Lcd {
    fn frame(&mut self) -> &mut [u8] {
        &mut self.frame
    }

    fn present(&mut self) -> Result<(), ()> {
        self.flush();
        Ok(())
    }

    fn set_backlight(&mut self, on: bool) -> Result<(), ()> {
        self.backlight.set_level(if on { Level::High } else { Level::Low });
        Ok(())
    }
}

fn now_ms() -> u32 {
    Instant::now().duration_since_epoch().as_millis() as u32
}

/// The calibration screen: black, a white frame on the outermost pixel of the panel, two diagonals, a centre cross,
/// and colour bars on the top row. Any edge the panel cuts off shows as a missing white line.
fn draw_calibration(frame: &mut [u8]) {
    let mut canvas = Canvas::new(frame);
    canvas.fill_rect(0, 0, WIDTH, HEIGHT, 0x0000);
    canvas.fill_rect(0, 0, WIDTH, 1, 0xffff);
    canvas.fill_rect(0, HEIGHT - 1, WIDTH, 1, 0xffff);
    canvas.fill_rect(0, 0, 1, HEIGHT, 0xffff);
    canvas.fill_rect(WIDTH - 1, 0, 1, HEIGHT, 0xffff);
    canvas.draw_line(0, 0, WIDTH - 1, HEIGHT - 1, 0x07e0);
    canvas.draw_line(WIDTH - 1, 0, 0, HEIGHT - 1, 0x07e0);
    canvas.fill_rect(WIDTH / 2, 0, 1, HEIGHT, 0xf800);
    canvas.fill_rect(0, HEIGHT / 2, WIDTH, 1, 0xf800);
    let bars = [0xf800u16, 0x07e0, 0x001f, 0xffe0, 0xf81f, 0x07ff];
    let width = WIDTH / bars.len() as i32;
    for (index, color) in bars.iter().enumerate() {
        canvas.fill_rect(index as i32 * width + 2, 3, width - 4, 6, *color);
    }
    canvas.draw_text(40, 60, b"CALIBRATION", 2, 0xffff, 16);
    canvas.draw_text(40, 84, b"240 X 135", 2, 0xffe0, 16);
}

fn menu_list() -> Panel {
    let line = |label: &[u8], value: &[u8], tone| (label.to_vec(), value.to_vec(), tone);
    Panel {
        kind: PanelKind::List,
        title: b"MENU".to_vec(),
        lines: alloc::vec![
            line(b"STOP FOCUS", b"", Tone::Plain),
            line(b"VOLUME", b"65", Tone::Accent),
            line(b"MUTE", b"OFF", Tone::Accent),
            line(b"COMPUTERS", b"1", Tone::Dim),
            line(b"NETWORK", b"OPEN", Tone::Dim),
            line(b"STATUS", b"OPEN", Tone::Dim),
        ],
        selected: Some(1),
        hints: alloc::vec![(b"K1", b"NEXT"), (b"K0", b"CHANGE"), (b"K2", b"CLOSE")],
    }
}

fn menu_network() -> Panel {
    let line = |label: &[u8], value: &[u8], tone| (label.to_vec(), value.to_vec(), tone);
    Panel {
        kind: PanelKind::Facts,
        title: b"NETWORK".to_vec(),
        lines: alloc::vec![
            line(b"LINK", b"USB", Tone::Plain),
            line(b"WIFI", b"OFF", Tone::Dim),
            line(b"RELAY", b"OFF", Tone::Dim),
        ],
        selected: None,
        hints: alloc::vec![(b"K0", b"BACK"), (b"K2", b"CLOSE")],
    }
}

fn menu_status() -> Panel {
    let line = |label: &[u8], value: &[u8], tone| (label.to_vec(), value.to_vec(), tone);
    Panel {
        kind: PanelKind::Facts,
        title: b"STATUS".to_vec(),
        lines: alloc::vec![
            line(b"FIRMWARE\n10-16 21:00", b"0.1.0", Tone::Plain),
            line(b"LINK", b"USB", Tone::Plain),
            line(b"VOICE", b"BUILTIN", Tone::Plain),
            line(b"TODAY", b"3 FOCUS 1H15", Tone::Plain),
        ],
        selected: None,
        hints: alloc::vec![(b"K0", b"BACK"), (b"K2", b"CLOSE")],
    }
}

/// The scenes in the order BOOT steps through them. Scene 0 is the calibration screen, not a firmware-core one.
const SCENES: usize = 16;

struct Demo {
    lcd: Lcd,
    display: Display,
    pomodoro: Pomodoro,
    leisure: Leisure,
    scene: usize,
}

impl Demo {
    /// Sets up scene `index`. Calibration is drawn straight into the frame; the rest go through the display.
    fn apply(&mut self, index: usize) {
        let now = now_ms();
        let Demo { lcd, display, pomodoro, leisure, .. } = self;
        // A scene built from the fields, so the display and the screen can be borrowed mutably next to it.
        macro_rules! scene {
            () => {
                Scene { now_ms: now, pomodoro: &*pomodoro, leisure: &*leisure }
            };
        }
        let three = [
            TaskInput { title: Some(b"CC:UPDATE README"), state: State::Working, elapsed_s: 20, project: Some(b"VIBEBUDDY") },
            TaskInput { title: Some(b"CX:BUILD FIRMWARE"), state: State::Working, elapsed_s: 60, project: Some(b"VIBEBUDDY") },
            TaskInput { title: Some(b"CI:APP TESTS"), state: State::Working, elapsed_s: 120, project: Some(b"VIBEBUDDY") },
        ];
        let two = [
            TaskInput { title: Some(b"CC:POMODORO TIMER FEATURE"), state: State::Working, elapsed_s: 75, project: Some(b"AGENT-BEACON") },
            TaskInput { title: Some(b"CX:EROS-TRAINING-INFRA"), state: State::InputRequired, elapsed_s: 900, project: Some(b"EROS") },
        ];
        match index {
            0 => {
                draw_calibration(&mut lcd.frame);
                let _ = lcd.present();
                return;
            }
            1 => {
                display.set_panel(lcd, &scene!(), None);
                display.set_mode(lcd, &scene!(), Mode::Duty);
                display.set_stats(&[b"7 DONE", b"4 ASKS", b"1H23 BUSY"]);
                let _ = display.show_tasks(lcd, &scene!(), State::Idle, Some(b"CC:AGENT-BEACON"), &[]);
            }
            2 => {
                let _ = display.show_tasks(lcd, &scene!(), State::Working, None, &three);
            }
            3 => {
                let _ = display.show_tasks(lcd, &scene!(), State::InputRequired, Some(b"CC:AGENT-BEACON"), &two);
            }
            4 => {
                pomodoro.restore_tally(Tally { day: 20260915, completed: 3, focus_s: 4500 });
                display.set_mode(lcd, &scene!(), Mode::Pomodoro);
            }
            5 => {
                pomodoro.toggle(now);
                display.set_mode(lcd, &scene!(), Mode::Pomodoro);
            }
            6 => {
                pomodoro.toggle(now);
                display.set_mode(lcd, &scene!(), Mode::Pomodoro);
            }
            7 => {
                // A focus that started 25 minutes ago and has just ended: tick() ends it now, so the alarm runs
                // without waiting. Starting the timer any later would cancel the alarm, which is by design.
                *pomodoro = Pomodoro::new();
                pomodoro.toggle(now.wrapping_sub(25 * 60 * 1000 + 100));
                let _ = pomodoro.tick(now);
                display.set_mode(lcd, &scene!(), Mode::Pomodoro);
                display.pomodoro_ended(&scene!());
            }
            8 => display.set_panel(lcd, &scene!(), Some(menu_list())),
            9 => display.set_panel(lcd, &scene!(), Some(menu_network())),
            10 => display.set_panel(lcd, &scene!(), Some(menu_status())),
            11 => {
                display.set_panel(lcd, &scene!(), None);
                display.set_mode(lcd, &scene!(), Mode::Duty);
                display.set_link_lost(lcd, &scene!(), true);
            }
            12 => {
                display.set_link_lost(lcd, &scene!(), false);
                display.set_muted(lcd, &scene!(), true);
            }
            13 => {
                display.set_muted(lcd, &scene!(), false);
                *leisure = Leisure::new(1, now);
                leisure.start_skit(Skit::Ball, now);
                display.set_mode(lcd, &scene!(), Mode::Leisure);
            }
            14 => {
                *leisure = Leisure::new(1, now);
                leisure.start_skit(Skit::Read, now);
                display.set_mode(lcd, &scene!(), Mode::Leisure);
            }
            _ => {
                *leisure = Leisure::new(1, now);
                leisure.start_skit(Skit::Sleep, now);
                display.set_mode(lcd, &scene!(), Mode::Leisure);
            }
        }
    }
}

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 120 * 1024);

    let backlight = Output::new(peripherals.GPIO38, Level::High, OutputConfig::default());
    let mut reset = Output::new(peripherals.GPIO33, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO34, Level::Low, OutputConfig::default());
    let cs = Output::new(peripherals.GPIO37, Level::High, OutputConfig::default());
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default().with_frequency(Rate::from_mhz(40)).with_mode(SpiMode::_0),
    )
    .expect("SPI config")
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO35);
    let boot = Input::new(peripherals.GPIO0, InputConfig::default().with_pull(Pull::Up));

    let mut lcd = Lcd { spi, dc, cs, backlight, frame: vec![0u8; FRAME_BYTES] };
    let delay = Delay::new();
    reset.set_low();
    delay.delay_millis(20);
    reset.set_high();
    delay.delay_millis(120);
    lcd.init();

    let mut demo = Demo {
        lcd,
        display: Display::new(),
        pomodoro: Pomodoro::new(),
        leisure: Leisure::new(1, 0),
        scene: 0,
    };
    // Display ignores every draw until start() has run: it marks the screen ready and turns the backlight on.
    let now = now_ms();
    let _ = demo.display.start(&mut demo.lcd, &Scene { now_ms: now, pomodoro: &demo.pomodoro, leisure: &demo.leisure });
    demo.apply(0);

    let mut was_pressed = false;
    let mut pressed_since = 0u32;
    loop {
        let now = now_ms();
        let pressed = boot.is_low();
        if pressed && !was_pressed {
            pressed_since = now;
        }
        if !pressed && was_pressed && now.wrapping_sub(pressed_since) >= 30 {
            demo.scene = (demo.scene + 1) % SCENES;
            demo.apply(demo.scene);
        }
        was_pressed = pressed;
        if demo.scene != 0 {
            let scene = Scene { now_ms: now, pomodoro: &demo.pomodoro, leisure: &demo.leisure };
            demo.display.tick(&mut demo.lcd, &scene);
        }
        delay.delay_millis(20);
    }
}
