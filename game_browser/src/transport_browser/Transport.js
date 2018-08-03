import colors from '../colors';
import update from 'immutability-helper';
import carMesh from './carMesh';

export const initialState = {
    rendering: {
        staticMeshes: {
            car: carMesh
        },
        laneAsphalt: {},
        laneMarker: {},
        laneMarkerGap: {},
        carInstances: []
    }
};

const asphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt]);
const roadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.roadMarker]);

export function render(state, _setState) {

    const layers = [
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneAsphalt).map(laneAsphalt => ({
                mesh: laneAsphalt,
                instances: asphaltInstance
            }))
        },
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneMarker).map(laneMarker => ({
                mesh: laneMarker,
                instances: roadMarkerInstance
            }))
        },
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneMarkerGap).map(laneMarkerGap => ({
                mesh: laneMarkerGap,
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

    return [layers, [], []];
}