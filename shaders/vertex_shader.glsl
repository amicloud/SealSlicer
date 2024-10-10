#version 310 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

uniform mat4 view_proj; // View-projection matrix
uniform vec3 view_direction; // Camera view direction

out vec3 v_normal; // To be interpolated and used in the fragment shader
out vec3 v_view_dir;

void main() {
    gl_Position = view_proj * vec4(position, 1.0);
    v_normal = normal;
    v_view_dir = view_direction; // Pass view direction to fragment shader
}
