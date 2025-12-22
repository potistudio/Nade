struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Generate a full screen quad from 6 vertices (2 triangles)
    // Vertices:
    // 0: (-1, -1), UV (0, 1)
    // 1: ( 1, -1), UV (1, 1)
    // 2: (-1,  1), UV (0, 0)
    // 3: (-1,  1), UV (0, 0)
    // 4: ( 1, -1), UV (1, 1)
    // 5: ( 1,  1), UV (1, 0)

    // Using a simpler approach with 3 large triangle or just array lookup
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    out.position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
    out.uv = uvs[in_vertex_index];
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
