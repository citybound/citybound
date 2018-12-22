import { RenderLayer } from "../browser_utils/Utils";
import colors from '../colors';
import renderOrder from '../renderOrder';

export const initialState = {
    trunkInstances: [],
    canopyInstances: []
}

import React from 'react';

export function Layers(props) {
    let { state } = props;

    return <RenderLayer
        key="vegetation"
        decal={false}
        renderOrder={renderOrder.vegetation}
        batches={[{
            mesh: state.vegetation.trunkMesh,
            instances: state.vegetation.trunkInstances
        }, {
            mesh: state.vegetation.canopyMesh,
            instances: state.vegetation.canopyInstances
        }]} />;
}