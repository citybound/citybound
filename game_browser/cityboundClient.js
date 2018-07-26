import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import { vec3, vec4, mat4 } from 'gl-matrix';
import ContainerDimensions from 'react-container-dimensions';
import update from 'immutability-helper';
window.update = update;

import * as cityboundBrowser from './Cargo.toml';
import * as Planning from './src/planning_browser/Planning';
import * as Transport from './src/transport_browser/Transport';
import Stage from './src/stage/Stage';
import colors from './src/colors';


const EL = React.createElement;

class CityboundClient extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            planning: Planning.initialState,
            transport: Transport.initialState,
            uiMode: "main",
            system: {
                networkingTurns: ""
            },
            view: {
                eye: [-150, -150, 150],
                target: [0, 0, 0],
                verticalFov: 0.3 * Math.PI
            }
        }

    }

    render() {
        const [planningLayers, planningInteractables, planningElements] = Planning.render(this.state, this.setState.bind(this));
        const [transportLayers, transportInteractables, transportElements] = Transport.render(this.state, this.setState.bind(this));

        const layers = [
            ...transportLayers,
            ...planningLayers
        ];

        const interactables = [
            ...planningInteractables,
            ...transportInteractables
        ];

        const { eye, target, verticalFov } = this.state.view;

        return EL("div", {
            style: { width: "100%", height: "100%" },
            onWheel: e => {
                const forward = vec3.sub(vec3.create(), target, eye);
                forward[2] = 0;
                vec3.normalize(forward, forward);
                const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

                const delta = vec3.scaleAndAdd(vec3.create(), vec3.scale(vec3.create(), forward, -e.deltaY), sideways, -e.deltaX);

                this.setState(oldState => ({
                    view: Object.assign(oldState.view, {
                        eye: vec3.add(vec3.create(), oldState.view.eye, delta),
                        target: vec3.add(vec3.create(), oldState.view.target, delta),
                    })
                }));

                e.preventDefault();
                return false;
            }
        },
            EL(ContainerDimensions, { style: { width: "100%", height: "100%", position: "relative" } }, ({ width, height }) => {
                const viewMatrix = mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]);
                const perspectiveMatrix = mat4.perspective(mat4.create(), verticalFov, width / height, 50000, 0.1);

                return EL("div", { style: { width, height } }, [
                    EL("div", { key: "ui2d", className: "ui2d" }, [
                        ...planningElements,
                        ...transportElements,
                        EL("div", { key: "networking", className: "window networking" },
                            EL("pre", {}, this.state.system.networkingTurns)
                        )
                    ]),
                    EL(Monet, {
                        key: "canvas",
                        layers,
                        width, height,
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