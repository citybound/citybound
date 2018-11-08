import colors from '../colors';
import renderOrder from '../renderOrder';

export const initialState = {
    rendering: {
        wall: {},
        flatRoof: {},
        brickRoof: {},
        field: {}
    }
}

const materialInstances = {};

for (let material of ["wall", "flatRoof", "brickRoof", "field"]) {
    materialInstances[material] = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[material]]);
}

export function render(state, _setState) {

    const layers = ["wall", "flatRoof", "brickRoof", "field"].map(material =>
        ({
            decal: false,
            renderOrder: material == "field" ? renderOrder.buildingGround : renderOrder.building3D,
            batches: Object.values(state.landUse.rendering[material]).map(housePart => ({
                mesh: housePart,
                instances: materialInstances[material]
            }))
        })
    );

    return { layers };
}