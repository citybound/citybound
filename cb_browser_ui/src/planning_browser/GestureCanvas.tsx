import * as React from 'react';
import { useState } from 'react';
import { Interactive3DShape } from '../browser_utils/Utils';
import { vec3 } from 'gl-matrix';

export default function GestureCanvas(props: { gestureActive, intentActive, onStartGesture, onAddPoint, onFinishGesture, finishDistance }) {
    const [previousClick, setPreviousClick] = useState<vec3 | undefined>(undefined);

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
                if (props.gestureActive) props.onAddPoint(e.hover.now, false);
            }
            if (e.drag && e.drag.end) {
                if (props.gestureActive) {
                    if (previousClick
                        && vec3.dist(e.drag.end, previousClick) < props.finishDistance) {
                        props.onFinishGesture();
                    } else {
                        setPreviousClick(e.drag.end);
                        props.onAddPoint(e.drag.end, true);
                    }
                } else if (props.intentActive) {
                    setPreviousClick(e.drag.end);
                    props.onStartGesture(e.drag.end);
                }
            }
        }} />
}