#version 450
// LICENSE: CC BY-NC-SA 3.0 DEED (https://www.shadertoy.com/terms)
// AUTHOR:  gamsas
// FOUND:   https://www.shadertoy.com/view/mtGczd
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

float SPEED = iTime * 0.5;

// [ch3ck3r] 
// https://www.instagram.com/ar/709155497587738

// PUBLIC DOMAIN CRT STYLED SCAN-LINE SHADER by Timothy Lottes
// https://www.shadertoy.com/view/XsjSzR

#define PI 3.1415926535897932384626433832795

//short special case of smoothstep(), turn gratient to sharpened border.
float ssf(float a,float d){return smoothstep(-a,a,d);}

  //triangle-function.
  //x includes phase and x*n has n adjust "range" 
float waveTri(float x,float d){d*=2.;
  return min(mod(x,d),mod(-x,d));
} //return abs((x)/w);//waveSaw saw-function

const float hardScan=-8.0;
const float hardPix=-2.;

// Display warp.
const vec2 warp=vec2(1.0/32.0,1.0/24.0); 

// Amount of shadow mask.
const float maskDark=0.4;


// Nearest emulated sample given floating point position and texel offset.
// Also zero's off screen.
vec3 Fetch(vec2 pos,vec2 off, float psize){
    vec2 res = iResolution.xy/psize;
    pos=floor(pos*res+off)/res;
    if(max(abs(pos.x-0.5),abs(pos.y-0.5))>0.5)return vec3(0.0,0.0,0.0);
    return texture(sampler2D(t_screenshot, s_screenshot),pos.xy).rgb;}


// Distance in emulated pixels to nearest texel.
vec2 Dist(vec2 pos, float gap){
  pos=pos*(iResolution.xy/gap);
  return -((pos-floor(pos))-vec2(0.5));}
    
// 1D Gaussian.
float Gaus(float pos,float scale){return exp2(scale*pos*pos);}

// 3-tap Gaussian filter along horz line.
vec3 Horz3(vec2 pos,float off,float gap, float psize){
  vec3 b=Fetch(pos,vec2(-1.0,off),psize);
  vec3 c=Fetch(pos,vec2( 0.0,off),psize);
  vec3 d=Fetch(pos,vec2( 1.0,off),psize);
  float dst=Dist(pos,gap).x;
  // Convert distance to weight.
  float scale=hardPix;
  float wb=Gaus(dst-1.0,scale);
  float wc=Gaus(dst+0.0,scale);
  float wd=Gaus(dst+1.0,scale);
  // Return filtered sample.
  return (b*wb+c*wc+d*wd)/(wb+wc+wd);}

// 5-tap Gaussian filter along horz line.
vec3 Horz5(vec2 pos,float off, float gap, float psize){
  vec3 a=Fetch(pos,vec2(-2.0,off),psize);
  vec3 b=Fetch(pos,vec2(-1.0,off),psize);
  vec3 c=Fetch(pos,vec2( 0.0,off),psize);
  vec3 d=Fetch(pos,vec2( 1.0,off),psize);
  vec3 e=Fetch(pos,vec2( 2.0,off),psize);
  float dst=Dist(pos,gap).x;
  // Convert distance to weight.
  float scale=hardPix;
  float wa=Gaus(dst-2.0,scale);
  float wb=Gaus(dst-1.0,scale);
  float wc=Gaus(dst+0.0,scale);
  float wd=Gaus(dst+1.0,scale);
  float we=Gaus(dst+2.0,scale);
  // Return filtered sample.
  return (a*wa+b*wb+c*wc+d*wd+e*we)/(wa+wb+wc+wd+we);}

// Return scanline weight.
float Scan(vec2 pos,float off, float gap){
  float dst=Dist(pos,gap).y;
  return Gaus(dst+off,hardScan);}

// Allow nearest three lines to effect pixel.
vec3 Tri(vec2 pos, float gap , float psize){
  vec3 a=Horz3(pos,-1.0,gap, psize);
  vec3 b=Horz5(pos, 0.0,gap,psize);
  vec3 c=Horz3(pos, 1.0,gap,psize);
  float wa=Scan(pos,-1.0, gap);
  float wb=Scan(pos, 0.0, gap);
  float wc=Scan(pos, 1.0, gap);
  return a*wa+b*wb+c*wc;}

