//! The Re-Link badge: a coloured disc with two white chain links, drawn in
//! code so the tray can show every status without image assets.

pub type Rgb = [u8; 3];

pub const GREEN: Rgb = [0x2E, 0x9E, 0x5B];
pub const AMBER: Rgb = [0xE0, 0x9B, 0x1A];
pub const RED: Rgb = [0xD1, 0x3A, 0x3A];
pub const GRAY: Rgb = [0x7A, 0x80, 0x88];
pub const BLUE: Rgb = [0x2F, 0x6F, 0xD6];

/// `size`×`size` straight (not premultiplied) RGBA pixels, top-down.
pub fn render(size: usize, c: Rgb) -> Vec<u8> {
    let s = size as f32;
    let (cx, cy, r) = (s / 2.0, s / 2.0, s / 2.0 - 0.5);
    let (link_len, link_rad, stroke, off) = (s * 0.20, s * 0.13, s * 0.075, s * 0.12);
    const SS: usize = 4;
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let (mut disc, mut white) = (0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;
                    let (dx, dy) = (px - cx, py - cy);
                    if (dx * dx + dy * dy).sqrt() > r {
                        continue;
                    }
                    disc += 1.0;
                    let u = (dx - dy) / std::f32::consts::SQRT_2;
                    let v = (dx + dy) / std::f32::consts::SQRT_2;
                    if on_capsule(u - off, v, link_len, link_rad, stroke)
                        || on_capsule(u + off, v, link_len, link_rad, stroke)
                    {
                        white += 1.0;
                    }
                }
            }
            let n = (SS * SS) as f32;
            let a = disc / n;
            let w = if disc > 0.0 { white / disc } else { 0.0 };
            let i = (y * size + x) * 4;
            for k in 0..3 {
                out[i + k] = (c[k] as f32 * (1.0 - w) + 255.0 * w).round() as u8;
            }
            out[i + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}

/// Whether (u, v) lies on the outline of a capsule centred at the origin,
/// with straight length `l` along u and radius `rad`.
fn on_capsule(u: f32, v: f32, l: f32, rad: f32, stroke: f32) -> bool {
    let h = l / 2.0;
    let cu = u.clamp(-h, h);
    let d = ((u - cu).powi(2) + v * v).sqrt();
    (d - rad).abs() <= stroke / 2.0
}

/// A Windows .ico holding the badge at several sizes (PNG-less, 32-bit BMP
/// frames), for the executable's icon.
pub fn ico(c: Rgb, sizes: &[usize]) -> Vec<u8> {
    let mut frames = Vec::new();
    for &size in sizes {
        let px = render(size, c);
        let mut f = Vec::new();
        // BITMAPINFOHEADER, height doubled for the AND mask.
        f.extend_from_slice(&40u32.to_le_bytes());
        f.extend_from_slice(&(size as i32).to_le_bytes());
        f.extend_from_slice(&((size * 2) as i32).to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&32u16.to_le_bytes());
        f.extend_from_slice(&[0u8; 24]);
        for y in (0..size).rev() {
            for x in 0..size {
                let i = (y * size + x) * 4;
                f.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
            }
        }
        let mask_row = size.div_ceil(32) * 4;
        f.extend(std::iter::repeat_n(0u8, mask_row * size));
        frames.push((size, f));
    }
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * frames.len();
    for (size, f) in &frames {
        let d = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[d, d, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(f.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += f.len();
    }
    for (_, f) in frames {
        out.extend(f);
    }
    out
}

/// The badge as a PNG (stored, uncompressed deflate; no extra dependency), for
/// the window icon.
pub fn png(c: Rgb, size: usize) -> Vec<u8> {
    let px = render(size, c);
    let mut raw = Vec::with_capacity((size * 4 + 1) * size);
    for row in px.chunks(size * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(b) = chunks.next() {
        z.push(u8::from(chunks.peek().is_none()));
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    for (kind, data) in [(b"IHDR", ihdr), (b"IDAT", z), (b"IEND", Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(&data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_has_transparent_corners_a_disc_and_white_links() {
        let px = render(32, GREEN);
        assert_eq!(px.len(), 32 * 32 * 4);
        assert_eq!(px[3], 0);
        let opaque = |p: &[u8], c: [u8; 3]| p[3] == 255 && p[..3] == c;
        assert!(px.chunks(4).any(|p| opaque(p, [255, 255, 255])));
        assert!(px.chunks(4).any(|p| opaque(p, GREEN)));
    }

    #[test]
    fn ico_directory_is_consistent() {
        let data = ico(BLUE, &[16, 32, 48]);
        assert_eq!(&data[..6], &[0, 0, 1, 0, 3, 0]);
        let last = 6 + 16 * 2;
        let size = u32::from_le_bytes(data[last + 8..last + 12].try_into().unwrap()) as usize;
        let off = u32::from_le_bytes(data[last + 12..last + 16].try_into().unwrap()) as usize;
        assert_eq!(off + size, data.len());
    }

    #[test]
    fn png_has_signature_and_known_crc() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        let data = png(BLUE, 16);
        assert_eq!(
            &data[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
        assert_eq!(&data[data.len() - 8..data.len() - 4], b"IEND");
    }
}
