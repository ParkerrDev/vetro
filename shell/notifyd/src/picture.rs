//! A picture handed over in memory, made into the bytes of a PNG file the
//! shell can load. It is drawn small (46 points), so a big one is shrunk on
//! the way, by averaging blocks of pixels, to at most [`MAX_SIDE`] a side.

use crate::note::Pixels;

/// The widest or tallest a picture is written.
pub const MAX_SIDE: u32 = 256;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the picture's rows are shorter than it says")]
    Short,
    #[error("the picture could not be written as a PNG: {0}")]
    Encode(#[from] png::EncodingError),
}

/// How many source pixels make one written pixel, on each side.
fn factor(pixels: &Pixels) -> u32 {
    pixels.width.max(pixels.height).div_ceil(MAX_SIDE).max(1)
}

/// One written pixel: the average of a block of source pixels, as RGBA.
fn block(pixels: &Pixels, x: u32, y: u32, step: u32) -> Result<[u8; 4], Error> {
    let channels: usize = if pixels.alpha { 4 } else { 3 };
    let mut sum = [0u32; 4];
    let mut count = 0u32;
    let rows = y.saturating_mul(step)..y.saturating_add(1).saturating_mul(step).min(pixels.height);
    for row in rows {
        let start = usize::try_from(row)
            .map_err(|_| Error::Short)?
            .saturating_mul(pixels.stride);
        let columns =
            x.saturating_mul(step)..x.saturating_add(1).saturating_mul(step).min(pixels.width);
        for column in columns {
            let at = start.saturating_add(
                usize::try_from(column)
                    .map_err(|_| Error::Short)?
                    .saturating_mul(channels),
            );
            let source = pixels
                .data
                .get(at..at.saturating_add(channels))
                .ok_or(Error::Short)?;
            for (total, value) in sum
                .iter_mut()
                .zip(source.iter().copied().chain(std::iter::once(255)))
            {
                *total = total.saturating_add(u32::from(value));
            }
            count = count.saturating_add(1);
        }
    }
    let count = count.max(1);
    let mut out = [0u8; 4];
    for (written, total) in out.iter_mut().zip(sum) {
        *written = u8::try_from(total.checked_div(count).unwrap_or(0)).unwrap_or(u8::MAX);
    }
    Ok(out)
}

/// The picture as the bytes of a PNG file, RGBA, shrunk to fit [`MAX_SIDE`].
///
/// # Errors
/// When the rows are shorter than the picture says (checked when it was read,
/// so only for a picture made some other way), or the encoder fails.
pub fn png(pixels: &Pixels) -> Result<Vec<u8>, Error> {
    let step = factor(pixels);
    let (width, height) = (pixels.width.div_ceil(step), pixels.height.div_ceil(step));
    let capacity = usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or(0);
    let mut rgba = Vec::with_capacity(capacity);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&block(pixels, x, y, step)?);
        }
    }
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgba)?;
    writer.finish()?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::{MAX_SIDE, png};
    use crate::note::Pixels;

    fn decoded(file: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(file));
        let Ok(mut reader) = decoder.read_info() else {
            return (0, 0, Vec::new());
        };
        let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
        let Ok(info) = reader.next_frame(&mut buffer) else {
            return (0, 0, Vec::new());
        };
        buffer.truncate(info.buffer_size());
        (info.width, info.height, buffer)
    }

    #[test]
    fn writes_rgb_as_opaque_rgba() {
        let pixels = Pixels {
            width: 2,
            height: 1,
            stride: 8,
            alpha: false,
            data: vec![255, 0, 0, 0, 0, 255, 9, 9],
        };
        let (width, height, data) = decoded(&png(&pixels).unwrap_or_default());
        assert_eq!((width, height), (2, 1));
        assert_eq!(data, vec![255, 0, 0, 255, 0, 0, 255, 255]);
    }

    #[test]
    fn shrinks_a_big_picture_by_averaging() {
        let side = MAX_SIDE * 2;
        let row = usize::try_from(side * 4).unwrap_or(0);
        let mut data = Vec::new();
        for y in 0..side {
            for _ in 0..side {
                data.extend_from_slice(&[if y % 2 == 0 { 200 } else { 100 }, 0, 0, 255]);
            }
        }
        let pixels = Pixels {
            width: side,
            height: side,
            stride: row,
            alpha: true,
            data,
        };
        let (width, height, data) = decoded(&png(&pixels).unwrap_or_default());
        assert_eq!((width, height), (MAX_SIDE, MAX_SIDE));
        assert_eq!(data.first().copied(), Some(150));
    }

    #[test]
    fn refuses_rows_that_are_short() {
        let pixels = Pixels {
            width: 2,
            height: 2,
            stride: 8,
            alpha: true,
            data: vec![0; 10],
        };
        assert!(png(&pixels).is_err());
    }
}
