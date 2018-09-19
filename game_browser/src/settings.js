import React from 'react';
import { Button, Input, Slider, InputNumber, Switch, Select } from 'antd';
const Option = Select.Option;

const EL = React.createElement;

export function loadSettings(specs) {
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

const ALL_KEYS_NAMES = {
    'q': 'q',
    'w': 'w',
    'e': 'e',
    'r': 'r',
    't': 't',
    'y': 'y',
    'u': 'u',
    'i': 'i',
    'o': 'o',
    'p': 'p',
    'a': 'a',
    's': 's',
    'd': 'd',
    'f': 'f',
    'g': 'g',
    'h': 'h',
    'j': 'j',
    'k': 'k',
    'l': 'l',
    'z': 'z',
    'x': 'x',
    'c': 'c',
    'v': 'v',
    'b': 'b',
    'n': 'n',
    'm': 'm',
    'backspace': 'backspace',
    'tab': 'tab',
    'enter': 'enter',
    'shift': 'shift',
    'ctrl': 'ctrl',
    'command': 'command',
    'alt': /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'option' : 'alt',
    'capslock': 'capslock',
    'esc': 'esc',
    'space': 'space',
    'pageup': 'pageup',
    'pagedown': 'pagedown',
    'end': 'end',
    'home': 'home',
    'left': '←',
    'up': '↑',
    'right': '→',
    'down': '↓',
    'ins': 'ins',
    'del': 'del',
    'meta': 'meta',
    '1': '1',
    '2': '2',
    '3': '3',
    '4': '4',
    '5': '5',
    '6': '6',
    '7': '7',
    '8': '8',
    '9': '9',
    '0': '0',
    '`': '`',
    '~': '~',
    '!': '!',
    '@': '@',
    '#': '#',
    '$': '$',
    '%': '%',
    '^': '^',
    '&': '&',
    '*': '*',
    '(': '(',
    ')': ')',
    '_': '_',
    'plus': '+',
    '-': '-',
    '=': '=',
    '[': '[',
    ']': ']',
    '\\': '\\',
    '{': '{',
    '}': '}',
    '|': '|',
    ';': ';',
    '\'': '\'',
    ':': ':',
    '"': '"',
    ',': ',',
    '.': '.',
    '/': '/',
    '<': '<',
    '>': '>',
    '?': '?',
    'f1': 'f1',
    'f2': 'f2',
    'f3': 'f3',
    'f4': 'f4',
    'f5': 'f5',
    'f6': 'f6',
    'f7': 'f7',
    'f8': 'f8',
    'f9': 'f9',
    'f10': 'f10',
    'f11': 'f11',
    'f12': 'f12',
}

const ALL_KEYS_NAMES_ALTS = {
    'backspace': 'backspace',
    'tab': 'tab',
    'enter': 'return',
    'ctrl': 'control ^',
    'command': 'cmd',
    'esc': 'escape',
    'space': 'space',
    'pageup': 'page up',
    'pagedown': 'page down',
    'left': 'arrow left',
    'up': 'arrow up',
    'right': 'arrow right',
    'down': 'arrow down',
    'ins': 'insert',
    'del': 'delete',
    '1': 'one',
    '2': 'two',
    '3': 'three',
    '4': 'four',
    '5': 'five',
    '6': 'six',
    '7': 'seven',
    '8': 'eight',
    '9': 'nine',
    '0': 'zero',
    '`': 'backtick',
    '~': 'tilde',
    '!': 'exclamation mark',
    '@': 'at',
    '#': 'pound hash',
    '$': 'dollar',
    '%': 'percent',
    '^': 'circumflex hat',
    '&': 'ampersand and',
    '*': 'star times asterisk',
    '(': 'open parenthesis',
    ')': 'close parenthesis',
    '_': 'underscore',
    'plus': 'plus',
    '-': 'minus',
    '=': 'equals',
    '[': 'square bracket open',
    ']': 'square bracket close',
    '\\': 'backslash',
    '{': 'curly bracket open',
    '}': 'curly bracket close',
    '|': 'pipe',
    ';': 'semicolon',
    '\'': 'single quote',
    ':': 'colon',
    '"': 'double quote',
    ',': 'comma',
    '.': 'full stop dot',
    '/': 'slash',
    '<': 'less than',
    '>': 'greater than',
    '?': 'question mark',
}

export class Settings extends React.Component {
    constructor(props) {
        super(props);
        const { specs, setState } = props;

        this.closeSettings = () => {
            localStorage.clear();
            setState({ settings: loadSettings(specs) })
        }
    }

    shouldComponentUpdate(nextProps) {
        return this.props.currentSettings != nextProps.currentSettings || this.props.setState != nextProps.setState || this.props.specs != nextProps.specs
    }

    render() {
        const { currentSettings, setState, specs } = this.props;

        return EL("div", { key: "settings", className: "settings" }, [
            EL("p", {}, "If you change key assignments, you'll need to reload the tab"),
            EL(Button, {
                onClick: this.closeSettings
            }, "Reset all to defaults"),
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

                            const onChangeInput = event => this.onChange(event.target.value);

                            let inputEls = [];

                            if (typeof spec.default === "number") {
                                const marks = {
                                    [spec.min]: spec.min,
                                    [spec.max]: spec.max
                                };

                                if (spec.min && spec.min < 0) {
                                    marks[0] = 0;
                                }

                                inputEls = [
                                    EL(Slider, {
                                        value: currentSettings[aspect][key],
                                        onChange,
                                        marks,
                                        included: !spec.min || spec.min > 0,
                                        min: spec.min, max: spec.max, step: spec.step
                                    }),
                                    EL(InputNumber, {
                                        value: currentSettings[aspect][key],
                                        onChange,
                                        min: spec.min, max: spec.max, step: spec.step
                                    })
                                ]
                            } else if (typeof spec.default === "boolean") {
                                inputEls = [
                                    spec.falseDescription || "",
                                    EL(Switch, { checked: currentSettings[aspect][key], onChange }),
                                    spec.trueDescription || "",
                                ]
                            } else if (typeof spec.default === "string") {
                                inputEls = [
                                    EL(Input, { value: currentSettings[aspect][key], onChangeInput })
                                ]
                            } else if (spec.default.key) {
                                let splitKeys = currentSettings[aspect][key].key.split("+");
                                splitKeys = splitKeys.length === 1 && splitKeys[0] === "" ? [] : splitKeys;
                                inputEls = [
                                    EL(Select, {
                                        key: aspect + key,
                                        value: splitKeys,
                                        onChange: keys => onChange({ key: keys.join("+") }),
                                        optionFilterProp: 'children',
                                        mode: 'multiple',
                                        filterOption: (input, option) => option.props.value.toLowerCase().includes(input.toLowerCase())
                                            || (ALL_KEYS_NAMES_ALTS[option.props.value] || "").toLowerCase().includes(input.toLowerCase())
                                            || option.props.children.toLowerCase().includes(input.toLowerCase())
                                    },
                                        Object.keys(ALL_KEYS_NAMES).map(keyCode =>
                                            EL(Option, { key: keyCode, value: keyCode, }, ALL_KEYS_NAMES[keyCode])
                                        )
                                    )
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
        ]);
    }
}

