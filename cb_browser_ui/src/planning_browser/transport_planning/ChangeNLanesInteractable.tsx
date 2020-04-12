import * as React from 'react';
import { Interactive3DShape, RenderLayer } from '../../browser_utils/Utils';
import { vec2 } from 'gl-matrix';
import { SharedState } from '../../citybound';
import renderOrder from '../../renderOrder';
import colors from '../../colors';
import { useState } from 'react';
import { LANE_DISTANCE } from './RoadPlanningLayers';

function setNLanes(projectId, gestureId, nLanesForward, nLanesBackward, doneChanging) {
    window.cbRustBrowser.set_n_lanes(projectId, gestureId, nLanesForward, nLanesBackward, doneChanging);
}

export function ChangeNLanesInteractable({ gestureId, centerLine, nLanesBackward, nLanesForward, state, currentProject }: {
    gestureId: string;
    centerLine: any;
    nLanesBackward: any;
    nLanesForward: any;
    state: SharedState
    currentProject: string;
}): any {
    const [hoverPoint, setHoverPoint] = useState<{ position: [number, number], direction: [number, number] } | null>(null);

    return <>
        {hoverPoint && <RenderLayer renderOrder={renderOrder.gestureInteractables} decal={true} batches={[
            {
                mesh: state.planning.rendering.staticMeshes.GestureChangeNLanes,
                instances: new Float32Array([
                    hoverPoint.position[0],
                    hoverPoint.position[1],
                    0.0,
                    hoverPoint.direction[0],
                    hoverPoint.direction[1],
                    ...colors.controlPointCurrentProject
                ])
            }
        ]} />}

        <Interactive3DShape id={gestureId + "changeNLanes"} shape={{
            type: "path",
            path: centerLine,
            maxDistanceLeft: LANE_DISTANCE * nLanesBackward + 2,
            maxDistanceRight: LANE_DISTANCE * nLanesForward + 2,
        }} zIndex={2} cursorHover="ew-resize" cursorActive="ew-resize" onEvent={e => {
            if (e.drag) {
                if (e.drag.end || e.drag.now) {
                    const position = e.drag.end || e.drag.now;
                    const { projectedPosition, direction } = e.drag;
                    const orthogonalRightDirection = [direction[1], -direction[0]];
                    const vector = vec2.sub(vec2.create(), position, projectedPosition);
                    const orthogonalDistance = vec2.dot(vector, orthogonalRightDirection);
                    let newNLanesForward = nLanesForward;
                    let newNLanesBackward = nLanesBackward;

                    if (orthogonalDistance > 0.0) {
                        newNLanesForward = Math.max(0.0, Math.round(orthogonalDistance / 3.0));
                    }
                    else {
                        newNLanesBackward = Math.max(0.0, Math.round(-orthogonalDistance / 3.0));
                    }

                    setNLanes(currentProject, gestureId, newNLanesForward, newNLanesBackward, e.drag.end ? true : false);
                }
            }
            if (e.hover) {
                if (e.hover.end) {
                    setHoverPoint(null);
                }
                else if (e.hover.now) {
                    const position = e.hover.now;
                    const { projectedPosition, direction } = e.hover;
                    const orthogonalRightDirection = [direction[1], -direction[0]];
                    const vector = vec2.sub(vec2.create(), position, projectedPosition);
                    const orthogonalDistance = vec2.dot(vector, orthogonalRightDirection);
                    let shiftedPoint;
                    if (orthogonalDistance > 0.0) {
                        shiftedPoint = vec2.scaleAndAdd(vec2.create(), e.hover.projectedPosition, orthogonalRightDirection, LANE_DISTANCE * nLanesForward);
                    }
                    else {
                        shiftedPoint = vec2.scaleAndAdd(vec2.create(), e.hover.projectedPosition, orthogonalRightDirection, -LANE_DISTANCE * nLanesBackward);
                    }
                    setHoverPoint({ position: shiftedPoint, direction });
                }
            }
        }} />
    </>;
}
