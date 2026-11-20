struct TimeUniform {
    time: f32,
}

@group(0) @binding(0)
var<uniform> time_uniform: TimeUniform;

struct Vertex {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv_position: vec2<f32>,
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.uv_position = (vertex.position + vec2<f32>(1.0, 1.0)) / 2.0;
    return out;
}


// Fundo dos menus — direção "sala de concerto": base escura (#0F0E13) com um
// brilho violeta-quente muito suave no canto superior direito, que "respira"
// devagar. Substitui as riscas diagonais. (Não é usado no ecrã de tocar.)

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv_position;
    let base = vec3<f32>(0.0055, 0.0050, 0.0065); // ~#0F0E13 em linear

    // Brilho principal (canto superior direito)
    let breathe = 0.85 + 0.15 * sin(time_uniform.time * 0.25);
    let c1 = vec2<f32>(0.78, 1.08);
    let d1 = distance(uv * vec2<f32>(1.6, 1.0), c1 * vec2<f32>(1.6, 1.0));
    let glow1 = exp(-d1 * d1 * 2.2) * 0.030 * breathe;
    let tint1 = vec3<f32>(0.55, 0.38, 0.85); // violeta

    // Brilho secundário quente, em baixo à esquerda, ainda mais discreto
    let c2 = vec2<f32>(0.08, -0.15);
    let d2 = distance(uv * vec2<f32>(1.6, 1.0), c2 * vec2<f32>(1.6, 1.0));
    let glow2 = exp(-d2 * d2 * 3.0) * 0.012 * (1.15 - breathe * 0.3);
    let tint2 = vec3<f32>(0.95, 0.70, 0.35); // dourado

    let color = base + tint1 * glow1 + tint2 * glow2;
    return vec4<f32>(color, 1.0);
}
