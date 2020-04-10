import colors from '../colors';
import renderOrder from '../renderOrder';
import { vec3, mat4 } from 'gl-matrix';
import uuid from '../uuid';
import { solidColorShader } from 'monet';
import * as PlanningMenu from './PlanningMenu';
export const Tools = PlanningMenu.Tools;
import * as React from 'react'
import { RenderLayer, Interactive3DShape } from '../browser_utils/Utils';
import { vec2 } from 'gl-matrix';
import ControlPointInteractable from './ControlPointInteractable';
import GestureCanvas from './GestureCanvas';
import update from 'immutability-helper';
import { SharedState, SetSharedState } from '../citybound';
import { Intent } from '../wasm32-unknown-unknown/release/cb_browser_ui';

const LAND_USES = [
    "Residential",
    "Commercial",
    "Industrial",
    "Agricultural",
    "Recreational",
    "Administrative",
];

type BatchID = string;
type GroupID = string;
type GroupMesh = {};

type Project = {
    gestures: {}
}

export type PlanningSharedState = {
    planningMode: null | "roads" | "zoning",
    rendering: {
        staticMeshes: {},
        currentPreview: {
            lanesToConstructGroups: Map<BatchID, Map<GroupID, GroupMesh>>,
            lanesToConstructMarkerGroups: Map<BatchID, Map<GroupID, GroupMesh>>,
            lanesToConstructMarkerGapsGroups: Map<BatchID, Map<GroupID, GroupMesh>>,
            zoneGroups: Map<BatchID, Map<GroupID, GroupMesh>>,
            zoneOutlineGroups: Map<BatchID, Map<GroupID, GroupMesh>>,
            buildingOutlinesGroup: Map<BatchID, Map<GroupID, GroupMesh>>,
        },
        roadInfos: {}
    },
    master: {
        gestures: {}
    },
    projects: {
        [projectId: string]: {
            undoable_history: Project[]
            ongoing: Project
        }
    },
    currentProject: null | string,
    hoveredControlPoint: { gestureId?: string, pointIdx?: number },
    hoveredInsertPoint: null | [number, number],
    hoveredSplitPoint: null | { point: [number, number], direction: [number, number] },
    hoveredChangeNLanesPoint: null,
    canvasMode: {
        intent: null | Intent,
        currentGesture: string,
        addToEnd: boolean,
        previousClick: null | [number, number],
        finishDistance: number
    },
}

export const initialState: PlanningSharedState = {
    planningMode: null,
    rendering: {
        staticMeshes: {},
        currentPreview: {
            lanesToConstructGroups: new Map(),
            lanesToConstructMarkerGroups: new Map(),
            lanesToConstructMarkerGapsGroups: new Map(),
            zoneGroups: new Map(LAND_USES.map(landUse => [landUse, new Map()])),
            zoneOutlineGroups: new Map(LAND_USES.map(landUse => [landUse, new Map()])),
            buildingOutlinesGroup: new Map(),
        },
        roadInfos: {}
    },
    master: {
        gestures: {}
    },
    projects: {
    },
    currentProject: null,
    hoveredControlPoint: {},
    hoveredInsertPoint: null,
    hoveredSplitPoint: null,
    hoveredChangeNLanesPoint: null,
    canvasMode: {
        intent: null,
        currentGesture: null,
        addToEnd: true,
        previousClick: null,
        finishDistance: 3
    },
};

export const settingsSpec = {
    implementProjectKey: {
        default: {
            key: /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'command+enter' : 'ctrl+enter'
        }, description: "Implement Plan"
    },
    undoKey: {
        default: {
            key: /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'command+z' : 'ctrl+z'
        }, description: "Undo Plan Step"
    },
    redoKey: {
        default: {
            key: /Mac|iPod|iPhone|iPad/.test(navigator.platform) ? 'command+shift+z' : 'ctrl+shift+z'
        }, description: "Redo Plan Step"
    },
    finishGestureDistance: { default: 3.0, description: "Finish Gesture Double-Click Distance", min: 0.5, max: 10.0, step: 0.1 }
}

// STATE MUTATING ACTIONS

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

