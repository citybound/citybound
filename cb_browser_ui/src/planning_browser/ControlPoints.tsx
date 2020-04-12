import * as React from 'react';
import renderOrder from '../renderOrder';
import { RenderLayer, useCollectInstances, Interactive3DShape, useSettings } from '../browser_utils/Utils';
import update from 'immutability-helper';
import { SharedState, SetSharedState } from '../citybound';
import { Gesture } from '../wasm32-unknown-unknown/release/cb_browser_ui';
import { useState } from 'react';
import colors from '../colors';
import { vec3 } from 'gl-matrix';

function moveControlPoint(projectId, gestureId, pointIdx, newPosition, doneMoving) {
    window.cbRustBrowser.move_gesture_point(projectId, gestureId, pointIdx, [newPosition[0], newPosition[1]], doneMoving);

    if (!doneMoving) {
        return oldState => {
            // optimistic update
            let currentGesture = getGestureAsOf(oldState, projectId, gestureId);
            if (currentGesture) {
                let newPoints = [...currentGesture.points];
                newPoints[pointIdx] = newPosition;

                return update(oldState, {
                    planning: {
                        projects: {
                            [projectId]: {
                                ongoing: {
                                    $set: { gestures: { [gestureId]: Object.assign(currentGesture, { points: newPoints }) } }
                                }
                            }
                        }
                    }
                })
            } else {
                return s => s;
            }
        }
    } else {
        return s => s
    }
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

export function ControlPoints({ state, currentProject, planningMode, editedGesture, setState, setEditedGesture, setAddToEnd }: {
    state: SharedState;
    currentProject: string;
    planningMode: string;
    editedGesture: boolean;
    setState: SetSharedState;
    setEditedGesture: (string) => void;
    setAddToEnd: (bolean) => void;
}) {
    return <>
        {Object.entries(gesturesFromAllRelevantPlans(state, currentProject)).flatMap(([gestureId, gesture]) => {
            let isRelevant = (gesture.intent.Road && planningMode === "roads")
                || (gesture.intent.Zone && planningMode === "zoning");
            if (!isRelevant)
                return [];
            const path = gesture.intent.Road ? gesture.intent.Road.path : (gesture.intent.Zone ? gesture.intent.Zone.boundary : null);
            if (!path)
                return [];
            return [...path.corners.entries()].map(([pointIdx, corner]) => {
                let isFirst = pointIdx == 0;
                let isLast = pointIdx == path.corners.length - 1;
                return <ControlPointInteractable
                    point={corner.position}
                    isFirst={isFirst}
                    isLast={isLast}
                    fromMaster={gesture.fromMaster}
                    canvasFocused={!!editedGesture}
                    onMoved={(position, done) => {
                        setState(moveControlPoint(currentProject, gestureId, pointIdx, position, done));
                    }}
                    onEndClicked={(endClicked, clickedPos) => {
                        setEditedGesture(gestureId);
                        setAddToEnd(endClicked);
                        // TODO: set last click
                    }}
                    mesh={state.planning.rendering.staticMeshes.GestureDot}
                />;
            });
        })}
    </>;
}

function gesturesFromAllRelevantPlans(state: SharedState, currentProject: string): {
    [gestureID: string]: Gesture & {
        fromMaster?: boolean;
    };
} {
    return Object.keys(state.planning.master.gestures).map(gestureId => ({ [gestureId]: Object.assign({}, state.planning.master.gestures[gestureId][0], { fromMaster: true }) })).concat((currentProject && state.planning.projects[currentProject])
        ? state.planning.projects[currentProject].undoable_history
            .concat([state.planning.projects[currentProject].ongoing || { gestures: [] }]).map(step => step.gestures)
        : []).reduce((coll, gestures) => Object.assign(coll, gestures), {});
}

function ControlPointInteractable({ point, isFirst, isLast, onMoved, canvasFocused, onEndClicked, ControlPointInstance, fromMaster, mesh }) {
    const [hovered, setHovered] = useState<boolean>(false);
    const { moveVsClickPointDistance } = useSettings().planning;

    return <>
        <RenderLayer renderOrder={renderOrder.gestureInteractables} decal={true} batches={[{
            mesh,
            instances: new Float32Array([
                point[0], point[1], 0,
                1, 0,
                ...(hovered ? colors.controlPointHover : fromMaster ? colors.controlPointMaster : colors.controlPointCurrentProject)
            ])
        }]} />
        <Interactive3DShape
            shape={{
                type: "circle",
                center: [point[0], point[1], 0],
                radius: 3
            }}
            zIndex={canvasFocused ? 0 : 5}
            cursorHover="grab"
            cursorActive="grabbing"
            onEvent={e => {
                if (e.hover) {
                    if (e.hover.start) setHovered(true);
                    else if (e.hover.end) setHovered(false);
                }

                if (e.drag) {
                    if (e.drag.end) {
                        if ((isFirst || isLast) && vec3.dist(e.drag.end, e.drag.start) < moveVsClickPointDistance) {
                            onEndClicked(isLast, e.drag.end)
                        } else {
                            onMoved(e.drag.end, true);
                        }
                    } else if (e.drag.now) {
                        onMoved(e.drag.now, false);
                    }
                }
            }
            } />
    </>
}