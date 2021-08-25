precision mediump float;
uniform sampler2D u_sampler;
varying vec4 v_rgba;
varying vec2 v_tc;


// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
  bvec3 cutoff = lessThan(srgb, vec3(10.31475));
  vec3 lower = srgb / vec3(3294.6);
  vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
  return mix(higher, lower, vec3(cutoff));
}

// 0-1 linear  from  0-255 sRGBA
vec4 linear_from_srgba(vec4 srgba) {
  return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}

void main() {
  // We must decode the colors, since WebGL doesn't come with sRGBA textures:
  vec4 texture_rgba = linear_from_srgba(texture2D(u_sampler, v_tc) * 255.0);

  /// Multiply vertex color with texture color (in linear space).
  gl_FragColor = v_rgba * texture_rgba;
}
