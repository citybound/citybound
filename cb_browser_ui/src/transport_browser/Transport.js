import colors from '../colors';
import renderOrder from '../renderOrder';
import carMesh from './carMesh';
import { RenderLayer } from '../browser_utils/Utils';
import React from 'react';

export const initialState = {
    rendering: {
        staticMeshes: {
            car: carMesh
        },
        laneAsphaltGroups: new Map(),
        laneMarkerGroups: new Map(),
        laneMarkerGapGroups: new Map(),
        carInstances: []
    }
};

const asphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt]);
const roadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.roadMarker]);

export function Layers(props) {
    const { state } = props

    return [
        <RenderLayer
            renderOrder={renderOrder.asphalt}
            decal={true}
            batches={[...state.transport.rendering.laneAsphaltGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: asphaltInstance
            }))} />,
        <RenderLayer
            renderOrder={renderOrder.asphaltMarker}
            decal={true}
            batches={[...state.transport.rendering.laneMarkerGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: roadMarkerInstance
            }))} />,
        <RenderLayer
            renderOrder={renderOrder.asphaltMarkerGap}
            decal={true}
            batches={[...state.transport.rendering.laneMarkerGapGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: asphaltInstance
            }))} />,
        <RenderLayer
            renderOrder={renderOrder.cars}
            decal={false}
            batches={[{
                mesh: state.transport.rendering.staticMeshes.car,
                instances: state.transport.rendering.carInstances
            }]} />
    ];
}