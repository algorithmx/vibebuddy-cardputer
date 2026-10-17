//! Device screenshots: sends `device.screenshot`, receives the run-length-encoded framebuffer (`SHOT BEGIN` /
//! a number of `SHOT` lines / `SHOT END`) and turns it into a PNG. Same format as tools/device-screenshot.py.

use std::sync::Arc;
use std::time::Duration;

use vibebuddy_protocol::Event;
use tokio::sync::broadcast;

use crate::serial_transport::{DeviceMessage, Transport};

/// Bound allocations to the largest supported screen, while taking each frame's dimensions from its header.
const MAX_PIXELS: usize = 320 * 240;
/// How long a frame may take in all. Over the bridge one takes over ten seconds, so the cap is generous.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the box may stay silent. A slow frame keeps sending rows and is worth waiting for; a box that sends
/// nothing at all isn't going to, and over the native port it would have started within a second. Giving up on a
/// frame that is still arriving was worse than waiting: the next request then landed mid-dump and was lost too.
const NATIVE_SILENCE: Duration = Duration::from_secs(10);
const BRIDGE_SILENCE: Duration = Duration::from_secs(30);

pub fn silence_timeout(bridge: bool) -> Duration {
    if bridge { BRIDGE_SILENCE } else { NATIVE_SILENCE }
}

pub struct Frame {
    pub width: usize,
    pub height: usize,
    /// RGB888, row by row.
    pub pixels: Vec<u8>,
    pub backlight_on: bool,
}

fn rgb565_to_rgb(color: u16) -> [u8; 3] {
    let red = ((color >> 11) & 0x1F) as u32;
    let green = ((color >> 5) & 0x3F) as u32;
    let blue = (color & 0x1F) as u32;
    [
        (red * 255 / 31) as u8,
        (green * 255 / 63) as u8,
        (blue * 255 / 31) as u8,
    ]
}

/// Expands the `rgb565:length` runs of a `SHOT` line into the pixel buffer.
pub fn decode_runs(line: &str, pixels: &mut Vec<u8>, max_pixels: usize) -> Result<(), String> {
    for run in line.split_whitespace() {
        let (color, count) = run
            .split_once(':')
            .ok_or_else(|| format!("bad run: {run}"))?;
        let color = u16::from_str_radix(color, 16).map_err(|_| format!("bad color: {color}"))?;
        let count: usize = count.parse().map_err(|_| format!("bad length: {count}"))?;
        // Check before expansion: a corrupt run must not allocate beyond the announced screen size.
        if count > max_pixels.saturating_sub(pixels.len() / 3) {
            return Err("more pixels than one screen".to_owned());
        }
        let rgb = rgb565_to_rgb(color);
        for _ in 0..count {
            pixels.extend_from_slice(&rgb);
        }
    }
    Ok(())
}

fn dimensions(header: &str) -> Result<(usize, usize), String> {
    let invalid = || format!("invalid screenshot dimensions: {}", header.trim());
    let (width, height) = header.split_whitespace().next().and_then(|size| size.split_once('x')).ok_or_else(invalid)?;
    let width: usize = width.parse().map_err(|_| invalid())?;
    let height: usize = height.parse().map_err(|_| invalid())?;
    if width == 0 || height == 0 || width.checked_mul(height).is_none_or(|pixels| pixels > MAX_PIXELS) {
        return Err(invalid());
    }
    Ok((width, height))
}

pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, frame.width as u32, frame.height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        let pixels = if frame.backlight_on {
            frame.pixels.clone()
        } else {
            // With the backlight off the picture can't be seen anyway; dim it into a "lights out" image rather than pretend it's lit.
            frame.pixels.iter().map(|value| value / 4).collect()
        };
        writer.write_image_data(&pixels).map_err(|error| error.to_string())?;
    }
    Ok(output)
}

