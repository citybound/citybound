import colors from '../colors';
import renderOrder from '../renderOrder';
import { RenderLayer } from "../browser_utils/Utils";
import * as propMeshes from './propMeshes';

const MATERIALS = ["WhiteWall", "TiledRoof", "FlatRoof", "FieldWheat", "FieldRows", "FieldPlant", "FieldMeadow", "WoodenFence", "MetalFence", "LotAsphalt"];
const PROP_TYPES = ["SmallWindow", "ShopWindowGlass", "ShopWindowBanner", "NarrowDoor", "WideDoor"];

const initialRenderingState = {
    buildingMeshes: {},
    buildingProps: {},
    propMeshes: {
        SmallWindow: propMeshes.smallWindow,
        NarrowDoor: propMeshes.narrowDoor,
        WideDoor: propMeshes.wideDoor,
        ShopWindowBanner: propMeshes.shopWindowBanner,
        ShopWindowGlass: propMeshes.shopWindowGlass,
    }
};
const materialInstances = {};

for (let material of MATERIALS) {
    initialRenderingState.buildingMeshes[material] = {};
    materialInstances[material] = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[material]]);
}

for (let propType of PROP_TYPES) {
    initialRenderingState.buildingProps[propType] = {};
}

export const initialState = {
    rendering: initialRenderingState
}

import React from 'react';

export function Layers(props) {
    let { state } = props;

    return MATERIALS.map(material =>
        <RenderLayer
            key={material}
            decal={false}
            renderOrder={material.startsWith("Field") ? renderOrder.buildingGround : renderOrder.building3D}
            batches={Object.values(state.landUse.rendering.buildingMeshes[material]).map(buildingPart => ({
                mesh: buildingPart,
                instances: materialInstances[material]
            }))} />
    ).concat(PROP_TYPES.map(propType =>
        <RenderLayer
            key={propType}
            decal={false}
            renderOrder={renderOrder.building3D}
            batches={[{
                mesh: state.landUse.rendering.propMeshes[propType],
                instances: new Float32Array(Object.values(state.landUse.rendering.buildingProps[propType])
                    .reduce((allPropInstances, buildingPropInstances) => allPropInstances.concat(buildingPropInstances), []))
            }]} />
    ));
}