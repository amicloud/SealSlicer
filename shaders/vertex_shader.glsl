// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.
#version 310 es

precision highp float;
precision highp int;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

uniform mat4 view_proj; // View-projection matrix
uniform vec3 view_direction; // Camera view direction
uniform mat4 model;

out vec3 v_normal; // To be interpolated and used in the fragment shader
out vec3 v_view_dir;

void main() {
    gl_Position = view_proj * model * vec4(position, 1.0);
    v_normal = normal;
    v_view_dir = view_direction; // Pass view direction to fragment shader
}
