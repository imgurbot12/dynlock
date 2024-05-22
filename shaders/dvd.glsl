#version 450
// LICENSE: CC BY-NC-SA 3.0 DEED (https://www.shadertoy.com/terms)
// AUTHOR:  dhooper
// FOUND:   https://www.shadertoy.com/view/wtcSzN
// STATUS:  slightly modified to work for dynlock

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_screenshot;
layout(set = 0, binding = 1) uniform sampler s_screenshot;

layout(push_constant) uniform FrameUniforms {
    float iTime;
    float iFadeAmount;
    vec2  iResolution;
};

#define PI 3.14159265359

float vmin(vec2 v) {
	return min(v.x, v.y);
}

float vmax(vec2 v) {
	return max(v.x, v.y);
}

float ellip(vec2 p, vec2 s) {
  float m = vmin(s);
	return (length(p / s) * m) - m;
}

float halfEllip(vec2 p, vec2 s) {
  p.x = max(0., p.x);
  float m = vmin(s);
	return (length(p / s) * m) - m;
}

float dvd_d(vec2 p) {
  float d = halfEllip(p, vec2(.8, .5));
  d = max(d, -p.x - .5);
  float d2 = halfEllip(p, vec2(.45, .3));
  d2 = max(d2, min(-p.y + .2, -p.x - .15));
  d = max(d, -d2);
  return d;
}

float dvd_v(vec2 p) {
  vec2 pp = p;
  p.y += .7;
  p.x = abs(p.x);
  vec2 a = normalize(vec2(1,-.55));
  float d = dot(p, a);
  float d2 = d + .3;
  p = pp;
  d = min(d, -p.y + .3);
  d2 = min(d2, -p.y + .5);
  d = max(d, -d2);
  d = max(d, abs(p.x + .3) - 1.1);
	return d;
}

float dvd_c(vec2 p) {
  p.y += .95;
	float d = ellip(p, vec2(1.8,.25));
  float d2 = ellip(p, vec2(.45,.09));
  d = max(d, -d2);
  return d;
}

float dvd(vec2 p) {
  p.y -= .345;
  p.x -= .035;
  p *= mat2(1,-.2,0,1);
	float d = dvd_v(p);
  d = min(d, dvd_c(p));
  p.x += 1.3;
  d = min(d, dvd_d(p));
  p.x -= 2.4;
  d = min(d, dvd_d(p));
  return d;
}

float range(float vmin, float vmax, float value) {
  return (value - vmin) / (vmax - vmin);
}

float rangec(float a, float b, float t) {
  return clamp(range(a, b, t), 0., 1.);
}

vec2 ref(vec2 p, vec2 planeNormal, float offset) {
	float t = dot(p, planeNormal) + offset;
	p -= (2. * t) * planeNormal;
  return p;
}

// Flip every second cell to create reflection
void flip(inout vec2 pos) {
    vec2 flip = mod(floor(pos), 2.);
    pos = abs(flip - mod(pos, 1.));
}

float stepSign(float a) {
    //return sign(a);
	return step(0., a) * 2. - 1.;
}

vec2 compassDir(vec2 p) {
    //return sign(p - sign(p) * vmin(abs(p))); // this caused problems on some GPUs
    vec2 a = vec2(stepSign(p.x), 0);
    vec2 b = vec2(0, stepSign(p.y));
    float s = stepSign(p.x - p.y) * stepSign(-p.x - p.y);
    return mix(a, b, s * .5 + .5);
}

vec2 calcHitPos(vec2 move, vec2 dir, vec2 size) {
    vec2 hitPos = mod(move, 1.);
    vec2 xCross = hitPos - hitPos.x / (size / size.x) * (dir / dir.x);
    vec2 yCross = hitPos - hitPos.y / (size / size.y) * (dir / dir.y);
   	hitPos = max(xCross, yCross);
    hitPos += floor(move);
    return hitPos;
}

vec3 hue( in float c ) {
    return cos(2.0*PI*c + 2.0*PI/3.0*vec3(3,2,1))*0.5+0.5;
}

const float LOGO_SCALE = .1;

void main ()
{
  vec2 p = (-iResolution.xy + 2.0*gl_FragCoord.xy)/iResolution.y * vec2(1.0, -1.0);

  vec2 screenSize = vec2(iResolution.x/iResolution.y, 1.) * 2.;

  float t = iTime;
  vec2 dir = normalize(vec2(9.,16) * screenSize );
  vec2 move = dir * t / 1.5;
  vec2 logoSize = vec2(2.,.85) * LOGO_SCALE * 1.;

  vec2 size = screenSize - logoSize * 2.;

  // Remap so (0,0) is bottom left, and (1,1) is top right
  move = move / size + .5;

  // hue shift the box with each bounce
  // vec2 period  = screenSize / size * 3;
  // vec2 tt      = floor(iTime/period);
  vec3 logoCol = hue((move.x+move.y)*0.1);

  // Calculate the point we last crossed a cell boundry
  vec2 hitPos = calcHitPos(move, dir, size);
  vec4 col    = vec4(1,1,1,0);

  // Flip every second cell to create reflection
  flip(hitPos);

	// Remap back to screen space
  hitPos = (hitPos - .5) * size;

  // Push the hits to the edges of the screen
  hitPos += logoSize * compassDir(hitPos / size);

  // Flip every second cell to create reflection
  flip(move);

  // Remap back to screen space
  move = (move - .5) * size;

  vec2 uv = gl_FragCoord.xy / iResolution.xy;
  col = texture(sampler2D(t_screenshot, s_screenshot), uv);

  // dvd logo
	float d = dvd((p - move) / LOGO_SCALE);
  d /= fwidth(d);
  d = 1. - clamp(d, 0., 1.);
  col.rgb = mix(col.rgb, logoCol, d);


  col.a = col.a * .5 + .5;
	col.a *= .3;
  f_color = col;
}
