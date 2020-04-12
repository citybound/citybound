import * as React from 'react';
import renderOrder from '../../renderOrder';
import { RenderLayer } from '../../browser_utils/Utils';
import { SharedState } from '../../citybound';
import colors from '../../colors';

// TODO: share constants with Rust somehow
export const LANE_DISTANCE = 0.8 * 3.9;

const destructedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.destructedAsphalt]);
const plannedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt]);
const plannedRoadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedRoadMarker]);

export function RoadPlanningLayers({ state }: {
    state: SharedState;
}) {
    const { lanesToConstructGroups, lanesToConstructMarkerGroups, lanesToConstructMarkerGapsGroups, } = state.planning.rendering.currentPreview;
    return <>
        <RenderLayer renderOrder={renderOrder.addedGesturesAsphalt} decal={true} batches={[...lanesToConstructGroups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: plannedAsphaltInstance
        }))} />
        <RenderLayer renderOrder={renderOrder.addedGesturesMarker} decal={true} batches={[...lanesToConstructMarkerGroups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: plannedRoadMarkerInstance
        }))} />
        <RenderLayer renderOrder={renderOrder.addedGesturesMarkerGap} decal={true} batches={[...lanesToConstructMarkerGapsGroups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: plannedAsphaltInstance
        }))} />
    </>;
}
