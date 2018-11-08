import { vec3, mat4, quat } from 'gl-matrix';
import Mousetrap from 'mousetrap';
import update from 'immutability-helper';

export const initialState = {
    target: [0, 0, 0],
    distance: 212,
    heading: 0.25 * Math.PI,
    pitch: 0.25 * Math.PI,
    panning: false,
    keyboardPanning: { x: 0, y: 0 },
    lastMousePos: null,
    rotating: false,
    zooming: false,
    headingDistanceAtGestureStart: null,
}

export const settingSpec = {
    scrollingPans: { default: false, description: "Meaning of scrolling", falseDescription: "Zooming", trueDescription: "Panning (For Multitouch Trackpad)" },
    pinchToZoom: { default: true, description: "Pinch‑to‑zoom", falseDescription: "Off", trueDescription: "On" },
    zoomSensitivity: { default: 0.3, min: -10, max: 10, step: 0.1, description: "Zoom sensitivity" },
    mousePanKey: { default: { key: 'shift' }, description: "Pan with mouse" },
    panXSensitivity: { default: 1, min: -10, max: 10, step: 0.1, description: "Pan sensitivity ↔︎" },
    panYSensitivity: { default: 1, min: -10, max: 10, step: 0.1, description: "Pan sensitivity ↕︎" },
    keyboardPanKeyForward: { default: { key: 'up' }, description: "Pan ↑" },
    keyboardPanKeyBackward: { default: { key: 'down' }, description: "Pan ↓" },
    keyboardPanKeyLeft: { default: { key: 'left' }, description: "Pan ←" },
    keyboardPanKeyRight: { default: { key: 'right' }, description: "Pan →" },
    keyboardPanSpeed: { default: 0.6, min: -10, max: 10, step: 0.1, description: "Pan keys speed" },
    rotateKey: { default: { key: 'alt' }, description: "Rotate with mouse" },
    rotateXSensitivity: { default: 1, min: -10, max: 10, step: 0.1, description: "Rotate sensitivity ↔︎" },
    rotateYSensitivity: { default: 1, min: -10, max: 10, step: 0.1, description: "Rotate sensitivity ↕︎" },
    verticalFov: { default: 0.3, description: "Fisheye", min: 0.1, max: 0.9, step: 0.02 }
}

const MIN_DISTANCE = 10;

