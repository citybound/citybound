import { solidColorShader } from 'monet';

const stripedShaders = [
    "mod(p.x + p.y, 6.0) < 2.0 && mod(p.x - p.y, 6.0) > 2.0",
    "mod(p.x + p.y, 6.0) > 2.0 && mod(p.x + p.y, 6.0) < 4.0 && mod(p.x - p.y, 6.0) > 2.0",
    "mod(p.x + p.y, 6.0) > 4.0 && mod(p.x - p.y, 6.0) > 2.0"
].map(condition => ({
    vertex: solidColorShader.vertex,
    fragment: `
precision mediump float;
varying vec3 p;
varying vec3 color;
void main() {
    if (${condition}) {
        gl_FragColor = vec4(pow(color, vec3(1.0/2.2)), 1.0);
    } else {
        discard;
    }
}`
}));

export const shadersForLandUses = {
    Residential: stripedShaders[0],
    Commercial: stripedShaders[1],
    Industrial: stripedShaders[2],
    Agricultural: stripedShaders[1],
    Recreational: stripedShaders[2],
    Administrative: stripedShaders[2]
};
