import * as React from 'react';
import { Interactive3DShape, RenderLayer } from '../../browser_utils/Utils';
import { SharedState } from '../../citybound';
import renderOrder from '../../renderOrder';
import { useState } from 'react';
import colors from '../../colors';
import { LANE_DISTANCE } from './RoadPlanningLayers';

export function splitGesture(projectId, gestureId, splitAt, doneSplitting) {
    window.cbRustBrowser.split_gesture(projectId, gestureId, [splitAt[0], splitAt[1]], doneSplitting);
}

export function SplitControlPointInteractable({ gestureId, centerLine, nLanesBackward, nLanesForward, state, currentProject }: {
    gestureId: string;
    centerLine: any;
    nLanesBackward: any;
    nLanesForward: any;
    state: SharedState;
    currentProject: string;
}): any {
    const [hoverPoint, setHoverPoint] = useState<{ position: [number, number], direction: [number, number] } | null>(null);

    return <>
        {hoverPoint && <RenderLayer
            renderOrder={renderOrder.gestureInteractables}
            decal={true}
            batches={[
                {
                    mesh: state.planning.rendering.staticMeshes.GestureSplit,
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

        <Interactive3DShape id={gestureId + "split"} shape={{
            type: "path",
            path: centerLine,
            maxDistanceLeft: LANE_DISTANCE * nLanesBackward,
            maxDistanceRight: LANE_DISTANCE * nLanesForward,
        }} zIndex={3} cursorHover="col-resize" cursorActive="col-resize" onEvent={e => {
            if (e.drag) {
                if (e.drag.end) {
                    splitGesture(currentProject, gestureId, e.drag.end, true);
                }
                else if (e.drag.now) {
                    splitGesture(currentProject, gestureId, e.drag.now, false);
                }
            }
            if (e.hover) {
                if (e.hover.end) {
                    setHoverPoint(null);
                }
                else if (e.hover.now) {
                    setHoverPoint({ position: e.hover.projectedPosition, direction: e.hover.direction });
                }
            }
        }} />
    </>;
}
