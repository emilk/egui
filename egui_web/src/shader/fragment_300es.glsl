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

// 0-255 sRGBA  from  0-1 linear
vec4 srgba_from_linear(vec4 rgba) {
  return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

void main() {
  // The texture is set up with `SRGB8_ALPHA8`, so no need to decode here!
  vec4 texture_rgba = texture2D(u_sampler, v_tc);

  /// Multiply vertex color with texture color (in linear space).
  gl_FragColor = v_rgba * texture_rgba;

  // WebGL doesn't support linear blending in the framebuffer,
  // so we do a hack here where we change the premultiplied alpha
  // to do the multiplication in gamma space instead:

  // Unmultiply alpha:
  if (gl_FragColor.a > 0.0) {
    gl_FragColor.rgb /= gl_FragColor.a;
  }

  // Empiric tweak to make e.g. shadows look more like they should:
  gl_FragColor.a *= sqrt(gl_FragColor.a);

  // To gamma:
  gl_FragColor = srgba_from_linear(gl_FragColor) / 255.0;

  // Premultiply alpha, this time in gamma space:
  if (gl_FragColor.a > 0.0) {
    gl_FragColor.rgb *= gl_FragColor.a;
  }
}
