#version 140
in vec2 position;
in vec2 tex_coords;
in vec4 color;
out vec2 v_tex_coords;
out vec4 v_color;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_coords = tex_coords;
    v_color = color;
}
