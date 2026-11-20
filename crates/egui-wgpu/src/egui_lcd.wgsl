// Subpixel (LCD) text, for the font texture only. Appended to egui.wgsl after
// `enable dual_source_blending;`.
//
// A subpixel texel holds one coverage per color channel, with alpha the smallest. A premultiplied
// color never has a channel above its alpha, so that's how they're told from color glyphs (emoji)
// and grayscale coverage, which blend as in `fs_main_*`. Each color channel of a subpixel texel
// blends with its own coverage (dual-source blending).

fn is_subpixel_texel(tex: vec4<f32>) -> bool {
    return max(tex.r, max(tex.g, tex.b)) > tex.a + 0.5 / 255.0;
}

// Subpixel coverage drawn grayscale, at its mean; other texels as they are.
fn gray_texel(tex: vec4<f32>) -> vec4<f32> {
    if is_subpixel_texel(tex) {
        return vec4<f32>((tex.r + tex.g + tex.b) / 3.0);
    }
    return tex;
}

struct LcdOutput {
    @location(0) @blend_src(0) color: vec4<f32>,
    @location(0) @blend_src(1) coverage: vec4<f32>,
};

// Whether this fragment samples the texture one texel per pixel, at texel centers horizontally:
// only then do a texel's three coverages line up with the pixel's color stripes. Scaled,
// rotated or fractionally shifted text (layer transforms, `TextShape::angle`) is drawn
// grayscale instead of smearing its color fringes.
fn texels_on_pixels(in: VertexOutput) -> bool {
    let size = vec2<f32>(textureDimensions(r_tex_color, 0));
    let dx = dpdx(in.tex_coord) * size;
    let dy = dpdy(in.tex_coord) * size;
    let off = abs(dx - vec2<f32>(1.0, 0.0)) + abs(dy - vec2<f32>(0.0, 1.0));
    let center = abs(fract(in.tex_coord.x * size.x) - 0.5);
    return off.x + off.y < 0.01 && center < 0.02;
}

fn lcd_output(in: VertexOutput, linear_framebuffer: bool) -> LcdOutput {
    let sampled = sample_texture(in);
    // Per-channel coverage where it lines up with the pixel's stripes; else as `fs_main_*`.
    // (Derivatives first: they need uniform control flow.)
    let aligned = texels_on_pixels(in);
    let subpixel = is_subpixel_texel(sampled) && aligned;
    var tex = sampled;
    if !subpixel {
        tex = gray_texel(sampled);
    }
    // `in.color` is premultiplied, so its alpha times the coverage weighs the framebuffer.
    var color = vec4<f32>(in.color.rgb * tex.rgb, in.color.a * tex.a);
    var coverage = vec4<f32>(color.a);
    if subpixel {
        color.a = in.color.a * (tex.r + tex.g + tex.b) / 3.0;
        coverage = vec4<f32>(in.color.a * tex.rgb, color.a);
    }
    if r_locals.dithering == 1 {
        color = vec4<f32>(dither_interleaved(color.rgb, 256.0, in.position), color.a);
    }
    if linear_framebuffer {
        color = vec4<f32>(linear_from_gamma_rgb(color.rgb), color.a);
    }
    var out: LcdOutput;
    out.color = color;
    out.coverage = coverage;
    return out;
}

@fragment
fn fs_lcd_linear_framebuffer(in: VertexOutput) -> LcdOutput {
    return lcd_output(in, true);
}

@fragment
fn fs_lcd_gamma_framebuffer(in: VertexOutput) -> LcdOutput {
    return lcd_output(in, false);
}
