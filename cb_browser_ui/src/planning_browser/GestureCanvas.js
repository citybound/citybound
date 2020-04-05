import React from 'react';
import { Interactive3DShape } from '../browser_utils/Utils';

export default function GestureCanvas({ gestureActive, intentActive, previousClick, onStartGesture, onAddPoint, onFinishGesture, finishDistance }) {
    return <Interactive3DShape
        id="planningCanvas"
        key="planningCanvas"
        shape={{
            type: "everywhere",
        }}
        zIndex={1}
        cursorHover={"crosshair"}
        cursorActive="pointer"
        onEvent={e => {
            if (e.hover && e.hover.now) {
                if (gestureActive) onAddPoint(e.hover.now, false);
            }
            if (e.drag && e.drag.end) {
                if (gestureActive) {
                    if (previousClick
                        && vec3.dist(e.drag.end, previousClick) < finishDistance) {
                        onFinishGesture();
                    } else {
                        onAddPoint(e.drag.end, true)
                    }
                } else if (intentActive) {
                    onStartGesture(e.drag.end);
                }
            }
        }
        } />
}