import colors from '../colors';
import update from 'immutability-helper';
import carMesh from './carMesh';

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

export function render(state, _setState) {

    const layers = [
        {
            decal: true,
            batches: [...state.transport.rendering.laneAsphaltGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: asphaltInstance
            }))
        },
        {
            decal: true,
            batches: [...state.transport.rendering.laneMarkerGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: roadMarkerInstance
            }))
        },
        {
            decal: true,
            batches: [...state.transport.rendering.laneMarkerGapGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: asphaltInstance
            }))
        },
        {
            decal: false,
            batches: [{
                mesh: state.transport.rendering.staticMeshes.car,
                instances: state.transport.rendering.carInstances
            }]
        }
    ];

    return { layers };
}