import * as React from 'react';
import { useCallback, useState } from 'react';
import { Interactive3DShape, useSettings } from '../browser_utils/Utils';
import { vec3 } from 'gl-matrix';
import uuid from '../uuid';
import { SharedState } from '../citybound';

export default function GestureCanvas({ state, currentProject, editedGesture, setEditedGesture, intent, addToEnd, setAddToEnd }: { state: SharedState, currentProject, editedGesture, setEditedGesture, intent, addToEnd, setAddToEnd }) {
    const { finishGestureDistance } = useSettings().planning;

    const [previousClick, setPreviousClick] = useState<[number, number, number] | null>(null);

    const startNewGesture = useCallback((startPoint: [number, number, number]) => {
        let gestureId = uuid();

        window.cbRustBrowser.start_new_gesture(
            currentProject,
            gestureId,
            window.cbRustBrowser.with_control_point_added(intent, [startPoint[0], startPoint[1]], true)
        );

        setEditedGesture(gestureId);
        setAddToEnd(true);
        setPreviousClick(startPoint)
    }, [setEditedGesture, setAddToEnd, setPreviousClick, currentProject, intent]);

    const finishGesture = useCallback(() => {
        setEditedGesture(null);
        setPreviousClick(null);
    }, [setEditedGesture, setPreviousClick]);

    const addControlPoint = useCallback((point: [number, number, number], doneAdding) => {
        const currentIntent = getGestureAsOf(state, currentProject, editedGesture)?.intent;
        if (currentIntent) {
            window.cbRustBrowser.set_intent(currentProject, editedGesture, window.cbRustBrowser.with_control_point_added(currentIntent, [point[0], point[1]], addToEnd), doneAdding);
        } else {
            console.error("Couldn't get existing gesture state for gesture:" + editedGesture);
        }
    }, [state, currentProject, editedGesture, addToEnd]);

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
                if (editedGesture) addControlPoint(e.hover.now, false);
            }
            if (e.drag && e.drag.end) {
                if (editedGesture) {
                    if (previousClick
                        && vec3.dist(e.drag.end, previousClick) < finishGestureDistance) {
                        finishGesture();
                    } else {
                        setPreviousClick(e.drag.end);
                        addControlPoint(e.drag.end, true);
                    }
                } else if (intent) {
                    setPreviousClick(e.drag.end);
                    startNewGesture(e.drag.end);
                }
            }
        }} />
}

function getGestureAsOf(state, projectId, gestureId) {
    if (projectId && state.planning.projects[projectId]) {
        let project = state.planning.projects[projectId];
        for (let i = project.undoable_history.length - 1; i >= 0; i--) {
            let gestureInStep = project.undoable_history[i].gestures[gestureId];
            if (gestureInStep) {
                return gestureInStep;
            }
        }
    }
    return state.planning.master.gestures[gestureId]?.[0];
}
