struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct Environment {
    scale: f32,
    min: u32,
    max: u32,
    padding: u32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> environment: Environment;

struct VertexInput {
    @location(0) position: vec2<f32>,
}
struct InstanceInput {
    @location(1) position: vec3<f32>,
}

struct PropertyInput {
    @location(4) value: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) value: u32,
    @location(1) pos: vec2<f32>,
}


@vertex
fn vs_main(
    vertex_in: VertexInput,
    instance_in: InstanceInput,
    property_in: PropertyInput,
) -> VertexOutput {
    var out: VertexOutput;
    if  property_in.value < environment.min || environment.max < property_in.value {
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.value = 0u;
        out.pos = vec2<f32>(0.0, 0.0);
        return out;
    }

    var pos = camera.view * vec4<f32>(instance_in.position, 1.0);
    pos.x += vertex_in.position.x * environment.scale;
    pos.y += vertex_in.position.y * environment.scale;

    out.clip_position = camera.proj * pos;
    out.value = property_in.value;
    out.pos = vertex_in.position;
    return out;
}

struct LookupUniform {
    mult: u32,
    shift: u32,
    _padding: vec2<u32>,
};

@group(2) @binding(0)
var lookup: texture_1d<f32>;
@group(2) @binding(1)
var<uniform> lookup_uniform: LookupUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.pos.x * in.pos.x + in.pos.y * in.pos.y >= 1.0 {
        discard;
    }
    // return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    let idx = (in.value * lookup_uniform.mult) >> lookup_uniform.shift;
    return textureLoad(lookup, idx, 0);
}
