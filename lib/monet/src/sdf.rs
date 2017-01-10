
use glium;
use glium::backend::glutin_backend::GlutinFacade;
use image::RgbaImage;

use std::borrow::Cow;

pub enum Glyph {
    Basic(BasicGlyph),
    Sdf(SdfGlyph),
    Msdf(MsdfGlyph),
}

pub struct BasicGlyph;

pub struct MsdfGlyph;

pub struct SdfGlyph {
    texture: glium::Texture2d,
    aspect: f32,
}

impl SdfGlyph {
    pub fn new(window: &GlutinFacade, image: &mut RgbaImage, size: u32) -> SdfGlyph {
        // Implementation of dead reckoning.

        if !size.is_power_of_two() {
            panic!("size must be a power of two");
        }

        const ALPHA_CUTOFF: u8 = 127;

        // These distances are squared
        const DIST_CROSS: f32 = 1.0;
        const DIST_DIAGONAL: f32 = 2.;

        let (width, height) = image.dimensions();
        if width < height {
            panic!("width must be greater than or equal to height");
        }

        let offset = |x: u32, y: u32, dx: i32, dy: i32| {
            let x = if dx < 0 {
                x.saturating_sub((-dx) as u32)
            } else {
                x.saturating_add(dx as u32)
            };
            let y = if dy < 0 {
                y.saturating_sub((-dy) as u32)
            } else {
                y.saturating_add(dy as u32)
            };
            (::std::cmp::min(width - 1, x), ::std::cmp::min(height - 1, y))
        };

        let is_inside = |x: u32, y: u32, dx: i32, dy: i32| {
            let (x, y) = offset(x, y, dx, dy);
            image.get_pixel(x, y).data[3] > ALPHA_CUTOFF
        };
        let xy_index = |x: u32, y: u32, dx: i32, dy: i32| {
            let (x, y) = offset(x, y, dx, dy);
            (x + y * width) as usize
        };

        let mut points = ::std::iter::repeat(None)
            .take((width * height) as usize)
            .collect::<Vec<Option<(u32, u32)>>>();
        let mut sdf = ::std::iter::repeat(::std::f32::INFINITY)
            .take((width * height) as usize)
            .collect::<Vec<_>>();

        let compare = |points: &mut [Option<(u32, u32)>],
                       sdf: &mut [f32],
                       x: u32,
                       y: u32,
                       dx: i32,
                       dy: i32| {
            let d = if dx != 0 && dy != 0 {
                DIST_DIAGONAL
            } else {
                DIST_CROSS
            };
            let dindex = xy_index(x, y, dx, dy);
            let index = xy_index(x, y, 0, 0);

            if sdf[dindex] + d < sdf[index] {
                points[index] = points[dindex];

                let point = points[dindex].unwrap();
                sdf[index] = (point.0 as f32 - x as f32).powi(2) +
                             (point.1 as f32 - y as f32).powi(2);
                assert!(!sdf[index].is_nan());
            }
        };

        for y in 0..height {
            for x in 0..width {
                let index = xy_index(x, y, 0, 0);
                let pixel = is_inside(x, y, 0, 0);

                if is_inside(x, y, -1, 0) != pixel || is_inside(x, y, 1, 0) != pixel ||
                   is_inside(x, y, 0, -1) != pixel ||
                   is_inside(x, y, 0, 1) != pixel {
                    sdf[index] = 0.;
                    points[index] = Some((x, y));
                }
            }
        }

        for y in 0..height {
            for x in 0..width {
                compare(&mut points, &mut sdf, x, y, -1, -1);
                compare(&mut points, &mut sdf, x, y, 0, -1);
                compare(&mut points, &mut sdf, x, y, 1, -1);
                compare(&mut points, &mut sdf, x, y, -1, 0);
            }
        }

        for y in (0..height).rev() {
            for x in (0..width).rev() {
                compare(&mut points, &mut sdf, x, y, 1, 0);
                compare(&mut points, &mut sdf, x, y, -1, 1);
                compare(&mut points, &mut sdf, x, y, 0, 1);
                compare(&mut points, &mut sdf, x, y, 1, 1);
            }
        }

        let (out_width, out_height, scale) = min_dimensions(width, height, size);
        let mut image_data = ::std::iter::repeat(0u8)
            .take((out_width * out_height) as usize)
            .collect::<Vec<_>>();

        for y in 0..out_height {
            for x in 0..out_width {
                let ix = (x as f32 * scale) as u32;
                let iy = (y as f32 * scale) as u32;
                let iw = ::std::cmp::min(width - 1, ix + scale as u32);
                let ih = ::std::cmp::min(height - 1, iy + scale as u32);

                // Kahan summation
                let mut sum = 0_f32;
                let mut c = 0_f32;

                for iy in iy..ih {
                    for ix in ix..iw {
                        let dist = sdf[xy_index(ix, iy, 0, 0)].sqrt();
                        let dist = if is_inside(ix, iy, 0, 0) { dist } else { -dist };

                        let y = dist / scale.powi(2) - c;
                        let t = sum + y;
                        c = (t - sum) - y;
                        sum = t;
                        assert!(!sum.is_nan());
                    }
                }

                if iy < ih && ix < iw {
                    let dist = (sum + 127.).max(0.).min(255.) as u8;
                    image_data[(x + y * out_width) as usize] = dist;
                }
            }
        }

        let texture =
            glium::Texture2d::with_format(window,
                                          glium::texture::RawImage2d {
                                              data: Cow::Owned(image_data),
                                              width: out_width,
                                              height: out_height,
                                              format: glium::texture::ClientFormat::U8,
                                          },
                                          glium::texture::UncompressedFloatFormat::U8,
                                          glium::texture::MipmapsOption::NoMipmap)
                .unwrap();

        SdfGlyph {
            texture: texture,
            aspect: width as f32 / height as f32,
        }
    }

    #[inline]
    pub fn texture(&self) -> &glium::Texture2d {
        &self.texture
    }

    #[inline]
    pub fn aspect(&self) -> f32 {
        self.aspect
    }
}

fn min_dimensions(width: u32, height: u32, size: u32) -> (u32, u32, f32) {
    fn reduce(mut ratio: u32, size: u32) -> u32 {
        // Find the closest power of two
        while !ratio.is_power_of_two() {
            ratio -= 1;
        }
        size / ratio
    }

    if width >= height {
        (size, reduce(width / height, size), width as f32 / size as f32)
    } else {
        (reduce(height / width, size), size, height as f32 / size as f32)
    }
}
