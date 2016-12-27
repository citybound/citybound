
use rusttype;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use ::{TextRenderer, Font, FontDescription, Formatting};

#[derive(Clone)]
pub struct Glyph {
    font: Font,
    color: [f32; 4],

    positioned: rusttype::PositionedGlyph<'static>,
    glyph_id: rusttype::GlyphId,
    position: rusttype::Point<f32>,
}

impl Glyph {
    fn new(font: Font, font_desc: FontDescription,
           color: [f32; 4], glyph_id: rusttype::GlyphId,
           position: rusttype::Point<f32>) -> Glyph {
        let positioned = font_desc.font()
            .glyph(glyph_id)
            .unwrap()
            .scaled(font_desc.scale())
            .positioned(position);

        Glyph {
            font: font,
            color: color,

            positioned: positioned,
            glyph_id: glyph_id,
            position: position,
        }
    }

    pub fn positioned(&self) -> &rusttype::PositionedGlyph<'static> {
        &self.positioned
    }

    pub fn font(&self) -> Font {
        self.font
    }

    pub fn color(&self) -> [f32; 4] {
        self.color
    }
}

pub struct GlyphIter<'a, 'b, 'c> {
    text_renderer: &'a TextRenderer,

    offset: usize,
    text_iter: &'b mut Iterator<Item = char>,

    formatting_offset: usize,
    formatting_text_offset: usize,
    formatting: &'c [Formatting],

    caret: rusttype::Point<f32>,
    last_glyph_id: Option<rusttype::GlyphId>,
}

impl<'a, 'b, 'c> GlyphIter<'a, 'b, 'c> {
    pub fn new(text_renderer: &'a TextRenderer,
               text_iter: &'b mut Iterator<Item = char>,
               formatting: &'c [Formatting])
               -> GlyphIter<'a, 'b, 'c> {
        GlyphIter {
            text_renderer: text_renderer,

            offset: 0,
            text_iter: text_iter,

            formatting_offset: 0,
            formatting_text_offset: 0,
            formatting: formatting,

            caret: rusttype::point(0.0, 0.0),
            last_glyph_id: None,
        }
    }

    fn next(&mut self, c: char) -> Option<Glyph> {
        let format = &self.formatting[self.formatting_offset];
        let format = if format.len <= self.formatting_text_offset {
            self.formatting_offset += 1;
            self.formatting_text_offset -= format.len;
            &self.formatting[self.formatting_offset]
        } else {
            format
        };

        let font_desc = self.text_renderer.font_bank().font(format.font);
        let v_metrics = font_desc.font().v_metrics(font_desc.scale());
        let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        // todo: we should keep track of the tallest character in the row
        if v_metrics.ascent > self.caret.y {
            self.caret.y = v_metrics.ascent;
        }

        if c.is_control() {
            if c == '\n' {
                self.caret = rusttype::point(0.0, self.caret.y + advance_height);
            }
            return None;
        }

        let base_glyph = match font_desc.font().glyph(c) {
            Some(glyph) => glyph,
            None => {
                // If the character does not exist, try the replacement character
                match font_desc.font().glyph('�') {
                    Some(glyph) => glyph,
                    None => return None,
                }
            }
        };

        if let Some(id) = self.last_glyph_id.take() {
            self.caret.x += font_desc.font().pair_kerning(font_desc.scale(), id, base_glyph.id());
        }
        self.last_glyph_id = Some(base_glyph.id());

        let glyph = base_glyph.scaled(font_desc.scale()).positioned(self.caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > format.width as i32 {
                self.caret = rusttype::point(0.0, self.caret.y + advance_height);
                self.last_glyph_id = None;
            }
        }

        let h_metrics = glyph.unpositioned().h_metrics();
        let glyph = Glyph::new(format.font, font_desc, format.color, glyph.id(), self.caret);

        self.caret.x += h_metrics.advance_width;
        Some(glyph)
    }
}

impl<'a, 'b, 'c> Iterator for GlyphIter<'a, 'b, 'c> {
    type Item = Glyph;

    fn next(&mut self) -> Option<Glyph> {
        #[allow(while_let_on_iterator)]
        while let Some(c) = self.text_iter.next() {
            let next = self.next(c);
            self.offset += 1;
            self.formatting_text_offset += 1;

            if let Some(next) = next {
                return Some(next);
            }
        }
        None
    }
}