/// Grabs one frame. The caller must already be subscribed to device messages so the first line isn't missed.
pub async fn capture(
    transport: Arc<dyn Transport>,
    mut bus: broadcast::Receiver<DeviceMessage>,
    silence: Duration,
) -> Result<Frame, String> {
    let frame = Event::named("device.screenshot")
        .to_ndjson()
        .map_err(|error| error.to_string())?;
    transport.send(frame).map_err(|error| format!("{error:?}"))?;

    let cap = tokio::time::Instant::now() + TOTAL_TIMEOUT;
    let mut deadline = (tokio::time::Instant::now() + silence).min(cap);
    let mut pixels = Vec::new();
    let mut size = None;
    let mut backlight_on = true;
    loop {
        let message = match tokio::time::timeout_at(deadline, bus.recv()).await {
            Ok(Ok(message)) => message,
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => return Err("device messages backed up, screenshot rows lost".to_owned()),
            Ok(Err(broadcast::error::RecvError::Closed)) => return Err("device message channel closed".to_owned()),
            Err(_) => {
                return Err(match size {
                    Some((width, height)) => {
                        let rows = pixels.len() / 3 / width;
                        format!("timed out waiting for the screenshot ({rows} of {height} rows arrived)")
                    }
                    None => "timed out waiting for the screenshot header".to_owned(),
                });
            }
        };
        let line = match message {
            DeviceMessage::Line(line) if line.starts_with("SHOT") => {
                deadline = (tokio::time::Instant::now() + silence).min(cap);
                line
            }
            DeviceMessage::Line(line) => line,
            DeviceMessage::Disconnected => return Err("link lost".to_owned()),
            _ => continue,
        };
        if let Some(header) = line.strip_prefix("SHOT BEGIN") {
            let (width, height) = dimensions(header)?;
            size = Some((width, height));
            backlight_on = !header.contains("BACKLIGHT OFF");
            pixels.clear();
            pixels.reserve(width * height * 3);
            continue;
        }
        if line == "SHOT END" {
            let Some((width, height)) = size else { continue };
            if pixels.len() != width * height * 3 {
                return Err(format!("incomplete frame: {} pixels", pixels.len() / 3));
            }
            return Ok(Frame { width, height, pixels, backlight_on });
        }
        if let Some((width, height)) = size && let Some(runs) = line.strip_prefix("SHOT ") {
            decode_runs(runs, &mut pixels, width * height)?;
            // A full frame is a full frame; `SHOT END` only confirms it, so there's no reason to wait for one more line.
            if pixels.len() == width * height * 3 {
                return Ok(Frame { width, height, pixels, backlight_on });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_expand_to_rgb_pixels() {
        let mut pixels = Vec::new();
        decode_runs("f800:2 07e0:1", &mut pixels, 3).expect("valid runs");
        assert_eq!(pixels, vec![255, 0, 0, 255, 0, 0, 0, 255, 0]);
        assert!(decode_runs("zz:1", &mut pixels, 3).is_err());
        assert!(decode_runs("ffff", &mut pixels, 3).is_err());
    }

    #[test]
    fn a_full_frame_encodes_as_png() {
        for (width, height) in [(320, 240), (240, 135)] {
            let frame = Frame { width, height, pixels: vec![0x80; width * height * 3], backlight_on: true };
            let png = encode_png(&frame).expect("encode PNG");
            let decoder = png::Decoder::new(std::io::Cursor::new(png));
            let mut reader = decoder.read_info().expect("read PNG");
            assert_eq!((reader.info().width, reader.info().height), (width as u32, height as u32));
            let mut pixels = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut pixels).expect("decode PNG");
            assert_eq!(pixels, frame.pixels);
        }
    }

    #[test]
    fn screenshot_dimensions_are_positive_and_bounded() {
        assert_eq!(dimensions(" 240x135 BACKLIGHT ON"), Ok((240, 135)));
        assert_eq!(dimensions(" 320x240 BACKLIGHT OFF"), Ok((320, 240)));
        for header in ["", "320", "axb", "0x240", "240x0", "320x241", "18446744073709551615x240"] {
            assert!(dimensions(header).is_err(), "{header}");
        }
    }

    #[test]
    fn oversized_runs_are_rejected_before_expansion() {
        let mut pixels = Vec::new();
        assert!(decode_runs("ffff:18446744073709551615", &mut pixels, 240 * 135).is_err());
        assert!(pixels.is_empty());
        decode_runs("0000:32400", &mut pixels, 240 * 135).unwrap();
        assert!(decode_runs("ffff:1", &mut pixels, 240 * 135).is_err());
        assert_eq!(pixels.len(), 240 * 135 * 3);
    }
}
