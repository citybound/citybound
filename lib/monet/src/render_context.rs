
use rusttype;
use glium;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use std::borrow::Cow;

use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;

use super::{Batch, Scene};

pub struct RenderContext {
    pub window: GlutinFacade,
    batch_program: glium::Program,
    text_program: glium::Program,
    text_cache_tex: glium::Texture2d,
    text_cache: rusttype::gpu_cache::Cache,
    font_bank: FontBank,
}

impl RenderContext {
    #[allow(redundant_closure)]
    pub fn new(window: GlutinFacade) -> RenderContext {
        let dpi_factor = window.get_window().unwrap().hidpi_factor();
        let (text_cache_width, text_cache_height) = (512 * dpi_factor as u32,
                                                     512 * dpi_factor as u32);

        RenderContext{
            batch_program: program!(&window, 140 => {
                vertex: include_str!("shader/solid_140.glslv"),
                fragment: include_str!("shader/solid_140.glslf")
            }).unwrap(),
            text_program: program!(&window, 140 => {
                vertex: include_str!("shader/text_140.glslv"),
                fragment: include_str!("shader/text_140.glslf")
            }).unwrap(),
            text_cache_tex: glium::Texture2d::with_format(
                &window,
                glium::texture::RawImage2d{
                    data: Cow::Owned(
                              vec![128u8; text_cache_width as usize * text_cache_height as usize]),
                    width: text_cache_width, height: text_cache_height,
                    format: glium::texture::ClientFormat::U8
                },
                glium::texture::UncompressedFloatFormat::U8,
                glium::texture::MipmapsOption::NoMipmap
            ).unwrap(),
            text_cache: rusttype::gpu_cache::Cache::new(
                text_cache_width, text_cache_height, 0.1, 0.1),
            font_bank: FontBank::new(dpi_factor),
            window: window,
        }
    }

    pub fn submit(&mut self, scene: &Scene) {
        let mut target = self.window.draw();

        let view: [[f32; 4]; 4] =
            *Iso3::look_at_rh(&scene.eye.position, &scene.eye.target, &scene.eye.up)
                .to_homogeneous()
                .as_ref();
        let perspective: [[f32; 4]; 4] = *Persp3::new(target.get_dimensions().0 as f32 /
                                                      target.get_dimensions().1 as f32,
                                                      scene.eye.field_of_view,
                                                      0.1,
                                                      50000.0)
            .to_matrix()
            .as_ref();

        let uniforms = uniform! {
            view: view,
            perspective: perspective
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let decal_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::Overwrite,
                write: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // draw a frame
        target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

        let mut render_debug_text = String::from("Renderer:\n");

        let mut batches_todo = scene.batches.iter().collect::<Vec<_>>();
        batches_todo.sort_by_key(|&(batch_id, _)| batch_id);

        for (i, &Batch { ref vertices, ref indices, ref instances, is_decal, .. }) in batches_todo {
            if instances.len() > 1 {
                render_debug_text.push_str(&format!("batch{}: {} instances\n", i, instances.len()));
            }
            let instance_buffer = glium::VertexBuffer::new(&self.window, instances).unwrap();
            target.draw((vertices, instance_buffer.per_instance().unwrap()),
                      indices,
                      &self.batch_program,
                      &uniforms,
                      if is_decal { &decal_params } else { &params })
                .unwrap();
        }

        let debug_text = self.create_debug_text(scene, &render_debug_text);
        self.render_text(&mut target, &[debug_text]);

        target.finish().unwrap();
    }

    fn create_debug_text(&self, scene: &Scene, render_debug_text: &str) -> RichText {
        let width = {
            let window = self.window.get_window().unwrap();
            window.get_inner_size_pixels().unwrap().0
        };

        let text = scene.debug_text.iter()
                                     .map(|(key, &(ref text, _))| format!("{}:\n{}\n", key, text))
                                     .collect::<String>() + render_debug_text;
        let formatting = scene.debug_text
                                 .iter()
                                 .map(|(key, &(ref text, ref color))| {
                                     Formatting {
                                         len: key.len() + text.len() + 3,
                                         font: Font::Debug,
                                         width: width,
                                         color: *color,
                                     }
                                 })
                                 .chain(Some(Formatting {
                                     len: render_debug_text.len(),
                                     font: Font::Debug,
                                     width: width,
                                     color: [0.0, 0.0, 0.0, 0.5],
                                 }))
                                 .collect::<Vec<_>>();
        RichText::new(self, &text, &formatting)
    }

    fn render_text(&mut self,
                         target: &mut glium::Frame,
                         text: &[RichText]) {
        for text in text {
            text.register_in_cache(self);
        }

        {
            let text_cache_tex = &mut self.text_cache_tex;
            self.text_cache
                .cache_queued(|rect, data| {
                    let rect = glium::Rect {
                        left: rect.min.x,
                        bottom: rect.min.y,
                        width: rect.width(),
                        height: rect.height(),
                    };

                    let image = glium::texture::RawImage2d {
                        data: Cow::Borrowed(data),
                        width: rect.width,
                        height: rect.height,
                        format: glium::texture::ClientFormat::U8,
                    };

                    text_cache_tex.main_level().write(rect, image);
                })
                .unwrap();
        }

        for text in text {
            text.render(self, target);
        }
    }

    pub fn font_bank(&self) -> &FontBank {
        &self.font_bank
    }
}

#[derive(Copy, Clone)]
pub enum Font {
    Debug,
}

pub struct FontBank {
    pub dpi_factor: f32,
    text_font: rusttype::Font<'static>,
}

impl FontBank {
    pub fn new(dpi_factor: f32) -> FontBank {
        FontBank {
            dpi_factor: dpi_factor,
            text_font: rusttype::FontCollection::from_bytes(
                include_bytes!("../../../fonts/ClearSans-Regular.ttf") as &[u8]
            ).into_font().unwrap(),
        }
    }

