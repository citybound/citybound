#version 140
uniform mat4 view;
uniform mat4 perspective;
in vec3 position;
in vec3 instance_position;
in vec3 instance_color;
in vec2 instance_direction;
out vec3 p;
out vec3 color;

void main() {
    mat4 model = mat4(1.0); // unit diagonal
    mat4 modelview = view * model;
    vec2 orth_instance_direction = vec2(-instance_direction.y, instance_direction.x);
    vec3 rotated_position = vec3(position.x * instance_direction + position.y * orth_instance_direction, position.z);
    gl_Position = perspective * modelview * vec4(rotated_position + instance_position, 1.0);
    p = position;
    color = instance_color;
}