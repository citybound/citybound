import * as React from 'react';
import { Interactive3DShape, RenderLayer } from '../../browser_utils/Utils';
import { SharedState } from '../../citybound';
import { useState } from 'react';
import colors from '../../colors';
import renderOrder from '../../renderOrder';

// TODO: also make this work for generic gestures, not just roads

export function insertControlPoint(projectId, gestureId, point, doneInserting) {
    window.cbRustBrowser.insert_control_point(projectId, gestureId, [point[0], point[1]], doneInserting);
}

export function InsertControlPointInteractable({ gestureId, centerLine, state, currentProject }: {
    gestureId: string;
    centerLine: any;
    state: SharedState;
    currentProject: string;
}): any {
    const [hoverPoint, setHoverPoint] = useState<[number, number] | null>(null);

    return <>
        {hoverPoint && <RenderLayer
            renderOrder={renderOrder.gestureInteractables}
            decal={true}
            batches={[
                {
                    mesh: state.planning.rendering.staticMeshes.GestureDot,
                    instances: new Float32Array([
                        hoverPoint[0],
                        hoverPoint[1],
                        0.0,
                        0.7, // scaled down
                        0.0,
                        ...colors.controlPointCurrentProject
                    ])
                }
            ]}
        />}

        <Interactive3DShape id={gestureId + "insert"} shape={{
            type: "path",
            path: centerLine,
            maxDistanceLeft: 2,
            maxDistanceRight: 2,
        }} zIndex={4} cursorHover="pointer" cursorActive="grabbing" onEvent={e => {
            if (e.drag) {
                if (e.drag.end) {
                    insertControlPoint(currentProject, gestureId, e.drag.end, true);
                }
                else if (e.drag.now) {
                    insertControlPoint(currentProject, gestureId, e.drag.now, false);
                }
            }
            if (e.hover) {
                if (e.hover.end) {
                    setHoverPoint(null);
                }
                else if (e.hover.now) {
                    setHoverPoint(e.hover.projectedPosition);
                }
            }
        }} />
    </>;
}
