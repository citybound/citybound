import React from 'react';
import * as cityboundBrowser from '../../Cargo.toml';
import { Button, InputNumber } from 'antd';
const EL = React.createElement;

export const initialState = {
    show: false,
    planGridSettings: {
        n: 10,
        spacing: 200
    },
    spawnCarsSettings: {
        triesPerLane: 50
    }
}

export function render(state, setState) {
    const elements = state.debug.show ? [EL("div", { key: "debug", className: "window debug" }, [
        EL("h1", {}, "Debugging"),
        ...(state.planning.currentProposal ? [
            EL("div", { key: "gridPlanning" }, [
                "Grid size ",
                EL(InputNumber, {
                    value: state.debug.planGridSettings.n,
                    onChange: (n) => setState(oldState => update(oldState, {
                        debug: { planGridSettings: { n: { $set: n } } }
                    })),
                    min: 0
                }),
                " Spacing ",
                EL(InputNumber, {
                    value: state.debug.planGridSettings.spacing,
                    onChange: (spacing) => setState(oldState => update(oldState, {
                        debug: { planGridSettings: { spacing: { $set: n } } }
                    })),
                    step: 10.0,
                    min: 0
                }),
                " ",
                EL(Button, {
                    onClick: () => cityboundBrowser.plan_grid(
                        state.planning.currentProposal,
                        state.debug.planGridSettings.n,
                        state.debug.planGridSettings.spacing
                    )
                }, "Plan grid")
            ]),
        ] : []),
        EL("div", { key: "carSpawning" }, [
            "Cars per lane (tries) ",
            EL(InputNumber, {
                value: state.debug.spawnCarsSettings.triesPerLane,
                onChange: (triesPerLane) => setState(oldState => update(oldState, {
                    debug: { spawnCarsSettings: { triesPerLane: { $set: triesPerLane } } }
                })),
                min: 1
            }),
            " ",
            EL(Button, {
                onClick: () => cityboundBrowser.spawn_cars(
                    state.debug.spawnCarsSettings.triesPerLane
                )
            }, "Spawn cars")
        ]),
        EL("div", { key: "rendering" }, [
            EL(Button, {
                onClick: () => setState(
                    oldState => update(oldState, { rendering: { enabled: { $apply: e => !e } } })
                )
            }, state.rendering.enabled ? "Disable rendering" : "Enable rendering")
        ]),
        EL("div", {}, Object.keys(state.system.networkingTurns).map(machine =>
            EL("div", {}, machine + ": " + state.system.networkingTurns[machine])
        )),
        EL("div", {}, Object.keys(state.system.queueLengths).map(actor =>
            EL("div", {}, actor + ": " + state.system.queueLengths[actor])
        )),
        EL("div", {}, Object.keys(state.system.messageStats).map(message =>
            EL("div", {}, message + ": " + state.system.messageStats[message])
        )),
    ])] : [];

    return [[], [], elements]
}