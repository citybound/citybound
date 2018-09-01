import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import ContainerDimensions from 'react-container-dimensions';
import update from 'immutability-helper';
window.update = update;

import * as cityboundBrowser from '../Cargo.toml';
import * as View from './view/View';
import * as Planning from './planning_browser/Planning';
import * as Transport from './transport_browser/Transport';
import * as LandUse from './land_use_browser/LandUse';
import * as Simulation from './simulation_browser/Simulation';
import * as Debug from './debug/Debug';
import Stage from './stage/Stage';
import colors from './colors';

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
            view: View.initialState,
        }

        this.renderer = React.createRef();
    }

    componentDidMount() {
        View.bindInputs(this.state, this.setState.bind(this));
        Debug.bindInputs(this.state, this.setState.bind(this));
        Planning.bindInputs(this.state, this.setState.bind(this));
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
                View.onWheel(e, this.state, this.setState.bind(this));
                e.preventDefault();
                return false;
            },
        },
            EL(ContainerDimensions, { style: { width: "100%", height: "100%", position: "relative" } }, ({ width, height }) => {
                const { viewMatrix, perspectiveMatrix } = View.getMatrices(this.state, width, height);

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