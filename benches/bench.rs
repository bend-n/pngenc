#![feature(test, array_chunks)]
extern crate test;
const W: u32 = 2144;
const H: u32 = 1424;
use atools::prelude::*;

fn rgba() -> &'static [u8] {
    Box::leak(std::hint::black_box(
        rgb()
            .array_chunks::<3>()
            .flat_map(|&x| x.join(255))
            .collect::<Box<_>>(),
    ))
}

fn rgb() -> &'static [u8] {
    std::hint::black_box(include_bytes!("cat"))
}

fn ya() -> &'static [u8] {
    Box::leak(std::hint::black_box(
        y().iter().flat_map(|&x| x.join(255)).collect::<Box<_>>(),
    ))
}

fn y() -> &'static [u8] {
    Box::leak(std::hint::black_box(
        include_bytes!("cat")
            .array_chunks::<3>()
            .map(|&[r, g, b]| ((2126 * r as u32 + 7152 * g as u32 + 722 * b as u32) / 10000) as u8)
            .collect::<Box<_>>(),
    ))
}

macro_rules! m {
    ($f:ident, $c:ident, $c2:ident, me $me:ident them $them: ident) => {
        #[bench]
        fn $me(b: &mut test::Bencher) {
            let mut v = Vec::with_capacity(10 << 20);
            let dat = $f();
            // pngenc::encode(
            //     pngenc::Color::$c,
            //     (W, H),
            //     dat,
            //     &mut std::fs::File::create(stringify!($f)).unwrap(),
            // )
            // .unwrap();
            b.bytes = dat.len() as _;
            b.iter(|| {
                v.clear();
                pngenc::ode(pngenc::Color::$c, (W, H), dat, &mut v).unwrap();
                std::hint::black_box(&v);
            })
        }

        #[bench]
        fn $them(b: &mut test::Bencher) {
            let mut v = Vec::with_capacity(10 << 20);
            let dat = $f();
            b.bytes = dat.len() as _;
            b.iter(|| {
                v.clear();
                let mut enc = png::Encoder::new(&mut v, W, H);
                enc.set_color(png::ColorType::$c2);
                enc.set_depth(png::BitDepth::Eight);
                enc.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));
                enc.set_source_chromaticities(png::SourceChromaticities::new(
                    (0.31270, 0.32900),
                    (0.64000, 0.33000),
                    (0.30000, 0.60000),
                    (0.15000, 0.06000),
                ));
                let mut writer = enc.write_header().unwrap();
                writer.write_image_data(dat).unwrap();
                drop(writer);
                std::hint::black_box(&v);
            })
        }
    };
}
m![rgba, RGBA, Rgba, me pngenc_rgba them png_rgba];
m![rgb, RGB, Rgb, me pngenc_rgb them png_rgb];
m![ya, YA, GrayscaleAlpha, me pngenc_ya them png_ya];
m![y, Y, Grayscale, me pngenc_y them png_y];
