#version 450
// LICENSE: MIT (https://github.com/RobinMcCorkell/shaderlock/blob/master/LICENSE)
// AUTHOR:  RobinMcCorkell
// FOUND:   https://github.com/RobinMcCorkell/shaderlock/blob/master/dist/shaders/crt.frag
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

const float BULGE_AMOUNT = 0.2;

const float ROLL_FACTOR = 12.0;
const float ROLL_AMOUNT = 3.0;
const float ROLL_PERIOD = 6.0;

const float ABBERATION_FACTOR = 1.5;
const float ABBERATION_AMOUNT = 0.02;

const float VIGNETTE_SCALE = 2.2;
const float VIGNETTE_FACTOR = 5.0;
const float VIGNETTE_AMOUNT = 0.9;

const float SCAN_LINE_COUNT = 700.0;
const float SCAN_LINE_AMOUNT = 0.25;
const float SCAN_LINE_FACTOR = 2.2;
const float SCAN_LINE_DRIFT = 0.002;

const float DOWNSCALE_RESOLUTION = 300.0;

vec2 downscale(vec2 v) {
    return floor(v * DOWNSCALE_RESOLUTION) / DOWNSCALE_RESOLUTION;
}

float get_center_distance(vec2 p) {
    return distance(p, vec2(0.5)) * 2.0;
}

vec2 bulge_coords(vec2 p, float amount) {
    float d = get_center_distance(p);
    p -= vec2(0.5);
    p *= 1.0 + d * amount;
    p *= 1.0 - amount;
    p += vec2(0.5);
    return p;
}

vec2 roll_coords(vec2 p) {
    float progress = fract(iTime / ROLL_PERIOD);
    float amount = ROLL_AMOUNT * clamp(ROLL_FACTOR * progress, 0.0, 1.0);
    p.y -= amount;
    return fract(p);
}

vec4 sample_abberation(vec2 p_fb, vec2 p) {
    float d = pow(get_center_distance(p), ABBERATION_FACTOR);

    float r = texture(sampler2D(t_screenshot, s_screenshot), downscale(p_fb)).r;
    float g = texture(sampler2D(t_screenshot, s_screenshot), downscale(p_fb * (1.0 + d * ABBERATION_AMOUNT))).g;
    float b = texture(sampler2D(t_screenshot, s_screenshot), downscale(p_fb * (1.0 - d * ABBERATION_AMOUNT))).b;

    return vec4(r, g, b, 1.0);
}

vec4 vignette(vec4 c, vec2 p) {
    float f = VIGNETTE_AMOUNT * min(1.0, pow(get_center_distance(p) / VIGNETTE_SCALE, VIGNETTE_FACTOR));
    c.rgb -= f;
    return c;
}

vec4 scan_lines(vec4 c, vec2 p) {
    p.y += iTime * SCAN_LINE_DRIFT;
    float f = SCAN_LINE_AMOUNT * pow(0.5 + 0.5 * cos(p.y * 6.28 * SCAN_LINE_COUNT), SCAN_LINE_FACTOR);
    c.rgb -= f;
    return c;
}

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;

    vec2 uv_monitor = bulge_coords(uv, BULGE_AMOUNT);
    vec2 uv_fb = roll_coords(uv_monitor);

    vec4 c = sample_abberation(uv_fb, uv_monitor);
    c = vignette(c, uv_monitor);
    c = scan_lines(c, uv_monitor);

    f_color = c;
    f_color = mix(f_color, vec4(0.0, 0.0, 0.0, 1.0), iFadeAmount);
}