function moveControlPoint(projectId, gestureId, pointIdx, newPosition, doneMoving) {
    window.cbRustBrowser.move_gesture_point(projectId, gestureId, pointIdx, [newPosition[0], newPosition[1]], doneMoving);

    if (!doneMoving) {

        return oldState => {
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

function startNewGesture(projectId, intent, startPoint) {
    let gestureId = uuid();

    window.cbRustBrowser.start_new_gesture(projectId, gestureId, window.cbRustBrowser.with_control_point_added(intent, [startPoint[0], startPoint[1]], true));

    return oldState => update(oldState, {
        planning: {
            canvasMode: {
                currentGesture: { $set: gestureId },
                addToEnd: { $set: true },
                previousClick: { $set: startPoint }
            }
        }
    });
}



function addControlPoint(projectId, gestureId, point, addToEnd, doneAdding) {
    return oldState => {
        const currentIntent = getGestureAsOf(oldState, projectId, gestureId)?.intent;
        if (currentIntent) {
            window.cbRustBrowser.set_intent(projectId, gestureId, window.cbRustBrowser.with_control_point_added(currentIntent, [point[0], point[1]], addToEnd), doneAdding);
        } else {
            console.error("Couldn't get existing gesture state for gesture:" + gestureId);
        }

        if (doneAdding) {
            return update(oldState, {
                planning: {
                    canvasMode: {
                        previousClick: { $set: point }
                    }
                }
            });
        } else {
            return oldState;
        }

    }
}

function insertControlPoint(projectId, gestureId, point, doneInserting) {
    window.cbRustBrowser.insert_control_point(projectId, gestureId, [point[0], point[1]], doneInserting);

    return oldState => update(oldState, {
        planning: {
            $unset: ["hoveredInsertPoint"]
        }
    });
}

function splitGesture(projectId, gestureId, splitAt, doneSplitting) {
    window.cbRustBrowser.split_gesture(projectId, gestureId, [splitAt[0], splitAt[1]], doneSplitting);

    return oldState => update(oldState, {
        planning: {
            $unset: ["hoveredSplitPoint"]
        }
    });
}

function setNLanes(projectId, gestureId, nLanesForward, nLanesBackward, doneChanging) {
    window.cbRustBrowser.set_n_lanes(projectId, gestureId, nLanesForward, nLanesBackward, doneChanging);

    return oldState => update(oldState, {
        planning: {
            $unset: ["hoveredChangeNLanesPoint"]
        }
    });
}

function finishGesture(projectId, gestureId) {
    return oldState => update(oldState, {
        planning: {
            canvasMode: {
                $unset: ['currentGesture', 'previousClick']
            }
        }
    });
}

// INTERACTABLES AND RENDER LAYERS

const destructedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.destructedAsphalt]);
const plannedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt]);
const plannedRoadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedRoadMarker]);
const landUseInstances = new Map(LAND_USES.map(landUse => [landUse, new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[landUse]])]));
const buildingOutlinesInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.buildingOutlines]);

const stripedShaders = [
    "mod(p.x + p.y, 6.0) < 2.0 && mod(p.x - p.y, 6.0) > 2.0",
    "mod(p.x + p.y, 6.0) > 2.0 && mod(p.x + p.y, 6.0) < 4.0 && mod(p.x - p.y, 6.0) > 2.0",
    "mod(p.x + p.y, 6.0) > 4.0 && mod(p.x - p.y, 6.0) > 2.0"
].map(condition => ({
    vertex: solidColorShader.vertex,
    fragment: `
precision mediump float;
varying vec3 p;
varying vec3 color;
void main() {
    if (${condition}) {
        gl_FragColor = vec4(pow(color, vec3(1.0/2.2)), 1.0);
    } else {
        discard;
    }
}`}));

const shadersForLandUses = {
    Residential: stripedShaders[0],
    Commercial: stripedShaders[1],
    Industrial: stripedShaders[2],
    Agricultural: stripedShaders[1],
    Recreational: stripedShaders[2],
    Administrative: stripedShaders[2]
};

// TODO: share constants with Rust somehow
const LANE_DISTANCE = 0.8 * 3.9;

