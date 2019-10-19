import React from 'react';
import { vec3, vec4, mat4 } from 'gl-matrix';

export default class Stage extends React.Component {
    render() {
        const onMouseDown = e => {
            const { eye, target, verticalFov, width, height } = this.props;
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(e, elementRect);

            const maybeInteractableInfo = this.findInteractableBelow(cursorPosition3d);
            if (maybeInteractableInfo) {
                const [newActiveInteractable, projectedPosition, direction] = maybeInteractableInfo;
                this.activeInteractable = newActiveInteractable;
                this.activeInteractable.onEvent({ drag: { start: projectedPosition }, projectedPosition, direction });
                this.dragStart = cursorPosition3d;
                this.dragStartProjectedPosition = projectedPosition;
                this.dragStartDirection = direction;
            }
        };
        const onMouseMove = e => {
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(e, elementRect);

            this.props.cursorMoved && this.props.cursorMoved(cursorPosition3d);

            if (this.activeInteractable) {
                this.activeInteractable.onEvent({
                    drag: {
                        start: this.dragStart,
                        projectedPosition: this.dragStartProjectedPosition,
                        direction: this.dragStartDirection,
                        now: cursorPosition3d,
                    }
                })
            } else {
                const oldHoveredInteractable = this.hoveredInteractable;
                const maybeInteractableInfo = this.findInteractableBelow(cursorPosition3d);
                let stopOldHover = false;

                if (maybeInteractableInfo) {
                    const [newHoveredInteractable, projectedPosition, hoveredDirection] = maybeInteractableInfo;
                    this.hoveredInteractable = newHoveredInteractable;
                    if (!oldHoveredInteractable || oldHoveredInteractable.id !== this.hoveredInteractable.id) {
                        this.hoveredInteractable.onEvent({
                            hover: {
                                start: cursorPosition3d,
                                direction: hoveredDirection,
                                projectedPosition
                            }
                        });
                        stopOldHover = true;
                    } else {
                        this.hoveredInteractable.onEvent({
                            hover: {
                                now: cursorPosition3d,
                                direction: hoveredDirection,
                                projectedPosition
                            }
                        });
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
            const elementRect = e.target.getBoundingClientRect();
            const cursorPosition3d = this.projectCursor(e, elementRect);

            if (this.activeInteractable) {
                this.activeInteractable.onEvent({
                    drag: {
                        start: this.dragStart,
                        projectedPosition: this.dragStartProjectedPosition,
                        direction: this.dragStartDirection,
                        end: cursorPosition3d,
                    }
                });
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
            onTouchEnd: e => onMouseUp(e.changedTouches[0]),
            onContextMenu: e => { e.preventDefault(); return false }
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

    projectCursor(e, elementRect) {
        const cursor2d = [
            e.clientX - elementRect.left,
            e.clientY - elementRect.top
        ];

        return this.props.project2dTo3d(cursor2d);
    }
}
