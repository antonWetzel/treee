struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_shadow: texture_2d<f32>;
@group(0) @binding(1)
var t_before: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(t_shadow);
    let coord = vec2<i32>(in.tex_coords * vec2<f32>(size));
    let depth = get_depth(coord);
    let color = textureLoad(t_before, coord, 0).xyz;
    if depth == 0.0 {
        return vec4<f32>(color, 1.0);
    }
    var m = depth;
    m = max(m, get_depth(coord + vec2<i32>(1, 1)));
    m = max(m, get_depth(coord + vec2<i32>(1, -1)));
    m = max(m, get_depth(coord + vec2<i32>(-1, -1)));
    m = max(m, get_depth(coord + vec2<i32>(-1, 1)));
    m = min(m, depth + 0.03);
    return vec4<f32>(color * exp((depth - m) / 0.03), 1.0);
}


fn get_depth(uv: vec2<i32>) -> f32 {
    let near = 0.1;
    let far = 100.0;
    let depth = textureLoad(t_shadow, uv, 0).x;
    if depth >= 1.0{
        return 0.0;
    }
    return (2.0 * near) / (far + near - depth * (far - near));
}
