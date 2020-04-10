import React from 'react';
import { Interactive3DShape } from '../browser_utils/Utils.js';
import { vec3 } from 'gl-matrix';

export default function ControlPointInteractable({ point, isFirst, isLast, onHover, onHoverEnd, onControlPointMoved, gestureActive, onEndClicked, finishDistance }) {
    return <Interactive3DShape
        shape={{
            type: "circle",
            center: [point[0], point[1], 0],
            radius: 3
        }}
        zIndex={gestureActive ? 0 : 5}
        cursorHover="grab"
        cursorActive="grabbing"
        onEvent={e => {
            if (e.hover) {
                if (e.hover.start) onHover();
                else if (e.hover.end) onHoverEnd();
            }

            if (e.drag) {
                if (e.drag.end) {
                    if ((isFirst || isLast) && vec3.dist(e.drag.end, e.drag.start) < finishDistance) {
                        onEndClicked(isLast, e.drag.end)
                    } else {
                        onControlPointMoved(e.drag.end, true);
                    }
                } else if (e.drag.now) {
                    onControlPointMoved(e.drag.now, false);
                }
            }
        }
        } />
}