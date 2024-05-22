#version 450
// LICENSE: CC BY-NC-SA 3.0 DEED (https://www.shadertoy.com/terms)
// AUTHOR:  existical
// FOUND:   https://www.shadertoy.com/view/Xltfzj
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

const float AMOUNT = 4.0;
const float SPEED  = 0.1;
const float ACCEL  = 1.8;

const float DIRECTIONS = 16.0;
const float QUALITY = 3.0;
const float SIZE = 8.0;

void main()
{
    float Pi = 6.28318530718; // Pi*2

    float tmod   = AMOUNT * clamp(pow(SPEED * iTime, ACCEL), 0.0, 1.0);
    vec2  radius = (SIZE / iResolution.xy) * tmod;
    vec2  uv     = gl_FragCoord.xy / iResolution.xy;
    vec4  color  = texture(sampler2D(t_screenshot, s_screenshot), uv);

    // Blur calculations
    for( float d=0.0; d<Pi; d+=Pi/DIRECTIONS)
    {
		for(float i=1.0/QUALITY; i<=1.0; i+=1.0/QUALITY)
        {
			color += texture(sampler2D(t_screenshot, s_screenshot), uv+vec2(cos(d),sin(d))*radius*i);
        }
    }

    // Output to screen
    color /= QUALITY * DIRECTIONS - 15.0;
    f_color =  color;
}
