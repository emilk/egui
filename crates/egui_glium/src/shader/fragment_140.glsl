#version 140

uniform sampler2D u_sampler;
in vec4 v_rgba;
in vec2 v_tc;
out vec4 f_color;

void main() {
    // The texture sampler is sRGB aware, and glium already expects linear rgba output
    // so no need for any sRGB conversions here:
    f_color = v_rgba * texture(u_sampler, v_tc);
}
