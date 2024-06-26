#version 450
// LICENSE: WTFPL/CC? (https://creativecommons.org/2015/05/06/medium-embraces-cc-licenses/)
// AUTHOR:  Jack Chan
// FOUND:   https://medium.com/nerd-for-tech/logging-in-rust-e529c241f92e
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

vec3 colorA = vec3(0.149,0.141,0.912);
vec3 colorB = vec3(1.000,0.833,0.224);

void main() {
    vec3 color = vec3(0.0);
    float pct  = abs(sin(iTime));
    color      = mix(colorA, colorB, pct);
    f_color    = vec4(color,1.0);
}

