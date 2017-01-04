#version 140
uniform sampler2D tex;

in vec2 v_tex_coords;

out vec4 f_color;

void main() {
    vec4 base_color = vec4(0.0, 0.0, 0.0, 1.0);
    float dist_alpha_map = texture(tex, v_tex_coords).a;

    base_color.a *= smoothstep(0.49, 0.51, dist_alpha_map);
    f_color = base_color;
}
