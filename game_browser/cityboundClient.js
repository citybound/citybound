import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import { vec3, vec4, mat4 } from 'gl-matrix';
import ContainerDimensions from 'react-container-dimensions';
import update from 'immutability-helper';

import colors from './colors';

window.update = update;

const EL = React.createElement;

class CityboundClient extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            planning: {
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
                hoveredGesturePoint: {}
            },
            transport: {
                rendering: {
                    laneAsphalt: {},
                    laneMarker: {},
                    laneMarkerGap: {}
                }
            },
            view: {
                eye: [-150, -150, 150],
                target: [0, 0, 0],
                verticalFov: 0.3 * Math.PI
            }
        }

    }

    switchToProposal(proposalId) {
        console.log("switching to", proposalId);
        this.setState(oldState => update(oldState, {
            planning: { currentProposal: { $set: proposalId } }
        }))
    }

    moveGesturePoint(proposalId, gestureId, pointIdx, newPosition) {

    }

    render() {
        const gesturePointInstances = [];
        const gesturePointInteractables = [];

        if (this.state.planning) {
            let gestures = Object.keys(this.state.planning.master.gestures).map(gestureId =>
                ({ [gestureId]: Object.assign(this.state.planning.master.gestures[gestureId], { fromMaster: true }) })
            ).concat(this.state.planning.currentProposal
                ? this.state.planning.proposals[this.state.planning.currentProposal].undoable_history.map(step => step.gestures)
                : []
            ).reduce((coll, gestures) => Object.assign(coll, gestures), {});

            let { gestureId: hoveredGestureId, pointIdx: hoveredPointIdx } = this.state.planning.hoveredGesturePoint;

            for (let gestureId of Object.keys(gestures)) {
                const gesture = gestures[gestureId];

                for (let [pointIdx, point] of gesture.points.entries()) {
                    let isHovered = gestureId == hoveredGestureId && pointIdx == hoveredPointIdx;
                    gesturePointInstances.push.apply(gesturePointInstances, [
                        point[0], point[1], 0,
                        1.0, 0.0,
                        ...(isHovered
                            ? colors.gesturePointHover
                            : (gesture.fromMaster ? colors.gesturePointMaster : colors.gesturePointCurrentProposal))
                    ]);

                    gesturePointInteractables.push({
                        shape: {
                            type: "circle",
                            center: [point[0], point[1], 0],
                            radius: 3
                        },
                        onEvent: e => {
                            if (e.hover) {
                                if (e.hover.start) {
                                    this.setState(update(this.state, {
                                        planning: {
                                            hoveredGesturePoint: {
                                                $set: { gestureId, pointIdx }
                                            }
                                        }
                                    }))
                                } else if (e.hover.end) {
                                    this.setState(update(this.state, {
                                        planning: {
                                            hoveredGesturePoint: {
                                                $set: {}
                                            }
                                        }
                                    }))
                                }
                            }

                            if (e.drag && e.drag.now) {
                                this.moveGesturePoint(this.state.planning.currentProposal, gestureId, pointIdx, e.drag.now);
                            }
                        }
                    })
                }
            }
        }

        const layers = [
            {
                decal: true,
                batches: Object.values(this.state.transport.rendering.laneAsphalt).map(laneAsphalt => ({
                    mesh: laneAsphalt,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt])
                }))
            },
            {
                decal: true,
                batches: Object.values(this.state.transport.rendering.laneMarker).map(laneMarker => ({
                    mesh: laneMarker,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.roadMarker])
                }))
            },
            {
                decal: true,
                batches: Object.values(this.state.transport.rendering.laneMarkerGap).map(laneMarkerGap => ({
                    mesh: laneMarkerGap,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.asphalt])
                }))
            },
            {
                decal: true,
                batches: [{
                    mesh: this.state.planning.rendering.currentPreview.lanesToDestruct,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.destructedAsphalt])
                }]
            },
            {
                decal: true,
                batches: [{
                    mesh: this.state.planning.rendering.currentPreview.lanesToConstruct,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt])
                }]
            },
            {
                decal: true,
                batches: [{
                    mesh: this.state.planning.rendering.currentPreview.lanesToConstructMarker,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedRoadMarker])
                }]
            },
            {
                decal: true,
                batches: [{
                    mesh: this.state.planning.rendering.currentPreview.switchLanesToConstructMarkerGap,
                    instances: new Float32Array([0.0, 0.0, 0.0, 1.0, 0.0, ...colors.plannedAsphalt])
                }]
            },
            {
                decal: true,
                batches: [{
                    mesh: this.state.planning.rendering.staticMeshes.GestureDot,
                    instances: new Float32Array(gesturePointInstances)
                }]
            },
        ];

        if (this.state.cursor3d) {
            const [x, y, z] = this.state.cursor3d;
            layers.push({
                decal: true,
                batches: [
                    {
                        mesh: this.state.meshes.GestureDot,
                        instances: new Float32Array([x, y, z, 1.0, 0.0, 0.3, 0.3, 0.0])
                    }
                ]
            })
        }

        const { eye, target, verticalFov } = this.state.view;

        return EL("div", {
            style: { width: "100%", height: "100%" },
            onWheel: e => {
                const forward = vec3.sub(vec3.create(), target, eye);
                forward[2] = 0;
                vec3.normalize(forward, forward);
                const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

                const delta = vec3.scaleAndAdd(vec3.create(), vec3.scale(vec3.create(), forward, -e.deltaY), sideways, -e.deltaX);

                this.setState(oldState => ({
                    view: Object.assign(oldState.view, {
                        eye: vec3.add(vec3.create(), oldState.view.eye, delta),
                        target: vec3.add(vec3.create(), oldState.view.target, delta),
                    })
                }));

                e.preventDefault();
                return false;
            }
        },
            EL(ContainerDimensions, { style: { width: "100%", height: "100%", position: "relative" } }, ({ width, height }) => {
                const viewMatrix = mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]);
                const perspectiveMatrix = mat4.perspective(mat4.create(), verticalFov, width / height, 50000, 0.1);

                return EL("div", { style: { width, height } }, [
                    EL("div", { key: "ui2d", className: "ui2d" }, [
                        EL("div", { className: "window proposals" }, [
                            EL("h1", {}, "Proposals"),
                            ...Object.keys(this.state.planning.proposals).map(proposalId =>
                                proposalId == this.state.planning.currentProposal
                                    ? EL("p", {}, "" + proposalId)
                                    : EL("button", {
                                        onClick: () => this.switchToProposal(proposalId)
                                    }, "" + proposalId)
                            )
                        ])
                    ]),
                    EL(Monet, {
                        key: "canvas",
                        layers,
                        width, height,
                        viewMatrix, perspectiveMatrix,
                        clearColor: [...colors.grass, 1.0]
                    }),
                    EL(Stage, {
                        key: "stage",
                        interactables: gesturePointInteractables,
                        width, height,
                        eye, target, verticalFov,
                        style: { width, height, position: "absolute", top: 0, left: 0 }
                    })
                ])
            })
        );
    }
}