export function ShapesAndLayers(props: { state: SharedState, setState: SetSharedState }) {
    const { state, setState } = props;
    const controlPointsInstances = [];
    const controlPointsInteractables = [];

    if (state.uiMode != "planning" || !state.planning.currentProject) {
        return null;
    }

    if (state.planning) {
        let gestures = Object.keys(state.planning.master.gestures).map(gestureId =>
            ({ [gestureId]: Object.assign(state.planning.master.gestures[gestureId][0], { fromMaster: true }) })
        ).concat((state.planning.currentProject && state.planning.projects[state.planning.currentProject])
            ? state.planning.projects[state.planning.currentProject].undoable_history
                .concat([state.planning.projects[state.planning.currentProject].ongoing || { gestures: [] }]).map(step => step.gestures)
            : []
        ).reduce((coll, gestures) => Object.assign(coll, gestures), {});

        let { gestureId: hoveredGestureId, pointIdx: hoveredPointIdx } = state.planning.hoveredControlPoint;

        for (let gestureId of Object.keys(gestures)) {
            const gesture = gestures[gestureId];
            const path = gesture.intent.Road ? gesture.intent.Road.path : (gesture.intent.Zone ? gesture.intent.Zone.boundary : null);
            if (!path) continue;

            for (let [pointIdx, corner] of path.corners.entries()) {

                let isRelevant = (gesture.intent.Road && state.planning.planningMode === "roads")
                    || (gesture.intent.Zone && state.planning.planningMode === "zoning");

                if (isRelevant) {
                    let isHovered = gestureId == hoveredGestureId && pointIdx == hoveredPointIdx;

                    let isFirst = pointIdx == 0;
                    let isLast = pointIdx == path.corners.length - 1;

                    controlPointsInstances.push.apply(controlPointsInstances, [
                        corner.position[0], corner.position[1], 0,
                        1.0, 0.0,
                        ...(isHovered
                            ? colors.controlPointHover
                            : (gesture.fromMaster ? colors.controlPointMaster : colors.controlPointCurrentProject))
                    ]);

                    controlPointsInteractables.push(<ControlPointInteractable
                        point={corner.position}
                        isFirst={isFirst}
                        isLast={isLast}

                        gestureActive={!!state.planning.canvasMode.currentGesture}

                        onHover={() => setState((state: SharedState) => update(state, {
                            planning: {
                                hoveredControlPoint: {
                                    $set: { gestureId, pointIdx }
                                },
                                $unset: ["hoveredInsertPoint"]
                            }
                        }))}

                        onHoverEnd={() => setState(state => update(state, {
                            planning: {
                                hoveredControlPoint: {
                                    $set: {}
                                }
                            }
                        }))}

                        onControlPointMoved={(position, done) => {
                            setState(moveControlPoint(state.planning.currentProject, gestureId, pointIdx, position, done));
                        }}

                        finishDistance={state.settings.planning.finishGestureDistance}

                        onEndClicked={(endClicked, clickedPos) => {
                            setState(oldState => update(oldState, {
                                planning: {
                                    canvasMode: {
                                        currentGesture: { $set: gestureId },
                                        addToEnd: { $set: endClicked },
                                        previousClick: { $set: clickedPos }
                                    }
                                }
                            }))
                        }}
                    />)
                }
            }
        }
    }

    const roadCenterInteractables = [];

    if (state.planning.planningMode === "roads") {
        for (let gestureId of Object.keys(state.planning.rendering.roadInfos)) {
            let { centerLine, outline, nLanesForward, nLanesBackward } = state.planning.rendering.roadInfos[gestureId];

            roadCenterInteractables.push(<Interactive3DShape
                id={gestureId + "insert"}
                shape={{
                    type: "path",
                    path: centerLine,
                    maxDistanceLeft: 2,
                    maxDistanceRight: 2,
                }}
                zIndex={4}
                cursorHover="pointer"
                cursorActive="grabbing"
                onEvent={e => {
                    if (e.drag) {
                        if (e.drag.end) {
                            setState(insertControlPoint(state.planning.currentProject, gestureId, e.drag.end, true));
                        } else if (e.drag.now) {
                            setState(insertControlPoint(state.planning.currentProject, gestureId, e.drag.now, false));
                        }
                    }
                    if (e.hover) {
                        if (e.hover.end) {
                            setState(state => update(state, {
                                planning: {
                                    $unset: ["hoveredInsertPoint"]
                                }
                            }))
                        } else if (e.hover.now) {
                            setState(state => update(state, {
                                planning: {
                                    hoveredInsertPoint: {
                                        $set: e.hover.projectedPosition
                                    }
                                }
                            }))
                        }
                    }
                }} />);

            roadCenterInteractables.push(<Interactive3DShape
                id={gestureId + "split"}
                shape={{
                    type: "path",
                    path: centerLine,
                    maxDistanceLeft: LANE_DISTANCE * nLanesBackward,
                    maxDistanceRight: LANE_DISTANCE * nLanesForward,
                }}
                zIndex={3}
                cursorHover="col-resize"
                cursorActive="col-resize"
                onEvent={e => {
                    if (e.drag) {
                        if (e.drag.end) {
                            setState(splitGesture(state.planning.currentProject, gestureId, e.drag.end, true));
                        } else if (e.drag.now) {
                            setState(splitGesture(state.planning.currentProject, gestureId, e.drag.now, false));
                        }
                    }
                    if (e.hover) {
                        if (e.hover.end) {
                            setState(state => update(state, {
                                planning: {
                                    $unset: ["hoveredSplitPoint"]
                                }
                            }))
                        } else if (e.hover.now) {
                            setState(state => update(state, {
                                planning: {
                                    hoveredSplitPoint: {
                                        $set: { point: e.hover.projectedPosition, direction: e.hover.direction }
                                    }
                                }
                            }))
                        }
                    }
                }} />)

            roadCenterInteractables.push(<Interactive3DShape
                id={gestureId + "changeNLanes"}
                shape={{
                    type: "path",
                    path: centerLine,
                    maxDistanceLeft: LANE_DISTANCE * nLanesBackward + 2,
                    maxDistanceRight: LANE_DISTANCE * nLanesForward + 2,
                }}
                zIndex={2}
                cursorHover="ew-resize"
                cursorActive="ew-resize"
                onEvent={e => {
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
                            } else {
                                newNLanesBackward = Math.max(0.0, Math.round(-orthogonalDistance / 3.0));
                            }

                            setState(setNLanes(state.planning.currentProject, gestureId, newNLanesForward, newNLanesBackward, e.drag.end ? true : false));
                        }
                    }
                    if (e.hover) {
                        if (e.hover.end) {
                            setState(state => update(state, {
                                planning: {
                                    $unset: ["hoveredChangeNLanesPoint"]
                                }
                            }))
                        } else if (e.hover.now) {
                            const position = e.hover.now;
                            const { projectedPosition, direction } = e.hover;
                            const orthogonalRightDirection = [direction[1], -direction[0]];

                            const vector = vec2.sub(vec2.create(), position, projectedPosition);
                            const orthogonalDistance = vec2.dot(vector, orthogonalRightDirection);

                            let shiftedPoint;
                            if (orthogonalDistance > 0.0) {
                                shiftedPoint = vec2.scaleAndAdd(vec2.create(), e.hover.projectedPosition, orthogonalRightDirection, LANE_DISTANCE * nLanesForward);
                            } else {
                                shiftedPoint = vec2.scaleAndAdd(vec2.create(), e.hover.projectedPosition, orthogonalRightDirection, - LANE_DISTANCE * nLanesBackward);
                            }

                            setState(state => update(state, {
                                planning: {
                                    hoveredChangeNLanesPoint: {
                                        $set: { point: shiftedPoint, direction: direction }
                                    }
                                }
                            }))
                        }
                    }
                }
                } />)
        }
    }

    const { lanesToConstructGroups,
        lanesToConstructMarkerGroups,
        lanesToConstructMarkerGapsGroups,
        zoneGroups, zoneOutlineGroups,
        buildingOutlinesGroup } = state.planning.rendering.currentPreview;

    const layers = [
        <RenderLayer renderOrder={renderOrder.addedGesturesAsphalt}
            decal={true}
            batches={[...lanesToConstructGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))} />,
        <RenderLayer renderOrder={renderOrder.addedGesturesMarker}
            decal={true}
            batches={[...lanesToConstructMarkerGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedRoadMarkerInstance
            }))} />,
        <RenderLayer renderOrder={renderOrder.addedGesturesMarkerGap}
            decal={true}
            batches={[...lanesToConstructMarkerGapsGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))} />,
        [...zoneGroups.entries()].map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZones}
            decal={true}
            batches={[...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))} />
        ),
        [...zoneGroups.entries()].reverse().map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZonesStipple}
            decal={true}
            shader={shadersForLandUses[landUse]}
            batches={[...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))} />
        ),
        [...zoneOutlineGroups.entries()].map(([landUse, groups]) => <RenderLayer renderOrder={renderOrder.addedGesturesZonesOutlines}
            decal={true}
            batches={[...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))} />
        ),
        <RenderLayer renderOrder={renderOrder.buildingOutlines}
            decal={true}
            batches={[...buildingOutlinesGroup.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: buildingOutlinesInstance
            }))} />,
        <RenderLayer renderOrder={renderOrder.gestureInteractables}
            decal={true}
            batches={[{
                mesh: state.planning.rendering.staticMeshes.GestureDot,
                instances: new Float32Array(controlPointsInstances)
            },
            ...(state.planning.hoveredInsertPoint ? [
                {
                    mesh: state.planning.rendering.staticMeshes.GestureDot,
                    instances: new Float32Array([
                        state.planning.hoveredInsertPoint[0],
                        state.planning.hoveredInsertPoint[1],
                        0.0,
                        0.7, // scaled down
                        0.0,
                        ...colors.controlPointCurrentProject
                    ])
                }
            ] : [])]} />,
        <RenderLayer renderOrder={renderOrder.gestureInteractables}
            decal={true}
            batches={[
                ...(state.planning.hoveredSplitPoint ? [
                    {
                        mesh: state.planning.rendering.staticMeshes.GestureSplit,
                        instances: new Float32Array([
                            state.planning.hoveredSplitPoint.point[0],
                            state.planning.hoveredSplitPoint.point[1],
                            0.0,
                            state.planning.hoveredSplitPoint.direction[0],
                            state.planning.hoveredSplitPoint.direction[1],
                            ...colors.controlPointCurrentProject
                        ])
                    }
                ] : [])]} />,
        <RenderLayer renderOrder={renderOrder.gestureInteractables}
            decal={true}
            batches={[
                ...(state.planning.hoveredChangeNLanesPoint ? [
                    {
                        mesh: state.planning.rendering.staticMeshes.GestureChangeNLanes,
                        instances: new Float32Array([
                            state.planning.hoveredChangeNLanesPoint.point[0],
                            state.planning.hoveredChangeNLanesPoint.point[1],
                            0.0,
                            state.planning.hoveredChangeNLanesPoint.direction[0],
                            state.planning.hoveredChangeNLanesPoint.direction[1],
                            ...colors.controlPointCurrentProject
                        ])
                    }
                ] : [])]} />
    ];

    // TODO: invent a better way to preserve identity

    const interactables = [
        state.planning.canvasMode.currentGesture ? null : controlPointsInteractables,
        state.planning.canvasMode.currentGesture ? null : roadCenterInteractables,
        <GestureCanvas
            gestureActive={!!state.planning.canvasMode.currentGesture}
            intentActive={!!state.planning.canvasMode.intent}
            onStartGesture={startPosition => setState(startNewGesture(
                state.planning.currentProject, state.planning.canvasMode.intent, startPosition
            ))}
            previousClick={state.planning.previousClick}
            onAddPoint={(position, done) => {
                setState(addControlPoint(
                    state.planning.currentProject, state.planning.canvasMode.currentGesture,
                    position, state.planning.canvasMode.addToEnd, done
                ))
            }}
            onFinishGesture={() => {
                setState(finishGesture(state.planning.currentProject, state.planning.canvasMode.currentGesture));
            }}
            finishDistance={state.planning.canvasMode.finishDistance}
        />
    ];

    return [
        interactables,
        layers
    ]
}

export function bindInputs(state, setState) {
    PlanningMenu.bindInputs(state, setState);
}