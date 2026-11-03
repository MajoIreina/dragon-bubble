#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> settings: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> transitions: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    let normal = normalize(pbr.N);
    let light = normalize(vec3<f32>(-0.45, 0.75, 0.65));
    let diffuse = dot(normal, light);
    // Antialiased light bands, preserving the source texture's colour detail.
    // Keep three tonal regions, but soften their boundaries independently of resolution.
    let width = max(fwidth(diffuse), transitions.z);
    let shade = settings.x + (settings.y - settings.x) * smoothstep(transitions.x - width, transitions.x + width, diffuse)
        + (1.0 - settings.y) * smoothstep(transitions.y - width, transitions.y + width, diffuse);
    let facing = abs(dot(normal, normalize(pbr.V)));
    let edge_width = max(fwidth(facing), transitions.w);
    let edge = smoothstep(settings.z - edge_width, settings.z + edge_width, facing);
    let half_vector = normalize(light + normalize(pbr.V));
    let highlight = smoothstep(0.88, 0.99, dot(normal, half_vector)) * settings.w;
    let base = pbr.material.base_color;
    let ink = base.rgb * vec3<f32>(0.38, 0.40, 0.44);
    let colour = mix(ink, base.rgb * shade + vec3<f32>(highlight), edge) * mix(vec3<f32>(0.82), vec3<f32>(1.0), pbr.diffuse_occlusion);
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr, vec4<f32>(colour, base.a));
    return out;
}
