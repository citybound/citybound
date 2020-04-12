import { PlanningMenu } from './PlanningMenu';
import * as React from 'react';
import { useState } from 'react';
import GestureCanvas from './GestureCanvas';
import { SharedState, SetSharedState } from '../citybound';
import { Intent } from '../wasm32-unknown-unknown/release/cb_browser_ui';
import { RoadPlanningLayers } from './transport_planning/RoadPlanningLayers';
import { ZonePlanningLayers, LAND_USES } from './ZonePlanningLayers';
import { RoadInteractables } from './transport_planning/RoadInteractables';
import { ControlPoints } from './ControlPoints';
import update from 'immutability-helper';

type BatchID = string;
type GroupID = string;
type GroupMesh = {};

type Project = {
    gestures: {}
}

type Mesh = {};

export type PlanningSharedState = {
    planningMode: null | "roads" | "zoning",
    rendering: {
        staticMeshes: {
            GestureDot?: Mesh,
            GestureSplit?: Mesh,
            GestureChangeNLanes?: Mesh
        },
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
    currentProject: string | null
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
    }
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
    finishGestureDistance: { default: 3.0, description: "Finish Gesture Double-Click Distance", min: 0.5, max: 10.0, step: 0.1 },
    moveVsClickPointDistance: { default: 3.0, description: "Distance That Determines Move vs. Click", min: 0.5, max: 10.0, step: 0.1 }
}


export function PlanningUI(props: { state: SharedState, setState: SetSharedState }) {
    const [planningMode, setPlanningMode] = useState<'roads' | 'zoning' | null>(null);
    const [currentProject, setCurrentProject] = [
        props.state.planning.currentProject,
        (newProject) => props.setState(oldState => update(oldState, { planning: { currentProject: { $set: newProject } } }))
    ]
    const [addToEnd, setAddToEnd] = useState<boolean>(true);
    const [editedGesture, setEditedGesture] = useState<boolean>(false);
    const [intent, setIntent] = useState<Intent | null>(null);

    const { state, setState } = props;
    return <>
        {currentProject && <>
            <ControlPoints {...{ state, setState, currentProject, planningMode, editedGesture, setEditedGesture, setAddToEnd }} />

            {!editedGesture && planningMode === "roads" && <RoadInteractables {...{ state, currentProject }} />}

            <GestureCanvas
                state={state}
                currentProject={currentProject}
                editedGesture={editedGesture}
                setEditedGesture={setEditedGesture}
                intent={intent}
                addToEnd={addToEnd}
                setAddToEnd={setAddToEnd}
            />

            <RoadPlanningLayers state={state} />
            <ZonePlanningLayers state={state} />
        </>}

        <PlanningMenu
            state={state}
            planningMode={planningMode}
            setPlanningMode={setPlanningMode}
            currentProject={currentProject}
            setCurrentProject={setCurrentProject}
            intent={intent}
            setIntent={setIntent} />
    </>
}