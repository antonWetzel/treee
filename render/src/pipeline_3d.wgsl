struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
}
struct InstanceInput {
    @location(1) position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec3<f32>,
    @location(4) size: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) pos: vec2<f32>,
}

@vertex
fn vs_main(
    vertex_in: VertexInput,
    instance_in: InstanceInput,
) -> VertexOutput {
    let a = normalize(cross(instance_in.normal, vec3<f32>(instance_in.normal.y, instance_in.normal.z, -instance_in.normal.x)));
    let b = cross(instance_in.normal, a);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(
        instance_in.position +
            vertex_in.position.x * instance_in.size * a +
            vertex_in.position.y * instance_in.size * b,
        1.0,
    );
    out.color = instance_in.color;
    out.pos = vertex_in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.pos.x * in.pos.x + in.pos.y * in.pos.y >= 1.0 {
        discard;
    }
    return vec4<f32>(in.color, 1.0);
}
