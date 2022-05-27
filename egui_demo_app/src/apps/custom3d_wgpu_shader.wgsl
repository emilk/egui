struct VertexOut {
    [[location(0)]] color: vec4<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

struct Uniforms {
    angle: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

var<private> v_positions: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, -1.0),
);

var<private> v_colors: array<vec4<f32>, 3> = array<vec4<f32>, 3>(
    vec4<f32>(1.0, 0.0, 0.0, 1.0),
    vec4<f32>(0.0, 1.0, 0.0, 1.0),
    vec4<f32>(0.0, 0.0, 1.0, 1.0),
);

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] v_idx: u32) -> VertexOut {
    var out: VertexOut;

    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    out.position.x = out.position.x * cos(uniforms.angle);
    out.color = v_colors[v_idx];

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOut) -> [[location(0)]] vec4<f32> {
    return in.color;
}
