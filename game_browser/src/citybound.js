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
    }).catch(() => {
        document.getElementById("errorsloading").className = "loaded";
        el.insertAdjacentHTML("beforeend", `<h2>${prefix}: ${error.message}</h2>`);
        el.insertAdjacentHTML("beforeend", 'failed to gather error origin :(');
    });
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
import * as Households from './households_browser/Households';
import * as Simulation from './simulation_browser/Simulation';
import * as Debug from './debug/Debug';
import * as Settings from './settings';
import * as Menu from './menu';
import Stage from './stage/Stage';
import colors from './colors';

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
            households: Households.initialState,
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

            menu: Menu.initalState,
            settings: Settings.loadSettings(settingSpecs)
        }

        this.renderer = React.createRef();
        this.boundSetState = this.setState.bind(this);
    }

    componentDidMount() {
        Camera.bindInputs(this.state, this.boundSetState);
        Debug.bindInputs(this.state, this.boundSetState);
        Planning.bindInputs(this.state, this.boundSetState);
    }

    onFrame() {
        if (this.state.rendering.enabled) {
            Camera.onFrame(this.state, this.boundSetState);
            this.renderer.current.renderFrame();
        }
    }

    render() {
        const uiAspects = [
            Planning,
            Transport,
            LandUse,
            Households,
            Debug,
            Simulation,
        ];

        const uiAspectsRendered = uiAspects.map(aspect => aspect.render(this.state, this.boundSetState));
        const { tools: menuTools, windows: menuWindows } = Menu.render(this.state, settingSpecs, this.boundSetState);

        const layers = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.layers || []), []);
        const interactables = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.interactables || []), []);
        const tools = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.tools || []), []).concat(menuTools);
        const windows = uiAspectsRendered.reduce((acc, aspect) => acc.concat(aspect.windows || []), []).concat(menuWindows);

        layers.sort((a, b) => a.renderOrder - b.renderOrder)

        const verticalFov = this.state.settings.camera.verticalFov * Math.PI;

        return <div style={{ width: "100%", height: "100%" }}>
            <ContainerDimensions style={{ width: "100%", height: "100%", position: "relative" }}>{({ width, height }) => {
                const { eye, target, viewMatrix, perspectiveMatrix } = Camera.getMatrices(this.state, width, height);

                return <div style={{ width, height }}>
                    <div key="ui2dTools" className="ui2dTools">
                        {tools}
                    </div>
                    <div key="ui2d" className="ui2d">
                        {windows}
                    </div>
                    <Monet key="canvas" ref={this.renderer}
                        retinaFactor={this.state.settings.rendering.retinaFactor}
                        clearColor={[...colors.grass, 1.0]}
                        {... { layers, width, height, viewMatrix, perspectiveMatrix }} />
                    <Stage key="stage"
                        style={{ width, height, position: "absolute", top: 0, left: 0 }}
                        onWheel={e => {
                            Camera.onWheel(e, this.state, this.boundSetState);
                            e.preventDefault();
                            return false;
                        }}
                        onMouseMove={e => {
                            Camera.onMouseMove(e, this.state, this.boundSetState);
                        }}
                        {...{ interactables, width, height, eye, target, verticalFov }}
                    />
                </div>
            }}</ContainerDimensions>
        </div>;
    }
}

window.cbclient = ReactDOM.render(<CityboundClient />, document.getElementById('app'));

cityboundBrowser.start();