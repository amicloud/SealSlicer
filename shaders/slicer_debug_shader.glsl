// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.
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