#version 140
out vec4 f_color;
in vec3 p;
in vec3 color;
void main() {
    f_color = vec4(color, 1.0);
}