export function bindInputs(state, setState) {
    const inputActions = {
        "startRotateCamera": () => setState(oldState => update(oldState, {
            camera: { rotating: { $set: true } }
        })),
        "stopRotateCamera": () => setState(oldState => update(oldState, {
            camera: { rotating: { $set: false } }
        })),
        "startPanCamera": () => setState(oldState => update(oldState, {
            camera: { panning: { $set: true } }
        })),
        "stopPanCamera": () => setState(oldState => update(oldState, {
            camera: { panning: { $set: false }, lastMousePos: { $set: null } }
        })),
        "keyboardPanCamera": (panUpdate) => () => setState(oldState => update(oldState, {
            camera: { keyboardPanning: panUpdate }
        })),
    };

    Mousetrap.bind(state.settings.camera.rotateKey.key, inputActions["startRotateCamera"], 'keydown');
    Mousetrap.bind(state.settings.camera.rotateKey.key, inputActions["stopRotateCamera"], 'keyup');
    Mousetrap.bind(state.settings.camera.mousePanKey.key, inputActions["startPanCamera"], 'keydown');
    Mousetrap.bind(state.settings.camera.mousePanKey.key, inputActions["stopPanCamera"], 'keyup');

    Mousetrap.bind(state.settings.camera.keyboardPanKeyForward.key, inputActions["keyboardPanCamera"]({ y: { $set: 1 } }), 'keydown');
    Mousetrap.bind(state.settings.camera.keyboardPanKeyForward.key, inputActions["keyboardPanCamera"]({ y: { $set: 0 } }), 'keyup');

    Mousetrap.bind(state.settings.camera.keyboardPanKeyBackward.key, inputActions["keyboardPanCamera"]({ y: { $set: -1 } }), 'keydown');
    Mousetrap.bind(state.settings.camera.keyboardPanKeyBackward.key, inputActions["keyboardPanCamera"]({ y: { $set: 0 } }), 'keyup');

    Mousetrap.bind(state.settings.camera.keyboardPanKeyRight.key, inputActions["keyboardPanCamera"]({ x: { $set: 1 } }), 'keydown');
    Mousetrap.bind(state.settings.camera.keyboardPanKeyRight.key, inputActions["keyboardPanCamera"]({ x: { $set: 0 } }), 'keyup');

    Mousetrap.bind(state.settings.camera.keyboardPanKeyLeft.key, inputActions["keyboardPanCamera"]({ x: { $set: -1 } }), 'keydown');
    Mousetrap.bind(state.settings.camera.keyboardPanKeyLeft.key, inputActions["keyboardPanCamera"]({ x: { $set: 0 } }), 'keyup');

    document.addEventListener("gesturestart", e => {
        console.log("start")
        setState(oldState => update(oldState, {
            camera: {
                headingDistanceAtGestureStart: {
                    $set: {
                        distance: oldState.camera.distance,
                        heading: oldState.camera.heading,
                    }
                },
                gestureClientPos: { $set: [e.clientX, e.clientY] }
            }
        }));

        e.preventDefault();
    })

    document.addEventListener("gesturechange", e => {
        setState(oldState => {
            const delta = e.scale - 1.0;
            const scaledDelta = delta * oldState.settings.camera.zoomSensitivity / 3.0
            const scaledMul = Math.max(1.0 + scaledDelta, 0.01);
            const newDistance = oldState.camera.headingDistanceAtGestureStart.distance / scaledMul;

            const fakeWheelEvent = {
                deltaX: -(e.clientX - oldState.camera.gestureClientPos[0]),
                deltaY: -(e.clientY - oldState.camera.gestureClientPos[1]),
            };

            onWheel(fakeWheelEvent, oldState, setState);

            return update(oldState, {
                camera: {
                    distance: { $set: Math.max(MIN_DISTANCE, newDistance) },
                    heading: { $set: oldState.camera.headingDistanceAtGestureStart.heading + e.rotation * 0.03 * oldState.settings.camera.rotateXSensitivity },
                    gestureClientPos: { $set: [e.clientX, e.clientY] }
                }
            })
        });

        e.preventDefault();
    })

    document.addEventListener("gestureend", e => {
        console.log("end")
        setState(oldState => update(oldState, {
            camera: {
                headingDistanceAtGestureStart: { $set: null }
            }
        }));

        e.preventDefault();
    })
}

export function onWheel(e, state, setState) {
    const { distance, heading } = state.camera;

    if (state.camera.rotating) {
        const deltaY = -e.deltaY * state.settings.camera.rotateYSensitivity / 100;
        const deltaX = e.deltaX * state.settings.camera.rotateXSensitivity / 100;

        setState(oldState => update(oldState, {
            camera: {
                pitch: { $apply: oldPitch => Math.max(0.01, Math.min(0.49 * Math.PI, oldPitch + deltaY)) },
                heading: { $apply: oldHeading => (oldHeading + deltaX) % (2 * Math.PI) }
            }
        }))
    } else if (!state.settings.camera.scrollingPans || state.camera.zooming || (state.settings.camera.pinchToZoom && e.ctrlKey)) {
        const distanceBasedMultiplier = distance / 200;
        const delta = state.settings.camera.zoomSensitivity * e.deltaY * distanceBasedMultiplier;

        setState(oldState => update(oldState, {
            camera: {
                distance: { $apply: oldDistance => Math.max(MIN_DISTANCE, oldDistance + delta) }
            }
        }));
    } else if (state.settings.camera.scrollingPans) {
        const distanceBasedMultiplier = distance / 200;
        const forward = vec3.fromValues(Math.cos(heading), Math.sin(heading), 0.0);
        const sideways = vec3.fromValues(-Math.sin(heading), Math.cos(heading), 0.0);

        const deltaForward = vec3.scale(vec3.create(), forward, -e.deltaY * state.settings.camera.panYSensitivity);
        const delta = vec3.scaleAndAdd(vec3.create(), deltaForward, sideways, -e.deltaX * state.settings.camera.panXSensitivity);

        setState(oldState => update(oldState, {
            camera: {
                target: { $apply: oldTarget => vec3.scaleAndAdd(vec3.create(), oldTarget, delta, distanceBasedMultiplier) }
            }
        }));
    }
}

