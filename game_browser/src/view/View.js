import { vec3, mat4, quat } from 'gl-matrix';
import Mousetrap from 'mousetrap';
import update from 'immutability-helper';

export const initialState = {
    eye: [-150, -150, 150],
    target: [0, 0, 0],
    verticalFov: 0.3 * Math.PI,
    rotating: false,
    zooming: false,
    rotateXSensitivity: 0.01,
    rotateYSensitivity: -0.01,
    panXSensitivity: -1,
    panYSensitivity: -1,
    zoomSensitivity: -5,
    pinchToZoom: true
}

export function bindInputs(_state, setState) {
    const inputActions = {

        "startRotateEye": () => setState(oldState => update(oldState, {
            view: { rotating: { $set: true } }
        })),
        "stopRotateEye": () => setState(oldState => update(oldState, {
            view: { rotating: { $set: false } }
        })),
    };

    Mousetrap.bind('alt', inputActions["startRotateEye"], 'keydown');
    Mousetrap.bind('alt', inputActions["stopRotateEye"], 'keyup');
}

export function onWheel(e, state, setState) {
    const { eye, target } = state.view;

    if (state.view.rotating) {
        const eyeRotatedHorizontal = vec3.rotateZ(
            vec3.create(),
            eye,
            target,
            e.deltaX * state.view.rotateXSensitivity
        );

        const forward = vec3.sub(vec3.create(), target, eyeRotatedHorizontal);
        forward[2] = 0;
        vec3.normalize(forward, forward);
        const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

        const verticalRotation = quat.setAxisAngle(
            quat.create(),
            sideways,
            e.deltaY * state.view.rotateYSensitivity
        );

        const eyeRotatedBoth = vec3.transformQuat(
            vec3.create(),
            eyeRotatedHorizontal,
            verticalRotation
        );

        if (eyeRotatedBoth[2] > 10 && vec3.dot(forward, vec3.sub(vec3.create(), target, eyeRotatedBoth)) > 0) {
            setState(oldState => ({
                view: Object.assign(oldState.view, {
                    eye: eyeRotatedBoth,
                })
            }));
        }

    } else if (state.view.zooming || (state.view.pinchToZoom && e.ctrlKey)) {
        const forward = vec3.sub(vec3.create(), target, eye);
        vec3.normalize(forward, forward);

        const heightBasedMultiplier = vec3.dist(target, eye) / 200;

        const delta = state.view.zoomSensitivity * e.deltaY * heightBasedMultiplier;
        const eyeZoomed = vec3.scaleAndAdd(
            vec3.create(),
            eye,
            forward,
            delta
        );

        if (eyeZoomed[2] > 10) {
            setState(oldState => ({
                view: Object.assign(oldState.view, {
                    eye: eyeZoomed
                })
            }));
        }
    } else {
        const forward = vec3.sub(vec3.create(), target, eye);
        forward[2] = 0;
        vec3.normalize(forward, forward);
        const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

        const heightBasedMultiplier = vec3.dist(target, eye) / 200;

        const delta = vec3.scaleAndAdd(vec3.create(),
            vec3.scale(
                vec3.create(),
                forward,
                e.deltaY * state.view.panYSensitivity * heightBasedMultiplier
            ),
            sideways,
            e.deltaX * state.view.panXSensitivity * heightBasedMultiplier
        );

        setState(oldState => ({
            view: Object.assign(oldState.view, {
                eye: vec3.add(vec3.create(), oldState.view.eye, delta),
                target: vec3.add(vec3.create(), oldState.view.target, delta),
            })
        }));
    }
}

export function getMatrices(state, width, height) {
    const { eye, target, verticalFov } = state.view;

    return {
        viewMatrix: mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]),
        perspectiveMatrix: mat4.perspective(mat4.create(), verticalFov, width / height, 0.1, 50000)
    };
}