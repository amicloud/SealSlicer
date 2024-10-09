#version 310 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

uniform mat4 view_proj;

out vec3 v_normal;

void main() {
    gl_Position = view_proj * vec4(position, 1.0);
    v_normal = normal;
}
