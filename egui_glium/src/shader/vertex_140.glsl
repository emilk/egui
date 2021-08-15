#version 140

uniform vec2 u_screen_size;
in vec2 a_pos;
in vec2 a_tc;
in vec4 a_rgba; // 0-1
out vec4 v_rgba; // 0-1
out vec2 v_tc;

void main() {
  gl_Position = vec4(
                     2.0 * a_pos.x / u_screen_size.x - 1.0,
                     1.0 - 2.0 * a_pos.y / u_screen_size.y,
                     0.0,
                     1.0);
  v_rgba = a_rgba;
  v_tc = a_tc;
}
