export function fmtId(id) {
    let parts = id.split(/[_\.@]/g);
    let type = parseInt(parts[0], 16);
    let fullType = window.cbTypeIdMapping[type];
    let typeSplit = fullType.split("::");
    let shortType = typeSplit[typeSplit.length - 1];
    let instance = parseInt(parts[1], 16);
    let version = parts[2];
    let machine = parts[3];

    return shortType + " #" + instance;// + "v" + version + "@" + machine;
}

import React from 'react';

// TODO: move this to Monet and Stage respectively

export const RenderContext = React.createContext("render");

export function RenderLayer(props) {
    return <RenderContext.Consumer>
        {renderLayers => {
            renderLayers.push({ ...props });
            return null;
        }}
    </RenderContext.Consumer>
}

export const Interactive3DContext = React.createContext("interactive3D");

export function Interactive3DShape(props) {
    return <Interactive3DContext.Consumer>
        {interactiveShapes => {
            interactiveShapes.push({ ...props });
            return null;
        }}
    </Interactive3DContext.Consumer>
}
