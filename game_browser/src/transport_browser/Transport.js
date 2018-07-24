import colors from '../colors';
import update from 'immutability-helper';

export const initialState = {
    rendering: {
        laneAsphalt: {},
        laneMarker: {},
        laneMarkerGap: {}
    }
};

export function render(state, _setState) {

    const layers = [
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneAsphalt).map(laneAsphalt => ({
                mesh: laneAsphalt,
                instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt])
            }))
        },
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneMarker).map(laneMarker => ({
                mesh: laneMarker,
                instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.roadMarker])
            }))
        },
        {
            decal: true,
            batches: Object.values(state.transport.rendering.laneMarkerGap).map(laneMarkerGap => ({
                mesh: laneMarkerGap,
                instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt])
            }))
        },
    ];

    return [layers, [], []];
}