    pub fn font(&self, font: Font) -> (&rusttype::Font<'static>, rusttype::Scale) {
        let (font, scale) = match font {
            Font::Debug => (&self.text_font, 14.0),
        };

        (font, rusttype::Scale::uniform(scale * self.dpi_factor))
    }
}

#[derive(Copy, Clone)]
struct TextVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
}
implement_vertex!(TextVertex, position, tex_coords, color);

#[derive(Clone)]
pub struct RichText {
    glyphs: Vec<Glyph>,
}

#[derive(Clone)]
pub struct Formatting {
    len: usize,
    font: Font,
    width: u32,
    color: [f32; 4],
}

impl RichText {
    pub fn new(context: &RenderContext, text: &str, formatting: &[Formatting]) -> RichText {
        use unicode_normalization::UnicodeNormalization;
        let glyphs = GlyphIter::new(context, &mut text.nfc(), formatting).collect();

        RichText {
            glyphs: glyphs,
        }
    }

    pub fn register_in_cache(&self, context: &mut RenderContext) {
        for glyph in &self.glyphs {
            let positioned = glyph.positioned(&context.font_bank);
            context.text_cache.queue_glyph(0, positioned);
        }
    }

    pub fn render(&self, context: &RenderContext, target: &mut glium::Frame) {
        let vertices = self.glyphs.iter().flat_map(|glyph| {
                RichText::glyph_vertices(context, glyph)
            })
            .collect::<Vec<_>>();
        let vertices = glium::VertexBuffer::new(&context.window, &vertices).unwrap();

        let text_uniforms = uniform! {
            tex: context.text_cache_tex
                .sampled()
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        };

        target.draw(&vertices,
                  glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                  &context.text_program,
                  &text_uniforms,
                  &glium::DrawParameters {
                      blend: glium::Blend::alpha_blending(),
                      ..Default::default()
                  })
            .unwrap();
    }

