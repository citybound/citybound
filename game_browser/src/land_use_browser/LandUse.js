import colors from '../colors';

export const initialState = {
    rendering: {
        wall: {},
        flatRoof: {},
        brickRoof: {},
        field: {}
    }
}

const materialInstances = ["wall", "flatRoof", "brickRoof", "field"].map(material =>
    new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[material]])
);

export function render(state, _setState) {

    const layers = ["wall", "flatRoof", "brickRoof", "field"].map(material =>
        ({
            decal: false,
            batches: Object.values(state.landUse.rendering[material]).map(housePart => ({
                mesh: housePart,
                instances: materialInstances[material]
            }))
        })
    );

    return [layers, [], []];
}