// Distortion of scanlines, and end of screen alpha.
vec2 Warp(vec2 pos){
  pos=pos*2.0-1.0;    
  pos*=vec2(1.0+(pos.y*pos.y)*warp.x,1.0+(pos.x*pos.x)*warp.y);
  return pos*0.5+0.5;}

// Shadow mask.
vec3 Mask(vec2 pos, float maskLight){
  
  pos.x+=pos.y*3.0;
  vec3 mask=vec3(maskDark,maskDark,maskDark);
  pos.x=fract(pos.x/6.0);
  if(pos.x<0.333)mask.r=maskDark;
  else if(pos.x<0.666)mask.g=maskLight;
  else mask.b=maskLight;
  return mask;}    

// Draw dividing bars.
float Bar(float pos,float bar){pos-=bar;return pos*pos<4.0?0.0:1.0;}

vec3 pixelize(vec2 p, float light, in vec2 fragCoord,float gap, float psize){
    float maskLight =light;
    float maskDark=light;
    vec2 pos = p;
    return Tri(pos,gap, psize) * Mask(fragCoord.xy, maskLight); 
}

float squareEdgeSoftness(vec2 st, float size) {
  vec2 square = abs(fract(st - 0.5) - 0.5) / fwidth(st) * size;
  return clamp(min(square.x, square.y), 0.0, 1.);
}

void main(){
    vec2 uv = gl_FragCoord.xy/iResolution.xy;
    uv = uv * 2.0 - 1.0;

    // speed
    float S = 0.12;
    float ds[9];
  
    // wavy effect
    uv.x += sin(uv.y*1.+SPEED*S*PI*3.)*0.5;
    
    // x-axis movement
    uv.x += cos(SPEED*S*PI*1.)*1.5*0.5;
  
    float dist = 0.58;

    // move the layers
    for (int i = 0; i < ds.length(); i++) {
        ds[i] = (dist - mod(SPEED * S, dist)) + float(i) * dist;
    }

    // Modify displacement for x-axis for each layer
    float dx = sin(SPEED)*0.1;
    float dy = -S*8.;

    
    for (int i = 0; i < ds.length(); i++) {

        float d = ds[i];
        float size = d*6.;
        // float modd = mod(float(i),3.);

        float x0 = uv.x;
        float y0 = uv.y;

        // Modify displacement for x-axis for each layer
        x0 += d * dx;
        y0 += d * dy;

        // change freq for layer
        float x1 = x0 * d;
        float y1 = y0 * d;
   
        // center vertical
        y1 += PI / 1.0;
        
        int xon = cos(x1*size) > 0.0 ? 1 : 0;
        int yon = sin(y1*size) > 0.0 ? 1 : 0;
        int c = (xon != yon) ? 1 : 0;
        if (size < 2.) {c = 1;}
          
        // float test =float(xon)*floor(x1);

        if (c != 1) {
            float c1 = 1.0 - (d / (float(ds.length())));
            // c1 +=0.1;
            c1 = pow(c1+0.15, 4.);

            vec2 ch_uv = fract(vec2(x1 *size* (1./PI) - 0.5, y1 *size* (1./PI)));
            float smoothedge = squareEdgeSoftness(ch_uv, 0.3);
            
            ch_uv = Warp(ch_uv);
            
            float gap = size*4.;
            vec3 img = pixelize(ch_uv, pow(d,1.8)-d+0.5, gl_FragCoord.xy,gap,1.);                      

            vec3 r;
            float blur = 6.;
            blur/=16.;//lessen blur
    
            float shape = -(length(ch_uv-0.5)-0.5);     
           
            //tricolor:                                    
            float rr=ssf(blur,shape*1.20);
            float rg=ssf(blur,shape*1.20);
            float rb=ssf(blur,shape*fract(ch_uv.y+SPEED*8.)*(fract(SPEED*2.)*0.5+1.));
            r.rgb=vec3(rr,rg,rb);

            f_color = vec4(img*r*c1*(vec3(2.5,2.5,3.2)-d*d*size*0.03),smoothedge);
            break;
        }
      }
}
