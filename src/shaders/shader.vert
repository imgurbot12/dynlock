#version 450

// display made up of two opposite triangles
const vec2 positions[6] = vec2[6](
    vec2(-1.0,  1.0), //c
    vec2(-1.0, -1.0), //a
    vec2( 1.0, -1.0), //b
    vec2(-1.0,  1.0), //c
    vec2( 1.0, -1.0), //b
    vec2( 1.0,  1.0)  //d
);

// relative positions for texture drawing
const vec2 tex_positions[6] = vec2[6](
    vec2(0.0, -1.0), //w
    vec2(0.0, 0.0),  //x
    vec2(1.0, 0.0),  //y
    vec2(0.0, -1.0), //w
    vec2(1.0, 0.0),  //y
    vec2(1.0, -1.0)  //z
);

// layout(location=0) out vec2 v_tex_coords;

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    // v_tex_coords = tex_positions[gl_VertexIndex];
}
