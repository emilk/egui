#ifdef GL_ES
    precision mediump float;
#endif

uniform sampler2D u_sampler;

#ifdef NEW_SHADER_INTERFACE
    in vec4 v_rgba_gamma;  // 0-1 RGBA gamma
    in vec2 v_tc;
    out vec4 f_color;
    // a dirty hack applied to support webGL2
    #define gl_FragColor f_color
    #define texture2D texture
#else
    varying vec4 v_rgba_gamma;  // 0-1 RGBA gamma
    varying vec2 v_tc;
#endif

// 0-255 sRGB  from  0-1 linear
vec3 srgb_from_linear(vec3 rgb) {
    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
    vec3 lower = rgb * vec3(3294.6);
    vec3 higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
    return mix(higher, lower, vec3(cutoff));
}

// 0-255 sRGBA  from  0-1 linear
vec4 srgba_from_linear(vec4 rgba) {
    return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

void main() {
    vec4 tex_color_linear = texture2D(u_sampler, v_tc);
    vec4 tex_color_gamma = srgba_from_linear(tex_color_linear) / 255.0;
    gl_FragColor = v_rgba_gamma * tex_color_gamma;
}
