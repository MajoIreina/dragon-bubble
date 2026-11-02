// RGBA coverage resolve; adapted from Bevy TAA (MIT OR Apache-2.0).
// Catmull-Rom reconstruction: MJP; YCoCg clipping: Playdead (MIT).
@group(0) @binding(0) var view_target: texture_2d<f32>;
@group(0) @binding(1) var history: texture_2d<f32>;
@group(0) @binding(2) var motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var depth: texture_depth_2d;
@group(0) @binding(4) var nearest_sampler: sampler;
@group(0) @binding(5) var linear_sampler: sampler;

struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};
fn to_ycocg(c: vec4<f32>) -> vec4<f32> {
    return vec4(dot(c.rgb, vec3(0.25, 0.5, 0.25)), (c.r-c.b)*0.5, dot(c.rgb, vec3(-0.25,0.5,-0.25)), c.a);
}
fn from_ycocg(c: vec4<f32>) -> vec4<f32> {
    return vec4(max(vec3(c.x+c.y-c.z, c.x+c.z, c.x-c.y-c.z), vec3(0.0)), saturate(c.a));
}
fn sample_history(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(history, linear_sampler, uv, 0.0);
}
fn sample_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(depth));
    return textureLoad(depth, clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), size-1), 0);
}
@fragment
fn taa(@location(0) uv: vec2<f32>) -> Output {
    let size = vec2<f32>(textureDimensions(view_target));
    let texel = 1.0 / size;
    var current = textureSampleLevel(view_target, nearest_sampler, uv, 0.0);
#ifndef RESET
    // Dilate foreground motion by one pixel at silhouettes (reverse-Z).
    var closest_uv = uv;
    var closest_depth = sample_depth(uv);
    var mean = vec4(0.0);
    var moment2 = vec4(0.0);
    var low = vec4(1e10);
    var high = vec4(-1e10);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let sample_uv = uv + vec2<f32>(f32(x),f32(y))*texel;
            let d = sample_depth(sample_uv);
            if d > closest_depth { closest_uv = sample_uv; closest_depth = d; }
            let c = to_ycocg(textureSampleLevel(view_target, nearest_sampler, sample_uv, 0.0));
            mean += c;
            moment2 += c*c;
            low = min(low,c);
            high = max(high,c);
        }
    }
    let velocity = textureSampleLevel(motion_vectors, nearest_sampler, closest_uv, 0.0).rg;
    let history_uv = uv - velocity;
    // Five-tap Catmull-Rom preserves detail without an extra sharpening pass.
    let position = history_uv * size;
    let center = floor(position - 0.5) + 0.5;
    let f = position - center;
    let w0 = f*(-0.5+f*(1.0-0.5*f));
    let w1 = 1.0+f*f*(-2.5+1.5*f);
    let w2 = f*(0.5+f*(2.0-1.5*f));
    let w3 = f*f*(-0.5+0.5*f);
    let w12 = w1+w2;
    let p0 = (center-1.0)*texel;
    let p3 = (center+2.0)*texel;
    let p12 = (center+w2/w12)*texel;
    var old = sample_history(vec2(p12.x,p0.y))*w12.x*w0.y;
    old += sample_history(vec2(p0.x,p12.y))*w0.x*w12.y;
    old += sample_history(p12)*w12.x*w12.y;
    old += sample_history(vec2(p3.x,p12.y))*w3.x*w12.y;
    old += sample_history(vec2(p12.x,p3.y))*w12.x*w3.y;
    let weight = w12.x*w0.y+w0.x*w12.y+w12.x*w12.y+w3.x*w12.y+w12.x*w3.y;
    old /= weight;
    mean /= 9.0;
    let deviation = sqrt(max(moment2/9.0-mean*mean,vec4(0.0)));
    var lower = max(low,mean-deviation*1.25);
    var upper = min(high,mean+deviation*1.25);
    // Coverage needs the full neighborhood range to converge at thin contours.
    lower.a = low.a;
    upper.a = high.a;
    let clipped = clamp(to_ycocg(old),lower,upper);
    let motion_pixels = length(velocity*size);
    let disagreement = abs(to_ycocg(current).x-clipped.x);
    var blend = clamp(0.125+motion_pixels*0.08+disagreement*0.4,0.125,0.8);
    if any(history_uv < vec2(0.0)) || any(history_uv > vec2(1.0)) {
        blend = 1.0;
    }
    current = mix(from_ycocg(clipped),current,blend);
    // Newly uncovered empty neighborhoods must immediately become transparent.
    if high.a < 0.0001 { current = vec4(0.0); }
#endif
    current.a = saturate(current.a);
    current = vec4(clamp(current.rgb,vec3(0.0),vec3(current.a)),current.a);
    var out: Output;
    out.history = current;
    out.view_target = current;
    return out;
}
