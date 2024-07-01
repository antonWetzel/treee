struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(1) position: vec3<f32>,
}

// struct PropertyInput {
//     @location(4) value: u32,
// }

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // @location(0) @interpolate(flat) value: u32,
}


@vertex
fn vs_main(
    instance_in: InstanceInput,
    // property_in: PropertyInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.proj * camera.view * vec4<f32>(instance_in.position, 1.0);
    // out.value = property_in.value;
    return out;
}

// struct LookupUniform {
//     mult: u32,
//     shift: u32,
// };

// @group(1) @binding(0)
// var lookup: texture_1d<f32>;
// @group(1) @binding(1)
// var<uniform> lookup_uniform: LookupUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // let idx = (in.value * lookup_uniform.mult) >> lookup_uniform.shift;
    // return textureLoad(lookup, idx, 0);
    return vec4(0.0, 0.0, 0.0, 1.0);
}
