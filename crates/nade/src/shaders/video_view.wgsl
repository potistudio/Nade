struct ViewUniforms {
    viewport_size: vec2<f32>,
    frame_size: vec2<f32>,
    offset: vec2<f32>,
    zoom: f32,
    _pad: f32,
    background: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> view: ViewUniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen = in.uv * view.viewport_size;

    let fit = min(
        view.viewport_size.x / max(view.frame_size.x, 1.0),
        view.viewport_size.y / max(view.frame_size.y, 1.0),
    );
    let display_scale = fit * view.zoom;
    let display_size = view.frame_size * display_scale;
    let origin = (view.viewport_size - display_size) * 0.5 + view.offset;

    let local = (screen - origin) / display_scale;
    let tex_uv = local / view.frame_size;

    if (tex_uv.x < 0.0 || tex_uv.x > 1.0 || tex_uv.y < 0.0 || tex_uv.y > 1.0) {
        return view.background;
    }

    return textureSample(t_diffuse, s_diffuse, tex_uv);
}
