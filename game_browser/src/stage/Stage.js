import React from 'react';
import { vec3, vec4, mat4 } from 'gl-matrix';

export default class Stage extends React.Component {
    render() {
        const onMouseDown = e => {
            const { eye, target, verticalFov, width, height } = this.props;
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

            const maybeInteractableInfo = this.findInteractableBelow(cursorPosition3d);
            if (maybeInteractableInfo) {
                const [newActiveInteractable, hoveredPosition, hoveredDirection] = maybeInteractableInfo;
                this.activeInteractable = newActiveInteractable;
                this.activeInteractable.onEvent({ drag: { start: hoveredPosition } });
                this.dragStart = cursorPosition3d;
            }
        };
        const onMouseMove = e => {
            const { eye, target, verticalFov, width, height } = this.props;
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

            this.props.cursorMoved && this.props.cursorMoved(cursorPosition3d);

            if (this.activeInteractable) {
                this.activeInteractable.onEvent({ drag: { start: this.dragStart, now: cursorPosition3d } })
            } else {
                const oldHoveredInteractable = this.hoveredInteractable;
                const maybeInteractableInfo = this.findInteractableBelow(cursorPosition3d);
                let stopOldHover = false;

                if (maybeInteractableInfo) {
                    const [newHoveredInteractable, hoveredPosition, hoveredDirection] = maybeInteractableInfo;
                    this.hoveredInteractable = newHoveredInteractable;
                    if (!oldHoveredInteractable || oldHoveredInteractable.id !== this.hoveredInteractable.id) {
                        this.hoveredInteractable.onEvent({ hover: { start: hoveredPosition } });
                        stopOldHover = true;
                    } else {
                        this.hoveredInteractable.onEvent({ hover: { now: hoveredPosition, direction: hoveredDirection } });
                    }
                } else {
                    stopOldHover = true;
                }

                if (stopOldHover) {
                    oldHoveredInteractable && oldHoveredInteractable.onEvent({ hover: { end: cursorPosition3d } });
                }
            }

            this.props.onMouseMove(e);
        };
        const onMouseUp = e => {
            const { eye, target, verticalFov, width, height } = this.props;
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(eye, target, verticalFov, width, height, e, elementRect);

            if (this.activeInteractable) {
                this.activeInteractable.onEvent({ drag: { start: this.dragStart, end: cursorPosition3d } });
                this.activeInteractable = null;
                this.dragStart = null;
            }
        };

        return React.createElement("div", {
            style: Object.assign({}, this.props.style, {
                width: this.props.width, height: this.props.height,
                userSelect: "none",
                cursor: this.activeInteractable
                    ? (this.activeInteractable.cursorActive || "pointer")
                    : (this.hoveredInteractable ? (this.hoveredInteractable.cursorHover || "pointer") : "default")
            }),
            onWheel: this.props.onWheel,
            onMouseMove,
            onMouseDown,
            onMouseUp,
            onPointerDown: onMouseDown,
            onPointerMove: onMouseMove,
            onPointerUp: onMouseUp,
            onTouchStart: e => onMouseDown(e.changedTouches[0]),
            onTouchMove: e => onMouseMove(e.changedTouches[0]),
            onTouchEnd: e => onMouseUp(e.changedTouches[0])
        });
    }

    findInteractableBelow(cursorPosition3d) {
        const interactables = [...this.props.interactables];
        interactables.sort((a, b) => b.zIndex - a.zIndex);
        for (let interactable of interactables) {
            if (interactable.shape.type == "circle" && vec3.dist(cursorPosition3d, interactable.shape.center) < interactable.shape.radius) {
                return [interactable, cursorPosition3d, null];
            } else if (interactable.shape.type == "polygon" && cbRustBrowser.point_in_area([cursorPosition3d[0], cursorPosition3d[1]], interactable.shape.area)) {
                return [interactable, cursorPosition3d, null];
            } else if (interactable.shape.type == "path") {
                const maybeProjected = cbRustBrowser.point_close_to_path([cursorPosition3d[0], cursorPosition3d[1]], interactable.shape.path, interactable.shape.maxDistanceRight, interactable.shape.maxDistanceLeft);
                if (maybeProjected) {
                    const [point, projectedPoint, direction] = maybeProjected;
                    return [interactable, [...projectedPoint, 0.0], [...direction, 0.0]];
                }
            } else if (interactable.shape.type == "everywhere") {
                return [interactable, cursorPosition3d, null];
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