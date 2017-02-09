# monet

monet is a...

- [X] 3D Rendering engine on top of `kay`
- [X] That uses message passing for render commands
- [ ] For naturalistic and stylized rendering of huge worlds

It offers...

- [X] instanced drawing of geometry batches
- [X] rendering flat colors and geometry
- [ ] procedural textures
- [ ] light, shadow and shading
   - [ ] basic
   - [ ] deferred
   - [ ] physically-based
- [ ] volumetric materials
   - [ ] water
      - [ ] oceans/lakes
      - [ ] puddles
      - [ ] rivers
   - [ ] clouds
   - [ ] grass/leaves
   - [ ] particle effects

It internally uses...

- [X] OpenGL through `glium`
- [ ] OpenGL/Vulkan/DirectX/Metal through `gfx-rs`
- [X] A `kay` message queue for drawing commands
   - [ ] ...which is used directly as a zero-copy draw-command buffer for the graphics API

The main ambition of monet is to render landscapes and architecture in various lighting conditions and seasons, it is thus named after the impressionist [Claude Monet](https://en.wikipedia.org/wiki/Claude_Monet), who devoted his art to studying exactly that.