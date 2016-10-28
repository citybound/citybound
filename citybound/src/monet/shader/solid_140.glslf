#version 140
out vec4 f_color;
in vec3 p;
void main() {
    f_color = vec4(1.0, p.x, p.y, 1.0);
}