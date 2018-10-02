import React from 'react';
import { Tooltip } from 'antd';

export function Toolbar(props) {
    return <div id={props.id} className="toolbar">{Object.keys(props.options).filter(key => props.options[key]).map(key => {
        const { description, color, disabled } = props.options[key];
        return <Tooltip title={description} arrowPointAtCenter={true} mouseEnterDelay={0.6} mouseLeaveDelay={0}>
            <button id={key} key={key}
                className={(key == props.value ? "active" : "") + (disabled ? " disabled" : "")}
                onClick={!disabled && (() => props.onChange(key))}
            ><div style={{ backgroundColor: color }} className="button-icon"></div></button>
        </Tooltip>
    })}</div>
}