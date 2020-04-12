import * as React from 'react';
import { SharedState } from '../../citybound';
import { InsertControlPointInteractable } from './InsertControlPointInteractable';
import { SplitControlPointInteractable } from './SplitControlPointInteractable';
import { ChangeNLanesInteractable } from './ChangeNLanesInteractable';

export function RoadInteractables({ state, currentProject }: {
    state: SharedState;
    currentProject: string;
}) {
    return <>
        {Object.keys(state.planning.rendering.roadInfos).map(gestureId => {
            let { centerLine, outline, nLanesForward, nLanesBackward } = state.planning.rendering.roadInfos[gestureId];
            return <>
                <InsertControlPointInteractable {...{ gestureId, centerLine, state, currentProject }} />
                <SplitControlPointInteractable {...{ gestureId, centerLine, nLanesBackward, nLanesForward, state, currentProject }} />
                <ChangeNLanesInteractable {...{ gestureId, centerLine, nLanesBackward, nLanesForward, state, currentProject }} />
            </>;
        })}
    </>;
}
