import { RenderLayer } from "../browser_utils/Utils";
import colors from '../colors';
import renderOrder from '../renderOrder';

export const initialState = {
    trunkInstances: [],
    smallCanopyInstances: [],
    mediumCanopyInstances: [],
    largeCanopyInstances: []
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
            mesh: state.vegetation.smallCanopyMesh,
            instances: state.vegetation.smallCanopyInstances
        },{
            mesh: state.vegetation.mediumCanopyMesh,
            instances: state.vegetation.mediumCanopyInstances
        },{
            mesh: state.vegetation.largeCanopyMesh,
            instances: state.vegetation.largeCanopyInstances
        }]} />;
}