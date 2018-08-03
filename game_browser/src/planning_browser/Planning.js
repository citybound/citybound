import colors from '../colors';
import React from 'react';
import { vec3, mat4 } from 'gl-matrix';
import * as cityboundBrowser from '../../Cargo.toml';
import uuid from '../uuid';

const EL = React.createElement;

export const initialState = {
    rendering: {
        staticMeshes: {},
        currentPreview: {}
    },
    master: {
        gestures: {}
    },
    proposals: {
    },
    currentProposal: null,
    hoveredControlPoint: {},
    canvasMode: {
        intent: null,
        currentGesture: null,
        addToEnd: true,
        previousClick: null,
    },
    settings: {
        finishGestureDistance: 3.0
    }
};

// STATE MUTATING ACTIONS

function switchToProposal(proposalId) {
    console.log("switching to", proposalId);

    return oldState => update(oldState, {
        planning: { currentProposal: { $set: proposalId } }
    })
}

function getGestureAsOf(state, proposalId, gestureId) {
    if (proposalId && state.planning.proposals[proposalId]) {
        let proposal = state.planning.proposals[proposalId];
        for (let i = proposal.undoable_history.length - 1; i >= 0; i--) {
            let gestureInStep = proposal.undoable_history[i][gestureId];
            if (gestureInStep) {
                return gestureInStep;
            }
        }
    }
    return state.planning.master.gestures[gestureId][0];
}

let canvas = null;

