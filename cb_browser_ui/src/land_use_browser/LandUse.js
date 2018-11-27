import colors from '../colors';
import renderOrder from '../renderOrder';
import { RenderLayer } from "../browser_utils/Utils";

const MATERIALS = ["WhiteWall", "TiledRoof", "FlatRoof", "FieldWheat", "FieldRows", "FieldPlant", "FieldMeadow"];
const initialRenderingState = {};
const materialInstances = {};

for (let material of MATERIALS) {
    initialRenderingState[material] = {}
    materialInstances[material] = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[material]]);
}

export const initialState = {
    rendering: initialRenderingState
}

export function render(state, _setState) {
    return {}
}

import React from 'react';

export function Layers(props) {
    let { state } = props;

    return MATERIALS.map(material =>
        <RenderLayer
            key={material}
            decal={false}
            renderOrder={material.startsWith("Field") ? renderOrder.buildingGround : renderOrder.building3D}
            batches={Object.values(state.landUse.rendering[material]).map(housePart => ({
                mesh: housePart,
                instances: materialInstances[material]
            }))} />
    );
}