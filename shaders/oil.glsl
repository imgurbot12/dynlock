#version 450
// LICENSE: CC BY-NC-SA 3.0 DEED (https://www.shadertoy.com/terms)
// AUTHOR:  flockaroo  
// FOUND:   https://www.shadertoy.com/view/MsGSRd
// STATUS:  highly customized to use base background image

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

void main()
{
    //Scaled pixel coordinates
    vec2 p=gl_FragCoord.xy/iResolution.y*6.;
    // 8 wave passes
    for(float i=0.0; i<8.0;i++)
    {
        //Add a simple sine wave with an offset and animation
        p.x += sin(p.y+i+iTime*.3);
        //Rotate and scale down
        p *= mat2(6,-8,8,6)/8.;
    }
    // pick a color using the turbulent coordinates
    vec2 wave  = sin(p.xy*.3+vec2(0,1))*.5+.5;
    vec3 color = texture(sampler2D(t_screenshot, s_screenshot), wave.xy).rgb;
    // mix with base image for first second
    if (iTime < 1.) {
      vec2 uv   = gl_FragCoord.xy / iResolution.xy;
      vec3 base = texture(sampler2D(t_screenshot, s_screenshot), uv.xy).rgb;
      color = mix(base, color, iTime);
    }
    f_color  = vec4(color, 1.0);
}
