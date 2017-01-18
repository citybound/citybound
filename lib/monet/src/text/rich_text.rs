
use rusttype;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use ::{TextRenderer, Font, Glyph, GlyphIter, TextVertex};

#[derive(Clone)]
pub struct RichText {
    glyphs: Vec<Glyph>,
}

#[derive(Clone)]
pub struct Formatting {
    pub len: usize,
    pub font: Font,
    pub width: u32,
    pub color: [f32; 4],
}

impl RichText {
    pub fn new(text_renderer: &TextRenderer, text: &str, formatting: &[Formatting]) -> RichText {
        use unicode_normalization::UnicodeNormalization;
        let glyphs = GlyphIter::new(text_renderer, &mut text.nfc(), formatting).collect();

        RichText { glyphs: glyphs }
    }

    #[allow(needless_lifetimes)]
    #[inline]
    pub fn glyphs_iter<'a>(&'a self) -> impl Iterator<Item = &'a Glyph> {
        self.glyphs.iter()
    }

    pub fn vertices(&self, text_renderer: &TextRenderer, screen: (f32, f32)) -> Vec<TextVertex> {
        self.glyphs
            .iter()
            .flat_map(|glyph| RichText::glyph_vertices(text_renderer, glyph, screen))
            .collect()
    }

    fn glyph_vertices(text_renderer: &TextRenderer,
                      glyph: &Glyph,
                      (screen_width, screen_height): (f32, f32))
                      -> Vec<TextVertex> {
        let origin = rusttype::point(0.0, 0.0);
        match text_renderer.cache_rect_for(glyph) {
            Some((uv_rect, screen_rect)) => {
                let gl_rect = rusttype::Rect {
                    min: origin +
                         (rusttype::vector(screen_rect.min.x as f32 / screen_width - 0.5,
                                           1.0 - screen_rect.min.y as f32 / screen_height -
                                           0.5)) * 2.0,
                    max: origin +
                         (rusttype::vector(screen_rect.max.x as f32 / screen_width - 0.5,
                                           1.0 - screen_rect.max.y as f32 / screen_height -
                                           0.5)) * 2.0,
                };

                vec![TextVertex {
                         position: [gl_rect.min.x, gl_rect.max.y],
                         tex_coords: [uv_rect.min.x, uv_rect.max.y],
                         color: glyph.color(),
                     },
                     TextVertex {
                         position: [gl_rect.min.x, gl_rect.min.y],
                         tex_coords: [uv_rect.min.x, uv_rect.min.y],
                         color: glyph.color(),
                     },
                     TextVertex {
                         position: [gl_rect.max.x, gl_rect.min.y],
                         tex_coords: [uv_rect.max.x, uv_rect.min.y],
                         color: glyph.color(),
                     },
                     TextVertex {
                         position: [gl_rect.max.x, gl_rect.min.y],
                         tex_coords: [uv_rect.max.x, uv_rect.min.y],
                         color: glyph.color(),
                     },
                     TextVertex {
                         position: [gl_rect.max.x, gl_rect.max.y],
                         tex_coords: [uv_rect.max.x, uv_rect.max.y],
                         color: glyph.color(),
                     },
                     TextVertex {
                         position: [gl_rect.min.x, gl_rect.max.y],
                         tex_coords: [uv_rect.min.x, uv_rect.max.y],
                         color: glyph.color(),
                     }]
            }
            None => vec![],
        }
    }
}
