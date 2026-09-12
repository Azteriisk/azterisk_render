struct Camera2DUniform {
    view_proj: mat4x4<f32>,
};

struct Model2DUniform {
    model_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera2DUniform;
@group(0) @binding(1) var<uniform> model: Model2DUniform;

@group(1) @binding(0) var font_texture: texture_2d<f32>;
@group(1) @binding(1) var font_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model_matrix * vec4<f32>(in.position.xy, 0.0, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < 0.0 {
        return in.color;
    }
    let alpha = textureSample(font_texture, font_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
