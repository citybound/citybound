import React from 'react';
import { vec3, vec4, mat4 } from 'gl-matrix';

export default class Stage extends React.Component {
    render() {
        return React.createElement("div", {
            style: Object.assign({}, this.props.style, {
                width: this.props.width, height: this.props.height,
                userSelect: "none",
                cursor: this.activeInteractable
                    ? (this.activeInteractable.cursorActive || "pointer")
                    : (this.hoveredInteractable ? (this.hoveredInteractable.cursorHover || "pointer") : "default")
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

                    if (!oldHoveredInteractable
                        || (oldHoveredInteractable.id !== (this.hoveredInteractable && this.hoveredInteractable.id))) {
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
        const interactables = [...this.props.interactables];
        interactables.sort((a, b) => b.zIndex - a.zIndex);
        for (let interactable of interactables) {
            let below = interactable.shape.type == "circle"
                ? vec3.dist(cursorPosition3d, interactable.shape.center) < interactable.shape.radius
                : (interactable.shape.type == "everywhere"
                    ? true
                    : false);

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