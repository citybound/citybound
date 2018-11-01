import colors from '../colors';
import renderOrder from '../renderOrder';
import { vec3, mat4 } from 'gl-matrix';
import uuid from '../uuid';
import { solidColorShader } from 'monet';
import * as PlanningMenu from './PlanningMenu';

const LAND_USES = [
    "Residential",
    "Commercial",
    "Industrial",
    "Agricultural",
    "Recreational",
    "Official",
];

export const initialState = {
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
        roadCenterLines: {}
    },
    master: {
        gestures: {}
    },
    proposals: {
    },
    currentProposal: null,
    hoveredControlPoint: {},
    hoveredInsertPoint: null,
    canvasMode: {
        intent: null,
        currentGesture: null,
        addToEnd: true,
        previousClick: null,
    },
};

export const settingsSpec = {
    implementProposalKey: {
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

function getGestureAsOf(state, proposalId, gestureId) {
    if (proposalId && state.planning.proposals[proposalId]) {
        let proposal = state.planning.proposals[proposalId];
        for (let i = proposal.undoable_history.length - 1; i >= 0; i--) {
            let gestureInStep = proposal.undoable_history[i].gestures[gestureId];
            if (gestureInStep) {
                return gestureInStep;
            }
        }
    }
    return state.planning.master.gestures[gestureId][0];
}

function moveControlPoint(proposalId, gestureId, pointIdx, newPosition, doneMoving) {
    cbRustBrowser.move_gesture_point(proposalId, gestureId, pointIdx, [newPosition[0], newPosition[1]], doneMoving);

    if (!doneMoving) {

        return oldState => {
            let currentGesture = getGestureAsOf(oldState, proposalId, gestureId);
            if (currentGesture) {
                let newPoints = [...currentGesture.points];
                newPoints[pointIdx] = newPosition;

                return update(oldState, {
                    planning: {
                        proposals: {
                            [proposalId]: {
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

function startNewGesture(proposalId, intent, startPoint) {
    let gestureId = uuid();

    cbRustBrowser.start_new_gesture(proposalId, gestureId, intent, [startPoint[0], startPoint[1]]);

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

function addControlPoint(proposalId, gestureId, point, addToEnd, doneAdding) {
    cbRustBrowser.add_control_point(proposalId, gestureId, [point[0], point[1]], addToEnd, doneAdding);

    if (doneAdding) {
        return oldState => update(oldState, {
            planning: {
                canvasMode: {
                    previousClick: { $set: point }
                }
            }
        });
    } else {
        return s => s
    }
}

function insertControlPoint(proposalId, gestureId, point, doneInserting) {
    cbRustBrowser.insert_control_point(proposalId, gestureId, [point[0], point[1]], doneInserting);

    return oldState => update(oldState, {
        planning: {
            $unset: ["hoveredInsertPoint"]
        }
    });
}

function finishGesture(proposalId, gestureId) {
    cbRustBrowser.finish_gesture();

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
    Official: stripedShaders[2]
};

export function render(state, setState) {
    const controlPointsInstances = [];
    const controlPointsInteractables = [];

    if (state.planning) {
        let gestures = Object.keys(state.planning.master.gestures).map(gestureId =>
            ({ [gestureId]: Object.assign(state.planning.master.gestures[gestureId][0], { fromMaster: true }) })
        ).concat(state.planning.currentProposal
            ? state.planning.proposals[state.planning.currentProposal].undoable_history
                .concat([state.planning.proposals[state.planning.currentProposal].ongoing || { gestures: [] }]).map(step => step.gestures)
            : []
        ).reduce((coll, gestures) => Object.assign(coll, gestures), {});

        let { gestureId: hoveredGestureId, pointIdx: hoveredPointIdx } = state.planning.hoveredControlPoint;

        for (let gestureId of Object.keys(gestures)) {
            const gesture = gestures[gestureId];

            for (let [pointIdx, point] of gesture.points.entries()) {

                let isRelevant = (gesture.intent.Road && state.planning.planningMode === "roads")
                    || (gesture.intent.Zone && state.planning.planningMode === "zoning");

                if (isRelevant) {
                    let isHovered = gestureId == hoveredGestureId && pointIdx == hoveredPointIdx;

                    controlPointsInstances.push.apply(controlPointsInstances, [
                        point[0], point[1], 0,
                        1.0, 0.0,
                        ...(isHovered
                            ? colors.controlPointHover
                            : (gesture.fromMaster ? colors.controlPointMaster : colors.controlPointCurrentProposal))
                    ]);

                    controlPointsInteractables.push({
                        shape: {
                            type: "circle",
                            center: [point[0], point[1], 0],
                            radius: 3
                        },
                        zIndex: 3,
                        cursorHover: "grab",
                        cursorActive: "grabbing",
                        onEvent: e => {
                            if (e.hover) {
                                if (e.hover.end) {
                                    setState(update(state, {
                                        planning: {
                                            hoveredControlPoint: {
                                                $set: {}
                                            }
                                        }
                                    }))
                                } else if (e.hover.start) {
                                    setState(update(state, {
                                        planning: {
                                            hoveredControlPoint: {
                                                $set: { gestureId, pointIdx }
                                            },
                                            $unset: ["hoveredInsertPoint"]
                                        }
                                    }))
                                }
                            }

                            if (e.drag) {
                                if (e.drag.end) {
                                    setState(moveControlPoint(state.planning.currentProposal, gestureId, pointIdx, e.drag.end, true));
                                } else if (e.drag.now) {
                                    setState(moveControlPoint(state.planning.currentProposal, gestureId, pointIdx, e.drag.now, false));
                                }
                            }
                        }
                    })
                }
            }
        }
    }

    const roadCenterInteractables = [];

    if (state.planning.planningMode === "roads") {
        for (let gestureId of Object.keys(state.planning.rendering.roadCenterLines)) {
            let centerLine = state.planning.rendering.roadCenterLines[gestureId];

            roadCenterInteractables.push({
                shape: {
                    type: "path",
                    path: centerLine,
                    maxDistance: 1.5
                },
                zIndex: 2,
                cursorHover: "pointer",
                cursorActive: "grabbing",
                onEvent: e => {
                    if (e.drag) {
                        if (e.drag.end) {
                            setState(insertControlPoint(state.planning.currentProposal, gestureId, e.drag.end, true));
                        } else if (e.drag.now) {
                            setState(insertControlPoint(state.planning.currentProposal, gestureId, e.drag.now, false));
                        }
                    }
                    if (e.hover) {
                        if (e.hover.end) {
                            setState(update(state, {
                                planning: {
                                    $unset: ["hoveredInsertPoint"]
                                }
                            }))
                        } else if (e.hover.now) {
                            setState(update(state, {
                                planning: {
                                    hoveredInsertPoint: {
                                        $set: e.hover.now
                                    }
                                }
                            }))
                        }
                    }
                }
            })
        }
    }

    const { lanesToConstructGroups,
        lanesToConstructMarkerGroups,
        lanesToConstructMarkerGapsGroups,
        zoneGroups, zoneOutlineGroups,
        buildingOutlinesGroup } = state.planning.rendering.currentPreview;

    const layers = [
        {
            renderOrder: renderOrder.addedGesturesAsphalt,
            decal: true,
            batches: [...lanesToConstructGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))
        },
        {
            renderOrder: renderOrder.addedGesturesMarker,
            decal: true,
            batches: [...lanesToConstructMarkerGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedRoadMarkerInstance
            }))
        },
        {
            renderOrder: renderOrder.addedGesturesMarkerGap,
            decal: true,
            batches: [...lanesToConstructMarkerGapsGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))
        },
        ...[...zoneGroups.entries()].map(([landUse, groups]) => ({
            renderOrder: renderOrder.addedGesturesZones,
            decal: true,
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        ...[...zoneGroups.entries()].reverse().map(([landUse, groups]) => ({
            renderOrder: renderOrder.addedGesturesZonesStipple,
            decal: true,
            shader: shadersForLandUses[landUse],
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        ...[...zoneOutlineGroups.entries()].map(([landUse, groups]) => ({
            renderOrder: renderOrder.addedGesturesZonesOutlines,
            decal: true,
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        {
            renderOrder: renderOrder.buildingOutlines,
            decal: true,
            batches: [...buildingOutlinesGroup.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: buildingOutlinesInstance
            }))
        },
        {
            renderOrder: renderOrder.gestureInteractables,
            decal: true,
            batches: [{
                mesh: state.planning.rendering.staticMeshes.GestureDot,
                instances: new Float32Array(controlPointsInstances)
            },
            ...(state.planning.hoveredInsertPoint ? [
                {
                    mesh: state.planning.rendering.staticMeshes.GestureDot,
                    instances: new Float32Array([state.planning.hoveredInsertPoint[0], state.planning.hoveredInsertPoint[1], 0.0, 0.7, 0.0, ...colors.controlPointCurrentProposal])
                }
            ] : [])]
        }
    ];

    // TODO: invent a better way to preserve identity

    const interactables = [
        ...(state.planning.canvasMode.currentGesture ? [] : controlPointsInteractables),
        ...(state.planning.canvasMode.currentGesture ? [] : roadCenterInteractables),
        {
            id: "planningCanvas",
            shape: {
                type: "everywhere",
            },
            zIndex: 1,
            cursorHover: state.uiMode == "planning" ? "crosshair" : "normal",
            cursorActive: "pointer",
            onEvent: e => {
                const canvasMode = state.planning.canvasMode;
                if (e.hover && e.hover.now) {
                    if (canvasMode.currentGesture) {
                        setState(addControlPoint(
                            state.planning.currentProposal, canvasMode.currentGesture,
                            e.hover.now, canvasMode.addToEnd, false
                        ))
                    }
                }
                if (e.drag && e.drag.end) {
                    if (canvasMode.currentGesture) {
                        if (canvasMode.previousClick
                            && vec3.dist(e.drag.end, canvasMode.previousClick) < state.settings.planning.finishGestureDistance) {
                            setState(finishGesture(state.planning.currentProposal, canvasMode.currentGesture));
                        } else {
                            setState(addControlPoint(
                                state.planning.currentProposal, canvasMode.currentGesture,
                                e.drag.end, canvasMode.addToEnd, true
                            ))
                        }
                    } else if (canvasMode.intent) {
                        setState(startNewGesture(
                            state.planning.currentProposal, canvasMode.intent, e.drag.end
                        ));
                    }
                }
            }
        }
    ];

    let rendered = state.uiMode == "planning" && state.planning.currentProposal
        ? { layers, interactables }
        : {};

    return Object.assign(rendered, PlanningMenu.render(state, setState));
}

export function bindInputs(state, setState) {
    PlanningMenu.bindInputs(state, setState);
}