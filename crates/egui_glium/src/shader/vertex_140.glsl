#version 140

uniform vec2 u_screen_size;
in vec2 a_pos;
in vec4 a_srgba; // 0-255 sRGB
in vec2 a_tc;
out vec4 v_rgba_gamma;
out vec2 v_tc;

void main() {
    gl_Position = vec4(
                      2.0 * a_pos.x / u_screen_size.x - 1.0,
                      1.0 - 2.0 * a_pos.y / u_screen_size.y,
                      0.0,
                      1.0);
    v_rgba_gamma = a_srgba / 255.0;
    v_tc = a_tc;
}