function moveControlPoint(proposalId, gestureId, pointIdx, newPosition, doneMoving) {
    cityboundBrowser.move_gesture_point(proposalId, gestureId, pointIdx, [newPosition[0], newPosition[1]], doneMoving);

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

    cityboundBrowser.start_new_gesture(proposalId, gestureId, intent, [startPoint[0], startPoint[1]]);

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
    cityboundBrowser.add_control_point(proposalId, gestureId, [point[0], point[1]], addToEnd, doneAdding);

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

function finishGesture(proposalId, gestureId) {
    cityboundBrowser.finish_gesture();

    return oldState => update(oldState, {
        planning: {
            canvasMode: {
                $unset: ['currentGesture', 'previousClick']
            }
        }
    });
}

function implementProposal(proposalId) {
    cityboundBrowser.implement_proposal(proposalId);
    return oldState => update(oldState, { planning: { $unset: ['currentProposal'] } });
}

// INTERACTABLES AND RENDER LAYERS

const destructedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.destructedAsphalt]);
const plannedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt]);
const plannedRoadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedRoadMarker]);
const residentialInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.residential]);

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

                let isRelevant = (gesture.intent.Road && state.uiMode === "main/planning/roads")
                    || (gesture.intent.Zone && state.uiMode.startsWith("main/planning/zoning"));

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
                        zIndex: 2,
                        cursorHover: "grab",
                        cursorActive: "grabbing",
                        onEvent: e => {
                            if (e.hover) {
                                if (e.hover.start) {
                                    setState(update(state, {
                                        planning: {
                                            hoveredControlPoint: {
                                                $set: { gestureId, pointIdx }
                                            }
                                        }
                                    }))
                                } else if (e.hover.end) {
                                    setState(update(state, {
                                        planning: {
                                            hoveredControlPoint: {
                                                $set: {}
                                            }
                                        }
                                    }))
                                }
                            }

                            if (e.drag) {
                                if (e.drag.now) {
                                    setState(moveControlPoint(state.planning.currentProposal, gestureId, pointIdx, e.drag.now, false));
                                } else if (e.drag.end) {
                                    setState(moveControlPoint(state.planning.currentProposal, gestureId, pointIdx, e.drag.end, true));
                                }
                            }
                        }
                    })
                }
            }
        }
    }

    const layers = [
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.currentPreview.lanesToDestruct,
                instances: destructedAsphaltInstance
            }]
        },
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.currentPreview.lanesToConstruct,
                instances: plannedAsphaltInstance
            }]
        },
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.currentPreview.lanesToConstructMarker,
                instances: plannedRoadMarkerInstance
            }]
        },
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.currentPreview.switchLanesToConstructMarkerGap,
                instances: plannedAsphaltInstance
            }]
        },
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.currentPreview.zones,
                instances: residentialInstance
            }]
        },
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.staticMeshes.GestureDot,
                instances: new Float32Array(controlPointsInstances)
            }]
        }
    ];

    function makeToolbar(id, descriptions, prefix, uiMode, setMode, colorMap) {
        if (uiMode.startsWith(prefix)) {
            return [EL("div", { id, className: "toolbar" }, descriptions.map(description => {
                const descriptionSlug = description.toLowerCase().replace(/\s/g, "-")
                return EL("button", {
                    id: descriptionSlug,
                    key: descriptionSlug,
                    alt: description,
                    className: uiMode.startsWith(prefix + "/" + descriptionSlug) ? "active" : "",
                    onClick: () => setMode(prefix + "/" + descriptionSlug),
                    style: colorMap ? { backgroundColor: colorMap(descriptionSlug) } : {}
                })
            }))]
        } else {
            return []
        }
    }

    const setUiMode = uiMode => {
        let updateOp = { uiMode: { $set: uiMode } };
        if (uiMode === "main/planning/roads") {
            updateOp.planning = {
                canvasMode: {
                    intent: {
                        $set: {
                            Road: {
                                n_lanes_forward: 2,
                                n_lanes_backward: 2
                            }
                        }
                    }
                }
            }
        } else if (uiMode === "main/planning/zoning/residential") {
            updateOp.planning = {
                canvasMode: {
                    intent: {
                        $set: {
                            Zone: {
                                LandUse: "Residential"
                            }
                        }
                    }
                }
            }
        } else {
            updateOp.planning = { canvasMode: { intent: { $set: null } } }
        }

        setState(oldState => update(oldState, updateOp))
    }

    const elements = [
        ...makeToolbar("main-toolbar", ["Inspection", "Planning", "Budgeting"], "main", state.uiMode, setUiMode),
        ...makeToolbar("planning-toolbar", ["Roads", "Zoning"], "main/planning", state.uiMode, setUiMode),
        ...makeToolbar("zoning-toolbar", [
            "Residential",
            "Commercial",
            "Offices",
            "Industrial",
            "Agricultural",
            "Recreational",
            "Official"
        ], "main/planning/zoning", state.uiMode, setUiMode,
            zone => {
                let c = colors[zone];
                return `rgb(${Math.pow(c[0], 1 / 2.2) * 256}, ${Math.pow(c[1], 1 / 2.2) * 256}, ${Math.pow(c[2], 1 / 2.2) * 256}`
            }
        ),
        EL("div", { key: "proposals", className: "window proposals" }, [
            EL("h1", { key: "heading" }, "Proposals"),
            ...Object.keys(state.planning.proposals).map(proposalId =>
                proposalId == state.planning.currentProposal
                    ? EL("p", { key: proposalId }, [
                        "" + proposalId,
                        EL("button", {
                            onClick: () => setState(implementProposal(state.planning.currentProposal))
                        }, "implement")
                    ])
                    : EL("button", {
                        key: proposalId,
                        onClick: () => setState(switchToProposal(proposalId))
                    }, "" + proposalId)
            ),
        ]),
    ];

    // TODO: invent a better way to preserve identity

    const interactables = [
        ...(state.planning.canvasMode.currentGesture ? [] : controlPointsInteractables),
        {
            id: "planningCanvas",
            shape: {
                type: "everywhere",
            },
            zIndex: 1,
            cursorHover: "crosshair",
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
                            && vec3.dist(e.drag.end, canvasMode.previousClick) < state.planning.settings.finishGestureDistance) {
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
    ]

    if (state.uiMode.startsWith("main/planning") && state.planning.currentProposal) {
        return [layers, interactables, elements];
    } else {
        return [[], [], elements];
    }
}