    fn glyph_vertices(context: &RenderContext, glyph: &Glyph) -> Vec<TextVertex> {
        let (screen_width, screen_height) = {
            let (w, h) = context.window.get_framebuffer_dimensions();
            (w as f32, h as f32)
        };

        let origin = rusttype::point(0.0, 0.0);
        let rect = context.text_cache.rect_for(0, &glyph.positioned(&context.font_bank));
        if let Ok(Some((uv_rect, screen_rect))) = rect {
            let gl_rect = rusttype::Rect {
                min: origin +
                     (rusttype::vector(screen_rect.min.x as f32 / screen_width - 0.5,
                                       1.0 - screen_rect.min.y as f32 / screen_height - 0.5)) *
                     2.0,
                max: origin +
                     (rusttype::vector(screen_rect.max.x as f32 / screen_width - 0.5,
                                       1.0 - screen_rect.max.y as f32 / screen_height - 0.5)) *
                     2.0,
            };

            vec![TextVertex {
                     position: [gl_rect.min.x, gl_rect.max.y],
                     tex_coords: [uv_rect.min.x, uv_rect.max.y],
                     color: glyph.color,
                 },
                 TextVertex {
                     position: [gl_rect.min.x, gl_rect.min.y],
                     tex_coords: [uv_rect.min.x, uv_rect.min.y],
                     color: glyph.color,
                 },
                 TextVertex {
                     position: [gl_rect.max.x, gl_rect.min.y],
                     tex_coords: [uv_rect.max.x, uv_rect.min.y],
                     color: glyph.color,
                 },
                 TextVertex {
                     position: [gl_rect.max.x, gl_rect.min.y],
                     tex_coords: [uv_rect.max.x, uv_rect.min.y],
                     color: glyph.color,
                 },
                 TextVertex {
                     position: [gl_rect.max.x, gl_rect.max.y],
                     tex_coords: [uv_rect.max.x, uv_rect.max.y],
                     color: glyph.color,
                 },
                 TextVertex {
                     position: [gl_rect.min.x, gl_rect.max.y],
                     tex_coords: [uv_rect.min.x, uv_rect.max.y],
                     color: glyph.color,
                 }]
        } else {
            vec![]
        }
    }
}

#[derive(Copy, Clone)]
struct Glyph {
    font: Font,
    color: [f32; 4],

    glyph_id: rusttype::GlyphId,
    position: rusttype::Point<f32>,
}

impl Glyph {
    pub fn positioned<'a>(&self, font_bank: &'a FontBank) -> rusttype::PositionedGlyph<'a> {
        let (font, scale) = font_bank.font(self.font);
        font.glyph(self.glyph_id).unwrap()
            .scaled(scale)
            .positioned(self.position)
    }
}

struct GlyphIter<'a, 'b, 'c> {
    context: &'a RenderContext,

    offset: usize,
    text_iter: &'b mut Iterator<Item=char>,

    formatting_offset: usize,
    formatting_text_offset: usize,
    formatting: &'c [Formatting],

    caret: rusttype::Point<f32>,
    last_glyph_id: Option<rusttype::GlyphId>,
}

impl<'a, 'b, 'c> GlyphIter<'a, 'b, 'c> {
    fn new(context: &'a RenderContext,
           text_iter: &'b mut Iterator<Item=char>,
           formatting: &'c [Formatting])
           -> GlyphIter<'a, 'b, 'c> {
        GlyphIter {
            context: context,

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

        let (font, scale) = self.context.font_bank.font(format.font);
        let v_metrics = font.v_metrics(scale);
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

        let base_glyph = match font.glyph(c) {
            Some(glyph) => glyph,
            None => {
                // If the character does not exist, try the replacement character
                match font.glyph('�') {
                    Some(glyph) => glyph,
                    None => return None,
                }
            }
        };

        if let Some(id) = self.last_glyph_id.take() {
            self.caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        self.last_glyph_id = Some(base_glyph.id());

        let glyph = base_glyph.scaled(scale).positioned(self.caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > format.width as i32 {
                self.caret = rusttype::point(0.0, self.caret.y + advance_height);
                self.last_glyph_id = None;
            }
        }

        let h_metrics = glyph.unpositioned().h_metrics();

        let glyph = Glyph {
            font: format.font,
            color: format.color,
            glyph_id: glyph.id(),
            position: self.caret,
        };

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
