import React from 'react';
import * as cityboundBrowser from '../../Cargo.toml';
import { Button, InputNumber } from 'antd';

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

export const settingsSpec = {
    toggleDebugWindowKey: { default: { key: '.' }, description: "Toggle Debug Window" }
}

export function render(state, setState) {
    const connectionTolerance = window.cbNetworkSettings.acceptableTurnDistance + 3;
    const turnDiff = state.system.networkingTurns[0] - state.system.networkingTurns[1];
    const connectionIssue = state.system.networkingTurns[0] === 0
        ? "Connecting to server..."
        : (turnDiff > connectionTolerance
            ? "Catching up with server... " + turnDiff
            : (turnDiff < -connectionTolerance
                ? "Waiting for server... " + (-turnDiff)
                : ""));

    const windows = [
        state.debug.show && <div key="debug" className="window debug">
            <h1>Debugging</h1>
            {state.planning.currentProposal
                ? <div key="gridPlanning">
                    Grid size
                    <InputNumber
                        value={state.debug.planGridSettings.n}
                        onChange={(n) => setState(oldState => update(oldState, {
                            debug: { planGridSettings: { n: { $set: n } } }
                        }))}
                        min={0} />
                    Spacing
                    <InputNumber
                        value={state.debug.planGridSettings.spacing}
                        onChange={(spacing) => setState(oldState => update(oldState, {
                            debug: { planGridSettings: { spacing: { $set: n } } }
                        }))}
                        step={10.0}
                        min={0} />

                    <Button
                        onClick={() => cityboundBrowser.plan_grid(
                            state.planning.currentProposal,
                            state.debug.planGridSettings.n,
                            state.debug.planGridSettings.spacing
                        )
                        }>Plan grid</Button>
                </div>
                : <div>(open a proposal to plan a grid)</div>
            }
            <div key="carSpawning">
                Cars per lane (tries)
                <InputNumber
                    value={state.debug.spawnCarsSettings.triesPerLane}
                    onChange={(triesPerLane) => setState(oldState => update(oldState, {
                        debug: { spawnCarsSettings: { triesPerLane: { $set: triesPerLane } } }
                    }))}
                    min={1} /> <Button
                        onClick={() => cityboundBrowser.spawn_cars(
                            state.debug.spawnCarsSettings.triesPerLane
                        )}>Spawn cars</Button>
            </div>
            <div key="rendering">
                <Button
                    onClick={() => setState(
                        oldState => update(oldState, { rendering: { enabled: { $apply: e => !e } } })
                    )}>{state.rendering.enabled ? "Disable rendering" : "Enable rendering"}</Button>
            </div>
            <div>{Object.keys(state.system.networkingTurns).map(machine =>
                <div>{machine}: {state.system.networkingTurns[machine]}</div>
            )}</div>
            <div>{Object.keys(state.system.queueLengths).map(actor =>
                <div>{actor}: {state.system.queueLengths[actor]}</div>
            )}</div>
            <div>{Object.keys(state.system.messageStats).map(message =>
                <div>{message}: {state.system.messageStats[message]}</div>
            )}</div>
        </div>,
        connectionIssue && <div className="window connection">{connectionIssue}</div>
    ];

    return { windows }
}

export function bindInputs(state, setState) {
    const inputActions = {
        "toggleDebugView": () => setState(oldState => update(oldState, {
            debug: { show: { $apply: b => !b } }
        })),
    }

    Mousetrap.bind(state.settings.debug.toggleDebugWindowKey.key, inputActions["toggleDebugView"]);
}