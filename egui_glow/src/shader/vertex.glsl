#ifdef NEW_SHADER_INTERFACE
    #define I in
    #define O out
    #define V(x) x
#else
    #define I attribute
    #define O varying
    #define V(x) vec3(x)
#endif

#ifdef GL_ES
    precision mediump float;
#endif

uniform vec2 u_screen_size;
I vec2 a_pos;
I vec4 a_srgba; // 0-255 sRGB
I vec2 a_tc;
O vec4 v_rgba;
O vec2 v_tc;

// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, V(cutoff));
}

vec4 linear_from_srgba(vec4 srgba) {
    return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}

void main() {
    gl_Position = vec4(
                      2.0 * a_pos.x / u_screen_size.x - 1.0,
                      1.0 - 2.0 * a_pos.y / u_screen_size.y,
                      0.0,
                      1.0);
    // egui encodes vertex colors in gamma space, so we must decode the colors here:
    v_rgba = linear_from_srgba(a_srgba);
    v_tc = a_tc;
}
