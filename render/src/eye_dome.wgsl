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
    sensitivity: f32,
    strength: f32,
};

@group(0) @binding(0)
var depths: texture_2d<f32>;
@group(1) @binding(0)
var<uniform> settings: Settings;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(depths);
    let coord = vec2<i32>(in.tex_coords * vec2<f32>(size));
    let depth = get_depth(coord);
    // let color = textureLoad(t_before, coord, 0).xyz;
    if depth == 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    var m = depth;
    m = max(m, get_depth(coord + vec2<i32>(1, 1)));
    m = max(m, get_depth(coord + vec2<i32>(1, -1)));
    m = max(m, get_depth(coord + vec2<i32>(-1, -1)));
    m = max(m, get_depth(coord + vec2<i32>(-1, 1)));
    m = min(m, depth + settings.sensitivity);
    // return vec4<f32>(color * exp((depth - m) / 0.03), 1.0);
    let amount = (m - depth) / settings.sensitivity;
    return vec4<f32>(settings.color, 1.0 - exp(-amount * settings.strength));
}


fn get_depth(uv: vec2<i32>) -> f32 {
    let near = 0.1;
    let far = 1000.0;
    let depth = textureLoad(depths, uv, 0).x;
    if depth >= 1.0{
        return 0.0;
    }
    return (2.0 * near) / (far + near - depth * (far - near));
}
