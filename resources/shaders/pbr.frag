#version 310 es

precision highp float;
precision highp int;

// Inputs from Vertex Shader
in vec3 v_normal;          // Interpolated normal from vertex shader
in vec3 v_view_dir;        // View direction from vertex shader
in vec3 v_barycentric;     // Barycentric coordinates
in vec3 v_camera_position; // Camera position passed from vertex shader

// Output Color
out vec4 fragColor;

// Uniforms for Lighting and Material Properties
uniform vec3 light_direction;     // Light direction
uniform vec3 light_color;         // Light color
uniform vec3 albedo;              // Surface albedo
uniform float roughness;          // Surface roughness
uniform vec3 base_reflectance;    // Reflectance at normal incidence (F0)
uniform bool visualize_normals;   // Toggle normal visualization

// Uniforms for Edge Visualization
uniform bool visualize_edges;     // Toggle edge visualization
uniform float edge_thickness;    // Thickness of the edge lines

// Constants
const float PI = 3.14159265359;

// GGX/Trowbridge-Reitz Normal Distribution Function (NDF)
float D(float alpha, vec3 N, vec3 H) {
    float alpha2 = alpha * alpha;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float denominator = (NdotH2 * (alpha2 - 1.0) + 1.0);
    return alpha2 / (PI * denominator * denominator);
}

// Schlick-Beckman Geometry Shadowing Function (G1)
float G1(float alpha, vec3 N, vec3 X) {
    float NdotX = max(dot(N, X), 0.0);
    float k = (alpha + 1.0) * (alpha + 1.0) / 8.0; // Smith's k-value approximation
    return NdotX / ((NdotX * (1.0 - k) + k) + 0.00001);
}

// Smith's Geometry Function for both view and light (G)
float G(float alpha, vec3 N, vec3 V, vec3 L) {
    return G1(alpha, N, V) * G1(alpha, N, L);
}

// Fresnel-Schlick approximation
vec3 F(vec3 F0, vec3 V, vec3 H) {
    float VdotH = max(dot(V, H), 0.0);
    return F0 + (vec3(1.0) - F0) * pow(1.0 - VdotH, 5.0);
}

// Lambertian Diffuse (for diffuse light calculation)
vec3 diffuseLambert(vec3 albedo) {
    return albedo / PI;
}

void main() {
    // Normalize the interpolated normal and view direction
    vec3 N = normalize(v_normal);
    vec3 V = normalize(v_view_dir);
    vec3 L = normalize(light_direction);
    vec3 H = normalize(V + L); // Halfway vector between light and view direction

    // Roughness squared (alpha)
    float alpha = roughness * roughness;

    // Fresnel reflectance at normal incidence
    vec3 F0 = base_reflectance;

    // Fresnel term (F), geometry (G), and normal distribution function (D)
    vec3 F_spec = F(F0, V, H);
    float G_spec = G(alpha, N, V, L);
    float D_spec = D(alpha, N, H);

    // Cook-Torrance BRDF: Specular component
    vec3 specularBRDF = (F_spec * G_spec * D_spec) / (4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.000001); // Avoid division by zero

    // Diffuse component (Lambertian)
    vec3 diffuseBRDF = diffuseLambert(albedo);

    // Light intensity (without distance-based attenuation)
    vec3 lightIntensity = light_color;
    vec3 ambientLight = vec3(0.25, 0.25, 0.25); // Basic ambient light
    vec3 normal_color = v_normal * 0.5 * float(visualize_normals);  

    // Combine diffuse and specular contributions
    vec3 color = normal_color + ambientLight + (diffuseBRDF + specularBRDF) * lightIntensity * max(dot(N, L), 0.0);

     // Edge Visualization
    if (visualize_edges) {
        // Calculate the minimum barycentric coordinate
        float minBC = min(min(v_barycentric.x, v_barycentric.y), v_barycentric.z);

        // Determine the thickness threshold
        float threshold = edge_thickness * 0.005; // Adjust the scaling factor as needed

        // Compute the edge factor using smoothstep for smooth, anti-aliased edges
        float edgeFactor = smoothstep(threshold, threshold + fwidth(minBC), minBC);

        // Mix the original color with black based on the edge factor
        color = mix(color, vec3(0.0, 0.0, 0.0), 1.0 - edgeFactor);
    }

    // Set the final fragment color with full opacity
    fragColor = vec4(color, 1.0);
}