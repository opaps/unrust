#ifndef GL_ES
#define attribute in
#define varying out
#endif

attribute vec3 aVertexPosition;
attribute vec3 aVertexNormal;
attribute vec2 aTextureCoord;

uniform mat4 uMVMatrix;
uniform mat4 uPMatrix;
uniform mat4 uNMatrix;
uniform mat4 uMMatrix;

varying vec3 vFragPos;
varying vec3 vNormal;
varying vec2 vTexCoords;

void main(void) {
    vFragPos = vec3(uMMatrix * vec4(aVertexPosition, 1.0));            
    vNormal = mat3(uNMatrix) * aVertexNormal;
    vTexCoords = aTextureCoord;

    gl_Position = uPMatrix * uMVMatrix * vec4(aVertexPosition, 1.0);
}
