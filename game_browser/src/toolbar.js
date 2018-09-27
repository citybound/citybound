import React from 'react';

export function Toolbar(props) {
    return <div id={props.id} className="toolbar">{Object.keys(props.options).map(key => {
        const { description, color } = props.options[key];
        return <button id={key} key={key}
            alt={description}
            className={key == props.value ? "active" : ""}
            style={{ backgroundColor: color }}
            onClick={() => props.onChange(key)}
        />
    })}</div>
}