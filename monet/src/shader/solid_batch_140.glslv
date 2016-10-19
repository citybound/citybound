#version 140
uniform mat4 model;
uniform mat4 view;
uniform mat4 perspective;
in vec3 position;
in vec3 world_position;
out vec3 p;
void main() {
    mat4 modelview = view * model;
    gl_Position = perspective * modelview * vec4(position + world_position, 1.0);
    p = position;
}