//! uncompressing png encoding crate
//! ```
//! # let mut v = vec![];
//! pngenc::ode(
//!   pngenc::RGB, // color type
//!   (2144, 1424), // width and height
//!   include_bytes!("../benches/cat"), // image to encode
//! # /*
//!   &mut std::fs::File::create("hey.png").unwrap(), // output writer
//! # */
//! # &mut v,
//! ).unwrap();
//! # assert_eq!(crc32fast::hash(&v), 0xd7d47c0e);
//! ```
#![allow(incomplete_features)]
#![feature(generic_const_exprs, test, slice_as_chunks, array_chunks)]
use atools::prelude::*;
use std::{
    io::{self, Write},
    iter::once,
};

pub use Color::*;

#[derive(Copy, Debug, Clone)]
#[repr(u8)]
/// Color types.
pub enum Color {
    /// Grayscale
    Y = 1,
    /// Grayscale with alpha
    YA,
    /// Red, green, blue
    RGB,
    /// RGB with alpha
    RGBA,
}

impl Color {
    /// Color depth (number of channels)
    #[must_use]
    pub const fn depth(self) -> u8 {
        self as u8
    }

    const fn ty(self) -> u8 {
        match self {
            Color::Y => 0,
            Color::YA => 4,
            Color::RGB => 2,
            Color::RGBA => 6,
        }
    }
}

trait W: Write {
    fn u32(&mut self, x: u32) -> io::Result<()> {
        self.write_all(&x.to_be_bytes())
    }
    fn w(&mut self, x: impl AsRef<[u8]>) -> io::Result<()> {
        self.write_all(x.as_ref())
    }
}

impl<T: Write> W for T {}

const HEADER: &[u8; 8] = b"\x89PNG\x0d\x0a\x1a\x0a";

fn chunk_len(x: usize) -> usize {
    4 // length
        + 4 // type
        + x // data
        + 4 // crc
}

/// Get the size of an encoded png. Guaranteed to exactly equal the size of the encoded png.
pub fn size(color: Color, (width, height): (u32, u32)) -> usize {
    HEADER.len()
        + chunk_len(13) // IHDR
        + chunk_len(deflate_size(((width * color.depth() as u32 + 1) * height) as usize)) // IDAT
        + chunk_len(1) // sRGB
        + chunk_len(0) // IEND
}

#[doc(alias = "encode")]
/// Encode a png without any compression.
/// Takes advantage of the [Non-compressed blocks](http://www.zlib.org/rfc-deflate.html#noncompressed) deflate feature.
///
/// If you *do* want a compressed image, I recommend the [oxipng](https://docs.rs/oxipng/latest/oxipng/struct.RawImage.html) raw image api.
///
/// # Panics
///
/// if your width * height * color depth isnt data's length
pub fn ode(
    color: Color,
    (width, height): (u32, u32),
    data: &[u8],
    to: &mut impl Write,
) -> std::io::Result<()> {
    assert_eq!(
        (width as usize * height as usize)
            .checked_mul(color.depth() as usize)
            .unwrap(),
        data.len(),
        "please dont lie to me"
    );
    to.w(HEADER)?;
    chunk(
        *b"IHDR",
        &width
            .to_be_bytes()
            .couple(height.to_be_bytes())
            .join(8) // bit depth
            .join(color.ty())
            .join(0)
            .join(0)
            .join(0),
        to,
    )?;

    // removing this allocation is not a performance gain
    let mut scanned = Vec::<u8>::with_capacity(((width * color.depth() as u32 + 1) * height) as _);
    let mut out = scanned.as_mut_ptr();

    data.chunks(width as usize * color.depth() as usize)
        // set filter type for each scanline
        .flat_map(|x| once(0).chain(x.iter().copied()))
        .for_each(|x| unsafe {
            out.write(x);
            out = out.add(1);
        });
    unsafe { scanned.set_len(((width * color.depth() as u32 + 1) * height) as _) };

    let data = deflate(&scanned);
    chunk(*b"sRGB", &[0], to)?;
    chunk(*b"IDAT", &data, to)?;
    chunk(*b"IEND", &[], to)?;
    Ok(())
}

fn chunk(ty: [u8; 4], data: &[u8], to: &mut impl Write) -> std::io::Result<()> {
    to.u32(data.len() as _)?;
    to.w(ty)?;
    to.w(data)?;
    let mut crc = crc32fast::Hasher::new();
    crc.update(&ty);
    crc.update(data);
    to.u32(crc.finalize())?;
    Ok(())
}

fn deflate_size(x: usize) -> usize {
    // 2 header bytes, each header of chunk, and add remainder chunk, along with 4 bytes for adler32
    2 + 5 * (x / CHUNK_SIZE) + usize::from(x != (x / CHUNK_SIZE) * CHUNK_SIZE || x == 0) + x + 4 + 4
}

trait P<T: Copy> {
    unsafe fn put<const N: usize>(&mut self, x: [T; N]);
}

impl<T: Copy> P<T> for *mut T {
    #[cfg_attr(debug_assertions, track_caller)]
    unsafe fn put<const N: usize>(&mut self, x: [T; N]) {
        self.copy_from(x.as_ptr(), N);
        *self = self.add(N);
    }
}

const CHUNK_SIZE: usize = 0xffff;
fn deflate(data: &[u8]) -> Vec<u8> {
    let mut adler = simd_adler32::Adler32::new();
    let (chunks, remainder) = data.as_chunks::<CHUNK_SIZE>();
    // SAFETY: deflate_size is very correct.
    let mut out = Vec::<u8>::with_capacity(deflate_size(data.len()));
    let mut optr = out.as_mut_ptr();
    /// return LSB and SLSB
    fn split(n: u16) -> [u8; 2] {
        [(n & 0xff) as u8, ((n >> 8) & 0xff) as u8]
    }
    // 32k window
    unsafe { optr.put([0b1_111_000, 1]) };
    chunks.iter().for_each(|x| unsafe {
        adler.write(x);
        // http://www.zlib.org/rfc-deflate.html#noncompressed
        optr.put(
            [0b000]
                // lsb and slsb [255, 255]
                .couple(split(CHUNK_SIZE as _))
                // ones complement -- [0, 0]
                .couple(split(CHUNK_SIZE as _).map(|x| !x)),
        );
        optr.put(*x);
    });
    unsafe {
        adler.write(remainder);
        optr.put(
            [0b001]
                .couple(split(CHUNK_SIZE as _))
                .couple(split(CHUNK_SIZE as _).map(|x| !x)),
        );
        optr.copy_from(remainder.as_ptr(), remainder.len());
        optr = optr.add(remainder.len());
    };
    unsafe { optr.put(adler.finish().to_be_bytes()) };
    unsafe { out.set_len(deflate_size(data.len())) }
    out
}
