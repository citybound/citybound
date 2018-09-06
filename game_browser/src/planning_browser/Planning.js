import colors from '../colors';
import React from 'react';
import { vec3, mat4 } from 'gl-matrix';
import * as cityboundBrowser from '../../Cargo.toml';
import uuid from '../uuid';
import { Button, Select } from 'antd';
const Option = Select.Option;

import { solidColorShader } from 'monet';
import { makeToolbar } from '../toolbar';

const EL = React.createElement;

const LAND_USES = [
    "Residential",
    "Commercial",
    "Industrial",
    "Agricultural",
    "Recreational",
    "Official",
];

export const initialState = {
    rendering: {
        staticMeshes: {},
        currentPreview: {
            lanesToConstructGroups: new Map(),
            lanesToConstructMarkerGroups: new Map(),
            lanesToConstructMarkerGapsGroups: new Map(),
            zoneGroups: new Map(LAND_USES.map(landUse => [landUse, new Map()])),
            zoneOutlineGroups: new Map(LAND_USES.map(landUse => [landUse, new Map()])),
        }
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
};

export const settingsSpec = {
    implementProposalKey: { default: 'command+enter', description: "Implement Plan Key" },
    finishGestureDistance: { default: 3.0, description: "Finish Gesture Double-Click Distance", min: 0.5, max: 10.0, step: 0.1 }
}

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
            let gestureInStep = proposal.undoable_history[i].gestures[gestureId];
            if (gestureInStep) {
                return gestureInStep;
            }
        }
    }
    return state.planning.master.gestures[gestureId][0];
}

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

function implementProposal(oldState) {
    cityboundBrowser.implement_proposal(oldState.planning.currentProposal);
    return update(oldState, {
        planning: {
            $unset: ['currentProposal'],
        }
    });
}

// INTERACTABLES AND RENDER LAYERS

const destructedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.destructedAsphalt]);
const plannedAsphaltInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt]);
const plannedRoadMarkerInstance = new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedRoadMarker]);
const landUseInstances = new Map(LAND_USES.map(landUse => [landUse, new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors[landUse]])]));

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

                let isRelevant = (gesture.intent.Road && state.uiMode === "main/Planning/Roads")
                    || (gesture.intent.Zone && state.uiMode.startsWith("main/Planning/Zoning"));

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

    const { lanesToConstructGroups,
        lanesToConstructMarkerGroups,
        lanesToConstructMarkerGapsGroups,
        zoneGroups, zoneOutlineGroups } = state.planning.rendering.currentPreview;

    const layers = [
        {
            decal: true,
            batches: [...lanesToConstructGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))
        },
        {
            decal: true,
            batches: [...lanesToConstructMarkerGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedRoadMarkerInstance
            }))
        },
        {
            decal: true,
            batches: [...lanesToConstructMarkerGapsGroups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: plannedAsphaltInstance
            }))
        },
        ...[...zoneGroups.entries()].map(([landUse, groups]) => ({
            decal: true,
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        ...[...zoneGroups.entries()].reverse().map(([landUse, groups]) => ({
            decal: true,
            shader: shadersForLandUses[landUse],
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        ...[...zoneOutlineGroups.entries()].map(([landUse, groups]) => ({
            decal: true,
            batches: [...groups.values()].map(groupMesh => ({
                mesh: groupMesh,
                instances: landUseInstances.get(landUse)
            }))
        })),
        {
            decal: true,
            batches: [{
                mesh: state.planning.rendering.staticMeshes.GestureDot,
                instances: new Float32Array(controlPointsInstances)
            }]
        }
    ];

    const setUiMode = uiMode => {
        let updateOp = { uiMode: { $set: uiMode } };
        if (uiMode === "main/Planning/Roads") {
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
        } else if (uiMode.startsWith("main/Planning/Zoning/")) {
            let [landUse] = uiMode.split(/\//g).slice(-1);
            updateOp.planning = {
                canvasMode: {
                    intent: {
                        $set: {
                            Zone: {
                                LandUse: landUse
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

    const tools = [
        ...makeToolbar("main-toolbar", ["Inspection", "Planning"], "main", state.uiMode, setUiMode),
        ...(state.uiMode.startsWith("main/Planning")
            ? [EL(Select, {
                style: { width: 180 },
                showSearch: true,
                placeholder: "Open a proposal",
                optionFilterProp: "children",
                onChange: (value) => setState(switchToProposal(value)),
                value: state.planning.currentProposal
            },
                Object.keys(state.planning.proposals).map(proposalId =>
                    EL(Option, { value: proposalId }, "Proposal '" + proposalId.split("-")[0] + "'")
                )
            ), ...state.planning.currentProposal ? [
                EL(Button, {
                    type: "primary",
                    onClick: () => setState(implementProposal)
                }, "Implement")
            ] : []]
            : []),
        ...(state.planning.currentProposal
            ? makeToolbar("planning-toolbar", ["Roads", "Zoning"], "main/Planning", state.uiMode, setUiMode)
            : []),
        ...makeToolbar("zoning-toolbar", [
            "Residential",
            "Commercial",
            "Industrial",
            "Agricultural",
            "Recreational",
            "Official"
        ], "main/Planning/Zoning", state.uiMode, setUiMode,
            zone => {
                let c = colors[zone];
                return `rgb(${Math.pow(c[0], 1 / 2.2) * 256}, ${Math.pow(c[1], 1 / 2.2) * 256}, ${Math.pow(c[2], 1 / 2.2) * 256}`
            }
        ),
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
            cursorHover: state.uiMode.startsWith("main/Planning/") ? "crosshair" : "normal",
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
    ]

    if (state.uiMode.startsWith("main/Planning") && state.planning.currentProposal) {
        return { layers, interactables, tools };
    } else {
        return { tools };
    }
}

export function bindInputs(state, setState) {
    const inputActions = {
        "implementProposal": () => setState(implementProposal)
    }

    Mousetrap.bind(state.settings.planning.implementProposalKey, inputActions["implementProposal"]);
}