// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.
#version 310 es

precision highp float;
precision highp int;

in vec3 v_normal;
in vec3 v_view_dir;

uniform vec3 light_direction; // Uniform to control the light direction
out vec4 fragColor;

void main() {
    // Normalize the normal
    vec3 normal = normalize(v_normal);

    // Normalize the light direction
    vec3 lightDir = normalize(light_direction);

    // Normalize the view direction
    vec3 viewDir = normalize(v_view_dir);

    // Ambient component
    vec3 ambient = 0.2 * vec3(1.0, 1.0, 1.0); // 20% ambient light

    // Diffuse component
    float diff = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = diff * vec3(1.0, 1.0, 1.0); // White light

    // Specular component (Phong)
    vec3 reflectDir = reflect(-lightDir, normal);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32.0);
    vec3 specular = 0.5 * spec * vec3(1.0, 1.0, 1.0); // Specular strength of 0.5

    // Color contribution by normals for visualization
    vec3 normals_color = normal * 0.1 + 0.5;

    // Combine all components
    vec3 final_color = (ambient + diffuse + specular) * normals_color;
    fragColor = vec4(final_color, 1.0);
}
