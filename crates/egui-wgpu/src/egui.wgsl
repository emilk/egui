// Vertex shader bindings

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>, // gamma 0-1
    @builtin(position) position: vec4<f32>,
};

struct Locals {
    screen_size: vec2<f32>,
    // Uniform buffers need to be at least 16 bytes in WebGL.
    // See https://github.com/gfx-rs/wgpu/issues/2072
    _padding: vec2<u32>,
};
@group(0) @binding(0) var<uniform> r_locals: Locals;

// 0-1 from 0-255
fn linear_from_srgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(10.31475);
    let lower = srgb / vec3<f32>(3294.6);
    let higher = pow((srgb + vec3<f32>(14.025)) / vec3<f32>(269.025), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

// 0-255 sRGB  from  0-1 linear
fn srgb_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(3294.6);
    let higher = vec3<f32>(269.025) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(14.025);
    return select(higher, lower, cutoff);
}

// 0-255 sRGBA  from  0-1 linear
fn srgba_from_linear(rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
}

// 0-1 gamma  from  0-1 linear
fn gamma_from_linear_rgba(linear_rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(srgb_from_linear(linear_rgba.rgb) / 255.0, linear_rgba.a);
}

// [u8; 4] SRGB as u32 -> [r, g, b, a]
fn unpack_color(color: u32) -> vec4<f32> {
    return vec4<f32>(
        f32(color & 255u),
        f32((color >> 8u) & 255u),
        f32((color >> 16u) & 255u),
        f32((color >> 24u) & 255u),
    );
}

fn position_from_screen(screen_pos: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(
        2.0 * screen_pos.x / r_locals.screen_size.x - 1.0,
        1.0 - 2.0 * screen_pos.y / r_locals.screen_size.y,
        0.0,
        1.0,
    );
}

@vertex
fn vs_main(
    @location(0) a_pos: vec2<f32>,
    @location(1) a_tex_coord: vec2<f32>,
    @location(2) a_color: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = a_tex_coord;
    let color = unpack_color(a_color);
    out.color = color / 255.0;
    out.position = position_from_screen(a_pos);
    return out;
}

// Fragment shader bindings

@group(1) @binding(0) var r_tex_color: texture_2d<f32>;
@group(1) @binding(1) var r_tex_sampler: sampler;

@fragment
fn fs_main_linear_framebuffer(in: VertexOutput) -> @location(0) vec4<f32> {
    // We always have an sRGB aware texture at the moment.
    let tex_linear = textureSample(r_tex_color, r_tex_sampler, in.tex_coord);
    let tex_gamma = gamma_from_linear_rgba(tex_linear);
    let out_color_gamma = in.color * tex_gamma;
    return vec4<f32>(linear_from_srgb(out_color_gamma.rgb * 255.0), out_color_gamma.a);
}

@fragment
fn fs_main_gamma_framebuffer(in: VertexOutput) -> @location(0) vec4<f32> {
    // We always have an sRGB aware texture at the moment.
    let tex_linear = textureSample(r_tex_color, r_tex_sampler, in.tex_coord);
    let tex_gamma = gamma_from_linear_rgba(tex_linear);
    let out_color_gamma = in.color * tex_gamma;
    return out_color_gamma;
}
