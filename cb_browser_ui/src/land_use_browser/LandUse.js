import colors from '../colors';
import renderOrder from '../renderOrder';

const MATERIALS = ["WhiteWall", "TiledRoof", "FlatRoof", "Field"];
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

    const layers = MATERIALS.map(material =>
        ({
            decal: false,
            renderOrder: material == "Field" ? renderOrder.buildingGround : renderOrder.building3D,
            batches: Object.values(state.landUse.rendering[material]).map(housePart => ({
                mesh: housePart,
                instances: materialInstances[material]
            }))
        })
    );

    return { layers };
}