export function onMouseMove(e, state, setState) {
    const { distance, heading } = state.camera;

    if (state.camera.lastMousePos) {
        const rawDeltaX = e.screenX - state.camera.lastMousePos[0];
        const rawDeltaY = e.screenY - state.camera.lastMousePos[1];

        if (state.camera.panning) {
            const distanceBasedMultiplier = distance / 200;
            const forward = vec3.fromValues(Math.cos(heading), Math.sin(heading), 0.0);
            const sideways = vec3.fromValues(-Math.sin(heading), Math.cos(heading), 0.0);

            const deltaForward = vec3.scale(vec3.create(), forward, rawDeltaY * state.settings.camera.panYSensitivity);
            const delta = vec3.scaleAndAdd(vec3.create(), deltaForward, sideways, rawDeltaX * state.settings.camera.panXSensitivity);

            setState(oldState => update(oldState, {
                camera: {
                    target: { $apply: oldTarget => vec3.scaleAndAdd(vec3.create(), oldTarget, delta, distanceBasedMultiplier) }
                }
            }));
        } else if (state.camera.rotating) {
            const deltaX = -rawDeltaX * state.settings.camera.rotateXSensitivity / 100;
            const deltaY = rawDeltaY * state.settings.camera.rotateYSensitivity / 100;

            setState(oldState => update(oldState, {
                camera: {
                    pitch: { $apply: oldPitch => Math.max(0.01, Math.min(0.49 * Math.PI, oldPitch + deltaY)) },
                    heading: { $apply: oldHeading => (oldHeading + deltaX) % (2 * Math.PI) }
                }
            }))
        }
    }

    state.camera.lastMousePos = [e.screenX, e.screenY];
}

export function onFrame(state, setState) {

    if (state.camera.keyboardPanning.x || state.camera.keyboardPanning.y) {
        const { distance, heading } = state.camera;

        const distanceBasedMultiplier = distance / 200;
        const forward = vec3.fromValues(Math.cos(heading), Math.sin(heading), 0.0);
        const sideways = vec3.fromValues(Math.sin(heading), -Math.cos(heading), 0.0);

        const deltaForward = vec3.scale(vec3.create(), forward, state.camera.keyboardPanning.y * 10 * state.settings.camera.keyboardPanSpeed);
        const delta = vec3.scaleAndAdd(vec3.create(), deltaForward, sideways, state.camera.keyboardPanning.x * 10 * state.settings.camera.keyboardPanSpeed);

        setState(oldState => update(oldState, {
            camera: {
                target: { $apply: oldTarget => vec3.scaleAndAdd(vec3.create(), oldTarget, delta, distanceBasedMultiplier) }
            }
        }));
    }

}

export function getMatrices(state, width, height) {
    const { target, heading, pitch, distance } = state.camera;

    const eye2DRelative = vec3.fromValues(-distance * Math.cos(heading), -distance * Math.sin(heading), 0.0);
    const eyeHeight = distance * Math.sin(pitch);
    const eye3DRelative = vec3.scaleAndAdd(vec3.create(), vec3.fromValues(0.0, 0.0, eyeHeight), eye2DRelative, Math.cos(pitch));
    const eye = vec3.add(vec3.create(), target, eye3DRelative);

    return {
        eye,
        target,
        viewMatrix: mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]),
        perspectiveMatrix: mat4.perspective(mat4.create(), state.settings.camera.verticalFov * Math.PI, width / height, 0.1, 50000)
    };
}