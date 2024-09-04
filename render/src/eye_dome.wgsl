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

struct Settings {
    color: vec3<f32>,
    strength: f32,
};

@group(0) @binding(0)
var depths: texture_2d<f32>;
@group(0) @binding(1)
var depth_sampler: sampler;

@group(1) @binding(0)
var<uniform> settings: Settings;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {


    let size = textureDimensions(depths);
    let delta = 1.0 / vec2<f32>(size);
    let coord = in.tex_coords;
    let depth = get_depth(coord);

    if depth == 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    var m = depth;
    m = max(m, get_depth(coord + vec2<f32>(-delta.x, 0)));
    m = max(m, get_depth(coord + vec2<f32>(delta.x, 0)));
    m = max(m, get_depth(coord + vec2<f32>(0, -delta.y)));
    m = max(m, get_depth(coord + vec2<f32>(0, delta.y)));
    m = min(m, depth + settings.strength);
    return vec4<f32>(settings.color, (m - depth) / settings.strength);
}


fn get_depth(uv: vec2<f32>) -> f32 {
    let near = 0.1;
    let far = 10000.0;

    let depth = textureSample(depths, depth_sampler, uv).x;
    if depth >= 1.0{
        return 1.0;
    }
    return (2.0 * near) / (far + near - depth * (far - near));
}
