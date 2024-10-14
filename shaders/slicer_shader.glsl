#version 310 es

precision highp float;
precision highp int;
precision highp uint;

layout(local_size_x = 256) in;

// Binding points must match the SSBO bindings in Rust
layout(std430, binding = 0) buffer MeshBuffer {
    float vertices[]; // Flattened list of triangle vertices
};

layout(std430, binding = 1) buffer SlicePlanes {
    float slice_z[]; // List of slice plane z-values
};

layout(std430, binding = 2) buffer OutputSegments {
    float segments[]; // Output segments as (x1, y1, x2, y2, slice_index)
};

// Atomic counter for output segments
layout(binding = 3, offset = 0) uniform atomic_uint segment_count;

void main() {
    uint idx = gl_GlobalInvocationID.x;

    // Each triangle has 3 vertices (x, y, z)
    uint tri_idx = idx * 9u; // 3 vertices * 3 components

    if (tri_idx + 8u >= vertices.length())
        return; // Out of bounds

    // Read triangle vertices
    vec3 v0 = vec3(vertices[tri_idx], vertices[tri_idx + 1u], vertices[tri_idx + 2u]);
    vec3 v1 = vec3(vertices[tri_idx + 3u], vertices[tri_idx + 4u], vertices[tri_idx + 5u]);
    vec3 v2 = vec3(vertices[tri_idx + 6u], vertices[tri_idx + 7u], vertices[tri_idx + 8u]);

    // Iterate over slice planes
    for(uint s = 0u; s < slice_z.length(); s++) {
        float z = slice_z[s];

        // Compute distances from vertices to the plane
        float d0 = v0.z - z;
        float d1 = v1.z - z;
        float d2 = v2.z - z;

        // Check if the triangle intersects the plane
        bool positive = (d0 >= 0.0) || (d1 >= 0.0) || (d2 >= 0.0);
        bool negative = (d0 <= 0.0) || (d1 <= 0.0) || (d2 <= 0.0);

        if(!(positive && negative))
            continue; // No intersection

        // Find intersection points
        vec3 p[2];
        int count = 0;

        // Edge v0-v1
        if((d0 > 0.0 && d1 <= 0.0) || (d0 <= 0.0 && d1 > 0.0)) {
            float t = d0 / (d0 - d1);
            p[count++] = mix(v0, v1, t);
        }

        // Edge v1-v2
        if((d1 > 0.0 && d2 <= 0.0) || (d1 <= 0.0 && d2 > 0.0)) {
            float t = d1 / (d1 - d2);
            p[count++] = mix(v1, v2, t);
        }

        // Edge v2-v0
        if((d2 > 0.0 && d0 <= 0.0) || (d2 <= 0.0 && d0 > 0.0)) {
            float t = d2 / (d2 - d0);
            p[count++] = mix(v2, v0, t);
        }

        if(count == 2) {
            // Project to 2D (x, y)
            vec2 proj1 = p[0].xy;
            vec2 proj2 = p[1].xy;

            // Atomically get the current segment count
            uint current = atomicCounterIncrement(segment_count);

            // Write the segment to the output buffer
            uint out_idx = current * 5u; // Each segment has 5 floats (x1, y1, x2, y2, slice_index)
            if(out_idx + 4u < segments.length()) {
                segments[out_idx] = proj1.x;
                segments[out_idx + 1u] = proj1.y;
                segments[out_idx + 2u] = proj2.x;
                segments[out_idx + 3u] = proj2.y;
                segments[out_idx + 4u] = float(s); // Store slice index as float
            }
        }
    }
}
