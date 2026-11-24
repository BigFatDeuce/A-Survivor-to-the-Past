// Filtres video : l'image du jeu (texture intermediaire) vers l'ecran.
// Les effets suivent la grille des pixels du jeu (`post.scale` pixels ecran par pixel du jeu).
struct Post { region: vec4<f32>, scale: f32, mode: u32, _p: vec2<f32> };
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> post: Post;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

fn texel(p: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(post.region.zw);
    let t = clamp(vec2<i32>(floor(p)), vec2<i32>(0), size - vec2<i32>(1));
    return textureLoad(scene, t, 0).rgb;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// profil d'une ligne de balayage : 1 au centre d'une ligne du jeu, plus sombre entre deux lignes
fn scanline(p: vec2<f32>, k: f32) -> f32 {
    let fy = fract(p.y / post.scale);
    let w = 0.5 + 0.5 * cos(6.2831853 * (fy - 0.5));
    // pas de lignes quand un pixel du jeu fait 1 pixel ecran (moirage)
    let kk = k * clamp((post.scale - 1.0) * 0.5, 0.0, 1.0);
    return mix(1.0 - kk, 1.0, w);
}

// grille d'ecran LCD : liseré sombre au bord de chaque pixel du jeu
fn lcd_grid(p: vec2<f32>, dark: f32) -> f32 {
    if (post.scale < 2.0) {
        return 1.0;
    }
    let f = fract(p / post.scale) * post.scale;
    let edge = f.x < 1.0 || f.y < 1.0;
    let d = select(dark, mix(1.0, dark, 0.5), post.scale < 3.0);
    return select(1.0, d, edge);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

fn bayer4(c: vec2<i32>) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let i = (c.y & 3) * 4 + (c.x & 3);
    return (m[i] + 0.5) / 16.0;
}

@fragment
fn fs(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
    let p = fc.xy - post.region.xy;
    let S = post.scale;
    var c = texel(p);
    switch post.mode {
        case 1u: {
            // composite : etalement horizontal d'un demi-pixel du jeu, legere bavure, lignes discretes
            let o = vec2<f32>(max(1.0, S * 0.5), 0.0);
            c = c * 0.5 + texel(p - o) * 0.25 + texel(p + o) * 0.25;
            let d = vec2<f32>(max(1.0, S * 0.34), 0.0);
            let r = texel(p - d).r;
            let b = texel(p + d).b;
            c = vec3<f32>(mix(c.r, r, 0.3), c.g, mix(c.b, b, 0.3));
            c = mix(vec3<f32>(luma(c)), c, 1.08);
            c = c * scanline(p, 0.22) * 1.08;
        }
        case 2u: {
            // televiseur cathodique
            let size = post.region.zw;
            var uv = p / size * 2.0 - 1.0;
            uv = uv * (1.0 + vec2<f32>(0.045, 0.06) * uv.yx * uv.yx);
            let border = smoothstep(1.0, 0.985, abs(uv.x)) * smoothstep(1.0, 0.98, abs(uv.y));
            if (abs(uv.x) >= 1.0 || abs(uv.y) >= 1.0) {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }
            let q = (uv * 0.5 + 0.5) * size;
            let o = vec2<f32>(max(1.0, S * 0.6), 0.0);
            c = texel(q) * 0.5 + texel(q - o) * 0.25 + texel(q + o) * 0.25;
            // halo des zones claires
            let o2 = vec2<f32>(max(2.0, S * 1.5), 0.0);
            let glow = (texel(q - o2) + texel(q + o2) + texel(q - o2.yx) + texel(q + o2.yx)) * 0.25;
            c = c + max(glow - vec3<f32>(0.35), vec3<f32>(0.0)) * 0.35;
            // ligne plus epaisse quand le point est lumineux
            let k = mix(0.5, 0.25, clamp(luma(c) * 1.5, 0.0, 1.0));
            c = c * scanline(q, k);
            if (S >= 3.0) {
                let m = i32(fc.x) % 3;
                var mask = vec3<f32>(0.82);
                if (m == 0) {
                    mask.r = 1.0;
                } else if (m == 1) {
                    mask.g = 1.0;
                } else {
                    mask.b = 1.0;
                }
                c = c * mask;
            }
            let v = 1.0 - 0.11 * dot(uv, uv);
            c = c * v * 1.3 * border;
        }
        case 3u: {
            c = c * scanline(p, 0.38) * 1.18;
        }
        case 4u: {
            // LCD 32 bits sans retroeclairage : couleurs ternies, grille de pixels
            let m = mat3x3<f32>(
                vec3<f32>(0.82, 0.24, -0.06),
                vec3<f32>(0.125, 0.665, 0.21),
                vec3<f32>(0.195, 0.075, 0.73),
            );
            c = clamp(m * c * 0.94, vec3<f32>(0.0), vec3<f32>(1.0));
            c = pow(c, vec3<f32>(1.08)) * lcd_grid(p, 0.72);
        }
        case 5u: {
            // quatre tons de vert, trame ordonnee a l'echelle des pixels du jeu
            var l = pow(clamp(luma(c), 0.0, 1.0), 1.0 / 2.2);
            l = clamp((l - 0.06) * 1.2, 0.0, 1.0);
            let cell = vec2<i32>(floor(p / S));
            let level = clamp(floor(l * 3.0 + bayer4(cell) - 0.25), 0.0, 3.0);
            var pal = array<vec3<f32>, 4>(
                vec3<f32>(0.059, 0.220, 0.059),
                vec3<f32>(0.188, 0.384, 0.188),
                vec3<f32>(0.545, 0.675, 0.059),
                vec3<f32>(0.608, 0.737, 0.059),
            );
            c = srgb_to_linear(pal[i32(level)]) * lcd_grid(p, 0.85);
        }
        default: {}
    }
    return vec4<f32>(c, 1.0);
}
