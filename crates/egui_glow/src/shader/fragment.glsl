#ifdef GL_ES
    // To avoid weird distortion issues when rendering text etc, we want highp if possible.
    // But apparently some devices don't support it, so we have to check first.
    #if defined(GL_FRAGMENT_PRECISION_HIGH) && GL_FRAGMENT_PRECISION_HIGH == 1
        precision highp float;
    #else
        precision mediump float;
    #endif
#endif

uniform sampler2D u_sampler;

#if NEW_SHADER_INTERFACE
    in vec4 v_rgba_in_gamma;
    in vec2 v_tc;
    out vec4 f_color;
    // a dirty hack applied to support webGL2
    #define gl_FragColor f_color
    #define texture2D texture
#else
    varying vec4 v_rgba_in_gamma;
    varying vec2 v_tc;
#endif

// -----------------------------------------------
// Adapted from
// https://www.shadertoy.com/view/llVGzG
// Originally presented in:
// Jimenez 2014, "Next Generation Post-Processing in Call of Duty"
//
// A good overview can be found in
// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence/
// via https://github.com/rerun-io/rerun/
float interleaved_gradient_noise(vec2 n) {
    float f = 0.06711056 * n.x + 0.00583715 * n.y;
    return fract(52.9829189 * fract(f));
}

vec3 dither_interleaved(vec3 rgb, float levels) {
    float noise = interleaved_gradient_noise(gl_FragCoord.xy);
    // scale down the noise slightly to ensure flat colors aren't getting dithered
    noise = (noise - 0.5) * 0.95;
    return rgb + noise / (levels - 1.0);
}

void main() {
    vec4 texture_in_gamma = texture2D(u_sampler, v_tc);

    // We multiply the colors in gamma space, because that's the only way to get text to look right.
    vec4 frag_color_gamma = v_rgba_in_gamma * texture_in_gamma;

    // Dither the float color down to eight bits to reduce banding.
    // This step is optional for egui backends.
#if DITHERING
    frag_color_gamma.rgb = dither_interleaved(frag_color_gamma.rgb, 256.);
#endif
    gl_FragColor = frag_color_gamma;
}
