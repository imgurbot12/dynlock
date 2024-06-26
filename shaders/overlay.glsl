#version 450
// LICENSE: MIT (https://github.com/RobinMcCorkell/shaderlock/blob/master/LICENSE)
// AUTHOR:  RobinMcCorkell
// FOUND:   https://github.com/RobinMcCorkell/shaderlock/blob/master/dist/shaders/overlay.frag
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

const float PI = 3.141529;
const float SPEED = 0.3;
const float DIRECTIONS = 3.0;
const float OFFSET_FRACT = 0.1;
const float LOD_BIAS = 3.0;

vec4 overlay(vec2 uv, float amount) {
    vec4 color = vec4(0.0);
    for (float d = OFFSET_FRACT*PI/DIRECTIONS; d < PI; d += PI/DIRECTIONS) {
        color += texture(sampler2D(t_screenshot, s_screenshot), uv + vec2(cos(d), sin(d)) * amount, LOD_BIAS);
    }
    return color / DIRECTIONS;
}

void main() {
    float amount = SPEED * iTime;

    vec2 uv = gl_FragCoord.xy / iResolution.xy;

    f_color = overlay(uv, amount);
    f_color = mix(f_color, vec4(0.0, 0.0, 0.0, 1.0), iFadeAmount);
}
