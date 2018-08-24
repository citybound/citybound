import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import { vec3, mat4, quat } from 'gl-matrix';
import ContainerDimensions from 'react-container-dimensions';
import update from 'immutability-helper';
import Mousetrap from 'mousetrap';
window.update = update;

import * as cityboundBrowser from './Cargo.toml';
import * as Planning from './src/planning_browser/Planning';
import * as Transport from './src/transport_browser/Transport';
import * as LandUse from './src/land_use_browser/LandUse';
import * as Simulation from './src/simulation_browser/Simulation';
import * as Debug from './src/debug/Debug';
import Stage from './src/stage/Stage';
import colors from './src/colors';


const EL = React.createElement;

class CityboundClient extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            planning: Planning.initialState,
            transport: Transport.initialState,
            landUse: LandUse.initialState,
            debug: Debug.initialState,
            uiMode: "main",
            system: {
                networkingTurns: ""
            },
            rendering: {
                enabled: true
            },
            simulation: Simulation.initialState,
            view: {
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
        }

        this.renderer = React.createRef();
    }

    componentDidMount() {
        const inputActions = {
            "toggleDebugView": () => this.setState(oldState => update(oldState, {
                debug: { show: { $apply: b => !b } }
            })),
            "startRotateEye": () => this.setState(oldState => update(oldState, {
                view: { rotating: { $set: true } }
            })),
            "stopRotateEye": () => this.setState(oldState => update(oldState, {
                view: { rotating: { $set: false } }
            })),
            "implementProposal": () => this.setState(Planning.implementProposal(this.state.planning.currentProposal))
        };

        Mousetrap.bind(',', inputActions["toggleDebugView"]);
        Mousetrap.bind('command+enter', inputActions["implementProposal"]);
        Mousetrap.bind('alt', inputActions["startRotateEye"], 'keydown');
        Mousetrap.bind('alt', inputActions["stopRotateEye"], 'keyup');
    }

    onFrame() {
        if (this.state.rendering.enabled) {
            this.renderer.current.renderFrame();
        }
    }

    render() {
        const uiAspects = [
            Planning,
            Transport,
            LandUse,
            Debug
        ];

        const uiAspectsRendered = uiAspects.map(aspect => aspect.render(this.state, this.setState.bind(this)));

        const layers = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.layers || []), []);
        const interactables = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.interactables || []), []);
        const tools = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.tools || []), []);
        const windows = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.windows || []), []);

        const { eye, target, verticalFov } = this.state.view;

        return EL("div", {
            style: { width: "100%", height: "100%" },
            onWheel: e => {
                if (this.state.view.rotating) {
                    const eyeRotatedHorizontal = vec3.rotateZ(
                        vec3.create(),
                        eye,
                        target,
                        e.deltaX * this.state.view.rotateXSensitivity
                    );

                    const forward = vec3.sub(vec3.create(), target, eyeRotatedHorizontal);
                    forward[2] = 0;
                    vec3.normalize(forward, forward);
                    const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

                    const verticalRotation = quat.setAxisAngle(
                        quat.create(),
                        sideways,
                        e.deltaY * this.state.view.rotateYSensitivity
                    );

                    const eyeRotatedBoth = vec3.transformQuat(
                        vec3.create(),
                        eyeRotatedHorizontal,
                        verticalRotation
                    );

                    if (eyeRotatedBoth[2] > 10 && vec3.dot(forward, vec3.sub(vec3.create(), target, eyeRotatedBoth)) > 0) {
                        this.setState(oldState => ({
                            view: Object.assign(oldState.view, {
                                eye: eyeRotatedBoth,
                            })
                        }));
                    }

                } else if (this.state.view.zooming || (this.state.view.pinchToZoom && e.ctrlKey)) {
                    const forward = vec3.sub(vec3.create(), target, eye);
                    vec3.normalize(forward, forward);

                    const heightBasedMultiplier = vec3.dist(target, eye) / 200;

                    const delta = this.state.view.zoomSensitivity * e.deltaY * heightBasedMultiplier;
                    const eyeZoomed = vec3.scaleAndAdd(
                        vec3.create(),
                        eye,
                        forward,
                        delta
                    );

                    if (eyeZoomed[2] > 10) {
                        this.setState(oldState => ({
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
                            e.deltaY * this.state.view.panYSensitivity * heightBasedMultiplier
                        ),
                        sideways,
                        e.deltaX * this.state.view.panXSensitivity * heightBasedMultiplier
                    );

                    this.setState(oldState => ({
                        view: Object.assign(oldState.view, {
                            eye: vec3.add(vec3.create(), oldState.view.eye, delta),
                            target: vec3.add(vec3.create(), oldState.view.target, delta),
                        })
                    }));
                }

                e.preventDefault();
                return false;
            },
        },
            EL(ContainerDimensions, { style: { width: "100%", height: "100%", position: "relative" } }, ({ width, height }) => {
                const viewMatrix = mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]);
                const perspectiveMatrix = mat4.perspective(mat4.create(), verticalFov, width / height, 0.1, 50000);

                return EL("div", { style: { width, height } }, [
                    EL("div", { key: "ui2dTools", className: "ui2dTools" }, [
                        ...tools
                    ]),
                    EL("div", { key: "ui2d", className: "ui2d" }, [
                        ...windows
                    ]),
                    EL(Monet, {
                        key: "canvas",
                        ref: this.renderer,
                        layers,
                        width, height,
                        retinaFactor: 2,
                        viewMatrix, perspectiveMatrix,
                        clearColor: [...colors.grass, 1.0]
                    }),
                    EL(Stage, {
                        key: "stage",
                        interactables,
                        width, height,
                        eye, target, verticalFov,
                        style: { width, height, position: "absolute", top: 0, left: 0 }
                    })
                ])
            })
        );
    }
}

window.cbclient = ReactDOM.render(EL(CityboundClient), document.getElementById('app'));

cityboundBrowser.start();