#import bevy_sprite::mesh2d_vertex_output::VertexOutput
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var image: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var image_sampler: sampler;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(image));
    let footprint = max(abs(dpdx(in.uv)) + abs(dpdy(in.uv)), vec2(0.000001)) * dimensions;
    if max(footprint.x, footprint.y) <= 1.0 {
        return textureSampleLevel(image, image_sampler, in.uv, 0.0);
    }
    // Integrate the source texels covered by this output pixel. Single bilinear
    // taps skip detail at high resolutions; repeated resizes soften contours.
    let center = in.uv * dimensions;
    let lo = center - footprint * 0.5;
    let hi = center + footprint * 0.5;
    var sum = vec4(0.0);
    var weight = 0.0;
    for (var y = i32(floor(lo.y)); y < i32(ceil(hi.y)); y++) {
        for (var x = i32(floor(lo.x)); x < i32(ceil(hi.x)); x++) {
            let p = vec2<f32>(f32(x), f32(y));
            let overlap = max(min(hi, p+1.0)-max(lo,p),vec2(0.0));
            let w = overlap.x * overlap.y;
            let coord = clamp(vec2<i32>(x,y),vec2(0),vec2<i32>(textureDimensions(image))-1);
            sum += textureLoad(image,coord,0)*w;
            weight += w;
        }
    }
    return sum / max(weight,0.000001);
}
