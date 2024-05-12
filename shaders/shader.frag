#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

void main() {
    // f_color = vec4(1.0, 0.59, 0.132, 0.196);
    f_color = texture(sampler2D(t_screenshot, s_screenshot), v_tex_coords);
}
