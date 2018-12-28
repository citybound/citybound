import React from 'react';
import { Button, InputNumber } from 'antd';
import { fmtId } from '../browser_utils/Utils';

export const initialState = {
    show: false,
    planGridSettings: {
        n: 10,
        spacing: 200
    },
    spawnCarsSettings: {
        triesPerLane: 50
    },
    logLastEntry: 0,
    logTextStart: 0,
    logFirstEntry: 0,
    logEntries: [],
    logText: []
}

export const settingsSpec = {
    toggleDebugWindowKey: { default: { key: '.' }, description: "Toggle Debug Window" }
}

let refreshInterval = null;

export function Windows(props) {
    const { state, setState } = props;

    if (state.debug.show) {
        if (!refreshInterval) {
            refreshInterval = setInterval(() => cbRustBrowser.get_newest_log_messages(), 300);
        }
    } else {
        if (refreshInterval) {
            clearInterval(refreshInterval);
            refreshInterval = null;
        }
    }

    const connectionTolerance = window.cbNetworkSettings.acceptableTurnDistance + 3;
    const turnDiff = state.system.networkingTurns[0] - state.system.networkingTurns[1];
    const connectionIssue = state.system.networkingTurns[0] === 0
        ? "Connecting to server..."
        : (turnDiff > connectionTolerance
            ? "Catching up with server... " + turnDiff
            : (turnDiff < -connectionTolerance
                ? "Waiting for server... " + (-turnDiff)
                : ""));

    return state.debug.show && [
        <div key="debug" className="window debug">
            <h1>Debugging</h1>
            <details>
                <summary>Debug Actions</summary>
                {state.planning.currentProject
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
                                debug: { planGridSettings: { spacing: { $set: spacing } } }
                            }))}
                            step={10.0}
                            min={0} />

                        <Button
                            onClick={() => cbRustBrowser.plan_grid(
                                state.planning.currentProject,
                                state.debug.planGridSettings.n,
                                state.debug.planGridSettings.spacing
                            )
                            }>Plan grid</Button>
                    </div>
                    : <div>(open a project to plan a grid)</div>
                }
                <div key="carSpawning">
                    Cars per lane (tries)
                <InputNumber
                        value={state.debug.spawnCarsSettings.triesPerLane}
                        onChange={(triesPerLane) => setState(oldState => update(oldState, {
                            debug: { spawnCarsSettings: { triesPerLane: { $set: triesPerLane } } }
                        }))}
                        min={1} /> <Button
                            onClick={() => cbRustBrowser.spawn_cars(
                                state.debug.spawnCarsSettings.triesPerLane
                            )}>Spawn cars</Button>
                </div>
                <div key="rendering">
                    <Button
                        onClick={() => setState(
                            oldState => update(oldState, { rendering: { enabled: { $apply: e => !e } } })
                        )}>{state.rendering.enabled ? "Disable rendering" : "Enable rendering"}</Button>
                </div>
            </details>
            <details>
                <summary>Networking</summary>
                <div>{Object.keys(state.system.networkingTurns).map(machine =>
                    <div>{machine}: {state.system.networkingTurns[machine]}</div>
                )}</div>
                <div className="scrollableLog">{Object.keys(state.system.queueLengths).map(actor =>
                    <div>{actor}: {state.system.queueLengths[actor]}</div>
                )}</div>
                <div className="scrollableLog">{Object.keys(state.system.messageStats).map(message =>
                    <div>{message}: {state.system.messageStats[message]}</div>
                )}</div>
            </details>
            <details>
                <summary>Simulation Log</summary>
                <div className="scrollableLog">{state.debug.logEntries.map((entry, i) => {
                    let ts = state.debug.logTextStart;
                    let topic = state.debug.logText.slice(entry.topic_start - ts, entry.message_start - ts);
                    let message = state.debug.logText.slice(entry.message_start - ts, entry.message_start + entry.message_len - ts);
                    return <div key={i} className={entry.level}>{state.debug.logFirstEntry + i} [{topic}] {fmtId(entry.from)}: {message}</div>
                }
                )}</div>
            </details>
        </div>,
        connectionIssue && <div className="window connection">{connectionIssue}</div>];
}

export function bindInputs(state, setState) {
    const inputActions = {
        "toggleDebugView": () => setState(oldState => update(oldState, {
            debug: { show: { $apply: b => !b } }
        })),
    }

    Mousetrap.bind(state.settings.debug.toggleDebugWindowKey.key, inputActions["toggleDebugView"]);
}