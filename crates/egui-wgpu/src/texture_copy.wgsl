struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

var<private> positions: array<vec2f, 3> = array<vec2f, 3>(
    vec2f(-1.0, -3.0),
    vec2f(-1.0, 1.0),
    vec2f(3.0, 1.0)
);

// meant to be called with 3 vertex indices: 0, 1, 2
// draws one large triangle over the clip space like this:
// (the asterisks represent the clip space bounds)
//-1,1           1,1
// ---------------------------------
// |              *              .
// |              *           .
// |              *        .
// |              *      .
// |              *    .
// |              * .
// |***************
// |            . 1,-1
// |          .
// |       .
// |     .
// |   .
// |.
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;
    result.position = vec4f(positions[vertex_index], 0.0, 1.0);
    return result;
}

@group(0)
@binding(0)
var r_color: texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return textureLoad(r_color, vec2i(vertex.position.xy), 0);
}
