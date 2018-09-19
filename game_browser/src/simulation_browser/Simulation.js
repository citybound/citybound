import React from 'react';
const EL = React.createElement;
import { Slider } from 'antd';
import update from 'immutability-helper';
import * as cityboundBrowser from '../../Cargo.toml';

export const initialState = {
    ticks: 0,
    time: [0, 0],
    speed: 1
}

export function render(state, setState) {
    const windows = EL("div", { className: "sim-time" },
        [(state.simulation.time[0] + "").padStart(2, "0"),
        EL("span", { className: "sim-time-colon" }, ":"),
        (state.simulation.time[1] + "").padStart(2, "0"),
        EL(Slider, {
            className: "sim-speed",
            value: state.simulation.speed == 0 ? 0 : Math.log2(state.simulation.speed) + 1,
            min: 0, max: 6,
            marks: { 0: "||", 1: "1x", 3: "4x", 6: "32x" },
            onChange: newSpeedLog => {
                const newSpeed = newSpeedLog == 0 ? 0 : Math.pow(2, newSpeedLog - 1);
                cityboundBrowser.set_sim_speed(newSpeed);
                setState(oldState => update(oldState, { simulation: { speed: { $set: newSpeed } } }));
            },
            tipFormatter: speed => speed ? `Speed: ${Math.pow(2, speed - 1)}x` : "Pause"
        })]
    );

    return { windows }
}