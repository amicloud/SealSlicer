#version 310 es

layout(local_size_x = 256) in;

layout(std430, binding = 0) buffer OutputData {
    float data[];
};

void main() {
    uint idx = gl_GlobalInvocationID.x;
    if (idx < data.length()) {
        data[idx] = float(idx);
    }
}