#version 300 es

precision mediump float;
uniform sampler2D u_sampler;
varying vec4 v_rgba;
varying vec2 v_tc;

// 0-255 sRGB  from  0-1 linear
vec3 srgb_from_linear(vec3 rgb) {
    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
    vec3 lower = rgb * vec3(3294.6);
    vec3 higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
    return mix(higher, lower, vec3(cutoff));
}

vec4 srgba_from_linear(vec4 rgba) {
    return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

void main() {
    // The texture is set up with `SRGB8_ALPHA8`, so no need to decode here!
    vec4 texture_rgba = texture2D(u_sampler, v_tc);

    /// Multiply vertex color with texture color (in linear space).
    gl_FragColor = v_rgba * texture_rgba;

    // We must gamma-encode again since WebGL doesn't support linear blending in the framebuffer.
    gl_FragColor = srgba_from_linear(v_rgba * texture_rgba) / 255.0;

    // WebGL doesn't support linear blending in the framebuffer,
    // so we apply this hack to at least get a bit closer to the desired blending:
    gl_FragColor.a = pow(gl_FragColor.a, 1.6); // Empiric nonsense
}
