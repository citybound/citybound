import * as React from 'react';
import renderOrder from '../renderOrder';
import { RenderLayer } from '../browser_utils/Utils';
import { SharedState } from '../citybound';
import { shadersForLandUses } from './stripedShaders';
import colors from '../colors';

export const LAND_USES = [
    "Residential",
    "Commercial",
    "Industrial",
    "Agricultural",
    "Recreational",
    "Administrative",
];

const landUseInstances = new Map(LAND_USES.map(landUse => [landUse, new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[landUse]])]));
const buildingOutlinesInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.buildingOutlines]);

export function ZonePlanningLayers({ state }: {
    state: SharedState;
}) {
    const { zoneGroups, zoneOutlineGroups, buildingOutlinesGroup } = state.planning.rendering.currentPreview;
    return <>
        {[...zoneGroups.entries()].map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZones} decal={true} batches={[...groups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: landUseInstances.get(landUse)
        }))} />)}
        {[...zoneGroups.entries()].reverse().map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZonesStipple} decal={true} shader={shadersForLandUses[landUse]} batches={[...groups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: landUseInstances.get(landUse)
        }))} />)}
        {[...zoneOutlineGroups.entries()].map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZonesOutlines} decal={true} batches={[...groups.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: landUseInstances.get(landUse)
        }))} />)}
        <RenderLayer renderOrder={renderOrder.buildingOutlines} decal={true} batches={[...buildingOutlinesGroup.values()].map(groupMesh => ({
            mesh: groupMesh,
            instances: buildingOutlinesInstance
        }))} />
    </>;
}
