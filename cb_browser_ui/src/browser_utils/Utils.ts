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

import React, { useContext, useEffect } from 'react';

// TODO: move this to Monet and Stage respectively

export const RenderContext = React.createContext([]);

export function RenderLayer(props) {
    const renderLayers = useContext(RenderContext);
    renderLayers.push(props);
    return null;
}


export function useCollectInstances() {
    const instances = [];
    const InstancesContext = React.createContext<number[]>([]);
    const Instance = (props: { position: [number, number, number], direction?: [number, number], color?: [number, number, number] }) => {
        const inst = useContext(InstancesContext);
        inst.push.apply(instances, [
            props.position[0],
            props.position[1],
            props.position[2],
            props.direction ? props.direction[0] : 1,
            props.direction ? props.direction[1] : 0,
            props.color ? props.color[0] : 0,
            props.color ? props.color[1] : 0,
            props.color ? props.color[2] : 0
        ]);
        return null;
    }

    return [instances, InstancesContext.Provider, Instance];
}

export const Interactive3DContext = React.createContext({});

export function Interactive3DShape(props) {
    const interactiveShapes = useContext(Interactive3DContext);
    interactiveShapes.push(props);
    return null;
}

export function useInputBinding(bindings: { [key: string]: () => void }, deps: any[] = []) {
    useEffect(() => {
        for (let [key, action] of Object.entries(bindings)) {
            Mousetrap.bind(key, action);
        }

        return () => {
            for (let [key, action] of Object.entries(bindings)) {
                Mousetrap.unbind(key, action);
            }
        }
    }, (Object.keys(bindings) as any[]).concat(Object.values(bindings)).concat(deps));
}

export const SettingsContext = React.createContext({});

export function useSettings() {
    return useContext(SettingsContext);
}