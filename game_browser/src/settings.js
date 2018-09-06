import { makeToolbar } from './toolbar';
import React from 'react';
import { Input, Slider, InputNumber, Switch } from 'antd';

const EL = React.createElement;

export default function loadSettings(specs) {
    const loadedSettings = {};

    for (let aspect of Object.keys(specs)) {
        loadedSettings[aspect] = {};

        for (let key of Object.keys(specs[aspect])) {
            const loaded = localStorage["cbsettings:" + aspect + ":" + key];
            loadedSettings[aspect][key] = loaded ? JSON.parse(loaded) : specs[aspect][key].default;
        }
    }

    return loadedSettings;
}

export function render(state, specs, setState) {
    const setUiMode = uiMode => {
        let updateOp = { uiMode: { $set: uiMode } };
        setState(oldState => update(oldState, updateOp))
    };

    let tools = makeToolbar("settings-toolbar", ["Settings"], "main", state.uiMode, setUiMode);
    let windows = state.uiMode == "main/Settings" ? [
        EL("div", { key: "debug", className: "window settings" }, [
            EL("h1", {}, "Menu"),
            EL("p", {}, "If you change key assignments, you'll need to reload the tab"),
            ...Object.keys(specs).map(aspect =>
                [
                    EL("h2", {}, aspect),
                    EL("div", { className: "form" },
                        ...Object.keys(specs[aspect]).map(key => {
                            let spec = specs[aspect][key];
                            const onChange = newValue => {
                                setState(oldState => update(oldState, {
                                    settings: {
                                        [aspect]: {
                                            [key]: { $set: newValue }
                                        }
                                    }
                                }));

                                localStorage["cbsettings:" + aspect + ":" + key] = JSON.stringify(newValue);
                            };

                            const onChangeInput = event => onChange(event.target.value);

                            let inputEls = [];

                            if (typeof spec.default === "number") {
                                const marks = {
                                    [spec.min]: spec.min,
                                    0: 0,
                                    [spec.max]: spec.max
                                };

                                inputEls = [
                                    EL(Slider, {
                                        value: state.settings[aspect][key],
                                        onChange,
                                        marks,
                                        included: !spec.min || spec.min > 0,
                                        min: spec.min, max: spec.max, step: spec.step
                                    }),
                                    EL(InputNumber, {
                                        value: state.settings[aspect][key],
                                        onChange,
                                        min: spec.min, max: spec.max, step: spec.step
                                    })
                                ]
                            } else if (typeof spec.default === "boolean") {
                                inputEls = [
                                    EL(Switch, { checked: state.settings[aspect][key], onChange })
                                ]
                            } else if (typeof spec.default === "string") {
                                inputEls = [
                                    EL(Input, { value: state.settings[aspect][key], onChange: onChangeInput })
                                ]
                            }

                            return EL("div", { className: "formItem" }, [
                                EL("label", {}, spec.description || key),
                                ...inputEls
                            ])
                        })
                    )
                ]
            )
        ])
    ] : [];
    return { tools, windows };
}