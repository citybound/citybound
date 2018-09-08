const StackTrace = require("stacktrace-js");

function displayError(prefix, error) {
    const el = document.getElementById("errors");
    el.className = "errorsHappened";

    StackTrace.fromError(error).then(stackFrames => {
        document.getElementById("errorsloading").className = "loaded";
        el.insertAdjacentHTML("beforeend", `<h2>${prefix}: ${error.message}</h2>`);
        for (let frame of stackFrames) {
            const fileName = frame.fileName.replace(window.location.origin, "");
            el.insertAdjacentHTML("beforeend", `${frame.functionName} in ${fileName} ${frame.lineNumber}:${frame.columnNumber}<br/>`)
        }
        el.insertAdjacentHTML("beforeend", '<br/>');
    }).catch(() => alert("Error diplaying error, lol."));
}

window.onerror = function (msg, file, line, col, error) {
    displayError("Error", error);
};

window.addEventListener('unhandledrejection', function (e) {
    displayError("Unhandled Rejection", e.reason);
});

import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import ContainerDimensions from 'react-container-dimensions';
import update from 'immutability-helper';
window.update = update;

import * as cityboundBrowser from '../Cargo.toml';
import * as Camera from './camera/Camera';
import * as Planning from './planning_browser/Planning';
import * as Transport from './transport_browser/Transport';
import * as LandUse from './land_use_browser/LandUse';
import * as Simulation from './simulation_browser/Simulation';
import * as Debug from './debug/Debug';
import * as Settings from './settings';
import Stage from './stage/Stage';
import colors from './colors';

const EL = React.createElement;

const settingSpecs = {
    camera: Camera.settingSpec,
    debug: Debug.settingsSpec,
    planning: Planning.settingsSpec,
    rendering: {
        retinaFactor: { default: 2, description: "Oversampling/Retina Factor", min: 0.5, max: 4.0, step: 0.1 }
    }
};

class CityboundClient extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            planning: Planning.initialState,
            transport: Transport.initialState,
            landUse: LandUse.initialState,
            debug: Debug.initialState,
            uiMode: null,
            system: {
                networkingTurns: ""
            },
            rendering: {
                enabled: true
            },
            simulation: Simulation.initialState,
            camera: Camera.initialState,

            settingsMeta: Settings.initialState,
            settings: Settings.loadSettings(settingSpecs)
        }

        this.renderer = React.createRef();
    }

    componentDidMount() {
        Camera.bindInputs(this.state, this.setState.bind(this));
        Debug.bindInputs(this.state, this.setState.bind(this));
        Planning.bindInputs(this.state, this.setState.bind(this));
    }

    onFrame() {
        if (this.state.rendering.enabled) {
            Camera.onFrame(this.state, this.setState.bind(this));
            this.renderer.current.renderFrame();
        }
    }

    render() {
        const uiAspects = [
            Planning,
            Transport,
            LandUse,
            Debug,
        ];

        const uiAspectsRendered = uiAspects.map(aspect => aspect.render(this.state, this.setState.bind(this)));
        const { tools: settingsTools, windows: settingsWindows } = Settings.render(this.state, settingSpecs, this.setState.bind(this));

        const layers = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.layers || []), []);
        const interactables = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.interactables || []), []);
        const tools = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.tools || []), []).concat(settingsTools);
        const windows = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.windows || []), []).concat(settingsWindows);

        layers.sort((a, b) => a.renderOrder - b.renderOrder)

        const verticalFov = this.state.settings.camera.verticalFov * Math.PI;

        return EL("div", {
            style: { width: "100%", height: "100%" },
        },
            EL(ContainerDimensions, { style: { width: "100%", height: "100%", position: "relative" } }, ({ width, height }) => {
                const { eye, target, viewMatrix, perspectiveMatrix } = Camera.getMatrices(this.state, width, height);

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
                        retinaFactor: this.state.settings.rendering.retinaFactor,
                        viewMatrix, perspectiveMatrix,
                        clearColor: [...colors.grass, 1.0]
                    }),
                    EL(Stage, {
                        key: "stage",
                        interactables,
                        width, height,
                        eye, target, verticalFov,
                        style: { width, height, position: "absolute", top: 0, left: 0, '-webkit-overflow-scrolling': 'touch' },
                        onWheel: e => {
                            Camera.onWheel(e, this.state, this.setState.bind(this));
                            e.preventDefault();
                            return false;
                        },
                        onMouseMove: e => {
                            Camera.onMouseMove(e, this.state, this.setState.bind(this));
                        }
                    })
                ])
            })
        );
    }
}

window.cbclient = ReactDOM.render(EL(CityboundClient), document.getElementById('app'));

cityboundBrowser.start();