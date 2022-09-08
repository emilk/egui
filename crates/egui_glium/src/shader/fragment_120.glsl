#version 120

uniform sampler2D u_sampler;
varying vec4 v_rgba;
varying vec2 v_tc;

void main() {
    // The texture sampler is sRGB aware, and glium already expects linear rgba output
    // so no need for any sRGB conversions here:
    gl_FragColor = v_rgba * texture2D(u_sampler, v_tc);
}