class Stage extends React.Component {
    render() {
        return EL("div", {
            style: Object.assign({}, this.props.style, {
                width: this.props.width, height: this.props.height,
                cursor: this.hoveredInteractable ? "pointer" : "default"
            }),
            onMouseMove: e => {
                const { eye, target, verticalFov, width, height } = this.props;
                const elementRect = e.target.getBoundingClientRect();
                const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

                this.props.cursorMoved && this.props.cursorMoved(cursorPosition3d);

                if (this.activeInteractable) {
                    this.activeInteractable.onEvent({ drag: { start: this.dragStart, now: cursorPosition3d } })
                } else {
                    const oldHoveredInteractable = this.hoveredInteractable;
                    this.hoveredInteractable = this.findInteractableBelow(cursorPosition3d);

                    if (oldHoveredInteractable != this.hoveredInteractable) {
                        oldHoveredInteractable && oldHoveredInteractable.onEvent({ hover: { end: cursorPosition3d } });
                        this.hoveredInteractable && this.hoveredInteractable.onEvent({ hover: { start: cursorPosition3d } });
                    } else {
                        this.hoveredInteractable && this.hoveredInteractable.onEvent({ hover: { now: cursorPosition3d } });
                    }
                }
            },
            onMouseDown: e => {
                const { eye, target, verticalFov, width, height } = this.props;
                const elementRect = e.target.getBoundingClientRect();
                const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

                this.activeInteractable = this.findInteractableBelow(cursorPosition3d);
                this.activeInteractable && this.activeInteractable.onEvent({ drag: { start: cursorPosition3d } });
                this.dragStart = cursorPosition3d;
            },
            onMouseUp: e => {
                const { eye, target, verticalFov, width, height } = this.props;
                const elementRect = e.target.getBoundingClientRect();
                const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

                if (this.activeInteractable) {
                    this.activeInteractable.onEvent({ drag: { start: this.dragStart, end: cursorPosition3d } });
                    this.activeInteractable = null;
                    this.dragStart = null;
                }
            }
        });
    }

    findInteractableBelow(cursorPosition3d) {
        for (let interactable of this.props.interactables) {
            let below = interactable.shape.type == "circle"
                ? vec3.dist(cursorPosition3d, interactable.shape.center) < interactable.shape.radius
                : false;

            if (below) {
                return interactable;
            }
        }

        return null;
    }

    projectCursor(eye, target, verticalFov, width, height, e, elementRect) {
        const cursor2dX = e.clientX - elementRect.left;
        const cursor2dY = e.clientY - elementRect.top;

        const normalized2dPosition = [
            ((cursor2dX / width) * 2.0) - 1.0,
            ((-cursor2dY / height) * 2.0) + 1.0,
            -1.0,
            1.0
        ];

        const inverseView = mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]);
        mat4.invert(inverseView, inverseView);
        const inversePerspectiveMatrix = mat4.perspective(mat4.create(), verticalFov, width / height, 50000, 0.1);
        mat4.invert(inversePerspectiveMatrix, inversePerspectiveMatrix);

        const positionFromCamera = vec4.transformMat4(vec4.create(), normalized2dPosition, inversePerspectiveMatrix);
        positionFromCamera[3] = 0;

        const directionIntoWorld = vec4.transformMat4(vec4.create(), positionFromCamera, inverseView);

        const distance = -eye[2] / directionIntoWorld[2];
        const cursorPosition3d = vec3.scaleAndAdd(vec3.create(), eye, directionIntoWorld, distance);
        return cursorPosition3d;
    }
}

window.cbclient = ReactDOM.render(EL(CityboundClient), document.getElementById('app'));

import cityboundBrowser from './Cargo.toml';

cityboundBrowser.start();