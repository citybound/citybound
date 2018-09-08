import React from 'react';
const EL = React.createElement;

export function Toolbar(props) {
    return EL("div", { id: props.id, className: "toolbar" }, Object.keys(props.options).map(key => {
        const { description, color } = props.options[key];
        return EL("button", {
            id: key,
            key: key,
            alt: description,
            className: key == props.value ? "active" : "",
            style: { backgroundColor: color },
            onClick: () => props.onChange(key),
        });
    }));
}