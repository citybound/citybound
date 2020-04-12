import * as React from 'react';
import { ToToolPortal, ToWindowPortal } from './citybound';
import { Toolbar } from './toolbar';
import { PlanningUI } from './planning_browser/Planning';
import * as Households from './households_browser/Households';
import update from 'immutability-helper';

export default function MainUIModes(props: { state, setState, project2dTo3d, project3dTo2d }) {
    const [uiMode, setUIMode] = [
        props.state.uiMode,
        (newMode) => props.setState(oldState => update(oldState, { uiMode: { $set: newMode } }))
    ]

    return <>
        <ToToolPortal>
            <Toolbar id="main-toolbar"
                options={{ inspection: { description: "Inspection" }, planning: { description: "Planning" } }}
                value={uiMode}
                onChange={setUIMode} />
        </ToToolPortal>
        {uiMode === 'inspection'
            ? <InspectionUI state={props.state} setState={props.setState} project2dTo3d={props.project2dTo3d} project3dTo2d={props.project3dTo2d} />
            : uiMode === 'planning'
                ? <PlanningUI state={props.state} setState={props.setState} />
                : null
        }
    </>
}

function InspectionUI(props: { state, setState, project2dTo3d, project3dTo2d }) {
    return <>
        <Households.Shapes state={props.state} setState={props.setState} project2dTo3d={props.project2dTo3d} project3dTo2d={props.project3dTo2d} />
        <ToWindowPortal>
            <Households.Windows state={props.state} setState={props.setState} project2dTo3d={props.project2dTo3d} project3dTo2d={props.project3dTo2d} />
        </ToWindowPortal>
    </>
}