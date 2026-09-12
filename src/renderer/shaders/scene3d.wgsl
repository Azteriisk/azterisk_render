struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct LightUniform {
    position: vec4<f32>,      // xyz, radius
    direction: vec4<f32>,     // xyz, light_type (0=amb, 1=dir, 2=point, 3=spot)
    color: vec4<f32>,         // rgb, intensity
    spot_params: vec4<f32>,   // cos_inner, cos_outer, has_gobo, unused
    light_view_proj: mat4x4<f32>,
};

struct LightsUniform {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    lights: array<LightUniform, 16>,
};

struct ModelUniform {
    model_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> scene_lights: LightsUniform;
@group(0) @binding(2) var<uniform> model: ModelUniform;

@group(1) @binding(0) var gobo_texture: texture_2d<f32>;
@group(1) @binding(1) var gobo_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = model.model_matrix * vec4<f32>(in.position, 1.0);
    out.world_pos = world_pos4.xyz;
    out.clip_position = camera.view_proj * world_pos4;

    // Transform normal
    let normal_matrix = mat3x3<f32>(
        model.model_matrix[0].xyz,
        model.model_matrix[1].xyz,
        model.model_matrix[2].xyz,
    );
    out.normal = normalize(normal_matrix * in.normal);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.normal);

    // Baseline ambient light so objects aren't pitch black
    var ambient = vec3<f32>(0.08, 0.08, 0.10);
    var diffuse_contrib = vec3<f32>(0.0);

    let light_count = min(scene_lights.count, 16u);
    for (var i = 0u; i < light_count; i = i + 1u) {
        let light = scene_lights.lights[i];
        let light_type = u32(light.direction.w);

        let light_pos = light.position.xyz;
        let light_dir = normalize(light.direction.xyz);
        let light_color = light.color.rgb;
        let intensity = light.color.w;

        if (light_type == 0u) {
            // Pure ambient light
            ambient += light_color * intensity;
        } else if (light_type == 1u) {
            // Directional Sun Light
            let L = -light_dir;
            let diff = max(dot(N, L), 0.0);
            diffuse_contrib += light_color * (diff * intensity);
        } else if (light_type == 2u) {
            // Point Light
            let to_light = light_pos - in.world_pos;
            let dist = length(to_light);
            let radius = light.position.w;

            if (dist < radius) {
                let L = normalize(to_light);
                let diff = max(dot(N, L), 0.0);
                let atten = clamp(1.0 - (dist / radius), 0.0, 1.0);
                diffuse_contrib += light_color * (diff * intensity * atten * atten);
            }
        } else if (light_type == 3u) {
            // Spotlight with optional Film Gobo / Cookie Mask
            let to_frag = in.world_pos - light_pos;
            let dist = length(to_frag);
            let radius = light.position.w;

            if (dist < radius) {
                let frag_dir = normalize(to_frag);
                let cos_angle = dot(frag_dir, light_dir);
                let cos_inner = light.spot_params.x;
                let cos_outer = light.spot_params.y;

                if (cos_angle > cos_outer) {
                    // Smooth spot edge falloff
                    let spot_atten = clamp((cos_angle - cos_outer) / max(cos_inner - cos_outer, 0.001), 0.0, 1.0);
                    let dist_atten = clamp(1.0 - (dist / radius), 0.0, 1.0);

                    let L = -frag_dir;
                    let diff = max(dot(N, L), 0.0);

                    // Projective Gobo / Cookie Sampling
                    var gobo_factor = 1.0;
                    if (light.spot_params.z > 0.5) {
                        let light_clip = light.light_view_proj * vec4<f32>(in.world_pos, 1.0);
                        if (light_clip.w > 0.0) {
                            let ndc = light_clip.xy / light_clip.w;
                            let uv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
                            if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
                                let sample_color = textureSample(gobo_texture, gobo_sampler, uv);
                                gobo_factor = (sample_color.r + sample_color.g + sample_color.b) / 3.0 * sample_color.a;
                            } else {
                                gobo_factor = 0.0;
                            }
                        } else {
                            gobo_factor = 0.0;
                        }
                    }

                    diffuse_contrib += light_color * (diff * intensity * dist_atten * spot_atten * gobo_factor);
                }
            }
        }
    }

    let final_rgb = in.color.rgb * (ambient + diffuse_contrib);
    return vec4<f32>(final_rgb, in.color.a);
}
