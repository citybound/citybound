import colors, { toCSS, fromLinFloat } from '../colors';
import * as React from 'react';
import { useCallback } from 'react';
import { Button, Select, Divider, Icon } from 'antd';
const Option = Select.Option;
import uuid from '../uuid';

import { Toolbar } from '../toolbar';
import { SharedState, ToToolPortal } from '../citybound';
import { Intent } from '../wasm32-unknown-unknown/release/cb_browser_ui';
import { useInputBinding } from '../browser_utils/Utils';

export function PlanningMenu(
    { state, currentProject, setCurrentProject, planningMode, setPlanningMode, intent, setIntent }:
        { state: SharedState, currentProject: string | null, setCurrentProject: (project: string) => void, intent: Intent | null, setIntent: (intent: Intent | null) => void, planningMode: 'roads' | 'zoning' | null, setPlanningMode: (mode: 'roads' | 'zoning' | null) => void }) {

    const startNewProject = useCallback(() => {
        const projectId = uuid();
        cbRustBrowser.start_new_project(projectId);
        setCurrentProject(projectId);
    }, [setCurrentProject])

    const implementProject = useCallback(() => {
        if (currentProject) {
            cbRustBrowser.implement_project(currentProject);
        }
        setCurrentProject(null);
    }, [currentProject, setCurrentProject]);

    const undo = useCallback(() => {
        if (currentProject) {
            cbRustBrowser.undo(currentProject);
            if (oldState.planning.canvasMode.currentGesture) {
                const project = oldState.planning.projects[currentProject];
                const lastHistoryStep = project && project.undoable_history[project.undoable_history.length - 2];
                if (!lastHistoryStep || !lastHistoryStep.gestures[oldState.planning.canvasMode.currentGesture]) {
                    return update(oldState, {
                        planning: {
                            canvasMode: {
                                $unset: ['currentGesture', 'previousClick']
                            }
                        }
                    });
                }
            }
        }
    }, [currentProject]);

    const redo = useCallback(() => {
        if (currentProject) cbRustBrowser.redo(currentProject);
    }, [currentProject]);

    useInputBinding({
        [state.settings.planning.implementProjectKey.key]: implementProject,
        [state.settings.planning.undoKey.key]: undo,
        [state.settings.planning.redoKey.key]: redo,
    });

    return <ToToolPortal>
        {(currentProject || Object.keys(state.planning.projects).length > 0)
            ? <Select
                style={{ width: 180 }}
                showSearch={true}
                placeholder="Open a project"
                optionFilterProp="children"
                notFoundContent="No ongoing projects"
                onChange={setCurrentProject}
                value={state.planning.projects[currentProject] ? currentProject : (currentProject ? "Opening..." : undefined)}
                dropdownRender={menu => (
                    <div>
                        <div style={{ padding: '8px', cursor: 'pointer' }} onClick={startNewProject}>
                            <Icon type="plus" /> Start another project
                            </div>
                        <Divider style={{ margin: '4px 0' }} />
                        {menu}
                    </div>
                )}
            >{Object.keys(state.planning.projects).map(projectId =>
                <Option value={projectId}>Project '{projectId.slice(0, 3).toUpperCase()}'</Option>
            )}</Select>
            : <Button type="primary" onClick={startNewProject}>Start new project</Button>}

        {currentProject &&
            <Button type="primary" onClick={implementProject} >Implement</Button>}

        <Toolbar id="planning-history-toolbar"
            options={{
                undo: { description: "Undo", disabled: !state.planning.projects[currentProject] || !state.planning.projects[currentProject].undoable_history.length },
                redo: { description: "Redo", disabled: !state.planning.projects[currentProject] || !state.planning.projects[currentProject].redoable_history.length },
            }}
            onChange={value => value == "undo" ? undo : redo}
        />

        {currentProject &&
            <Toolbar id="planning-toolbar"
                options={{ roads: { description: "Roads" }, zoning: { description: "Zoning" } }}
                value={planningMode}
                onChange={
                    (mode) => {
                        setPlanningMode(mode);
                        if (mode === 'roads') {
                            setIntent({ Road: cbRustBrowser.new_road_intent(2, 2) })
                        } else {
                            setIntent(null);
                        }
                    }
                } />}
        {currentProject && planningMode == "zoning" &&
            <Toolbar id="zoning-toolbar"
                options={{
                    Residential: { description: "Residential", color: toCSS(fromLinFloat(colors["Residential"])) },
                    Commercial: { description: "Commercial", color: toCSS(fromLinFloat(colors["Commercial"])) },
                    Industrial: { description: "Industrial", color: toCSS(fromLinFloat(colors["Industrial"])) },
                    Agricultural: { description: "Agricultural", color: toCSS(fromLinFloat(colors["Agricultural"])) },
                    Recreational: { description: "Recreational", color: toCSS(fromLinFloat(colors["Recreational"])) },
                    Administrative: { description: "Administrative", color: toCSS(fromLinFloat(colors["Administrative"])) }
                }}
                value={intent && intent.Zone && intent.Zone.config.land_use}
                onChange={newLandUse => setIntent({ Zone: cbRustBrowser.new_zone_intent(newLandUse) })}
            />}
    </ToToolPortal>
}