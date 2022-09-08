precision mediump float;
uniform sampler2D u_sampler;
varying vec2 v_tc;

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
    gl_FragColor = texture2D(u_sampler, v_tc);

    gl_FragColor = srgba_from_linear(gl_FragColor) / 255.0;

    #ifdef APPLY_BRIGHTENING_GAMMA
        gl_FragColor = vec4(pow(gl_FragColor.rgb, vec3(1.0/2.2)), gl_FragColor.a);
    #endif
}
