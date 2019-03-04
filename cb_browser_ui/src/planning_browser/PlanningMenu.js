import colors, { toCSS, fromLinFloat } from '../colors';
import React from 'react';
import { Button, Select, Divider, Icon } from 'antd';
const Option = Select.Option;
import uuid from '../uuid';

import { Toolbar } from '../toolbar';

// STATE MUTATING ACTIONS

function switchToProject(projectId) {
    console.log("switching to", projectId);

    return oldState => update(oldState, {
        planning: { currentProject: { $set: projectId } }
    })
}

function implementProject(oldState) {
    if (oldState.planning.currentProject) {
        cbRustBrowser.implement_project(oldState.planning.currentProject);
        return update(oldState, {
            planning: {
                $unset: ['currentProject'],
            }
        });
    }
    return oldState;
}

function startNewProject(oldState) {
    const projectId = uuid();
    cbRustBrowser.start_new_project(projectId);
    return update(oldState, {
        planning: {
            currentProject: { $set: projectId },
        }
    });
}

function undo(oldState) {
    if (oldState.planning.currentProject) {
        cbRustBrowser.undo(oldState.planning.currentProject);
        if (oldState.planning.canvasMode.currentGesture) {
            const project = oldState.planning.projects[oldState.planning.currentProject];
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
    return oldState;
}

function redo(oldState) {
    if (oldState.planning.currentProject) cbRustBrowser.redo(oldState.planning.currentProject);
    return oldState
}

export function Tools(props) {
    const { state, setState } = props;
    return [
        <Toolbar id="main-toolbar"
            options={{ inspection: { description: "Inspection" }, planning: { description: "Planning" } }}
            value={state.uiMode}
            onChange={newMode => setState({ uiMode: newMode })} />,
        state.uiMode == 'planning' && [
            (state.planning.currentProject || Object.keys(state.planning.projects).length > 0)
                ? <Select
                    style={{ width: 180 }}
                    showSearch={true}
                    placeholder="Open a project"
                    optionFilterProp="children"
                    notFoundContent="No ongoing projects"
                    onChange={(value) => setState(switchToProject(value))}
                    value={state.planning.projects[state.planning.currentProject] ? state.planning.currentProject : (state.planning.currentProject ? "Opening..." : undefined)}
                    dropdownRender={menu => (
                        <div>
                            <div style={{ padding: '8px', cursor: 'pointer' }} onClick={() => setState(startNewProject)}>
                                <Icon type="plus" /> Start another project
                            </div>
                            <Divider style={{ margin: '4px 0' }} />
                            {menu}
                        </div>
                    )}
                >{Object.keys(state.planning.projects).map(projectId =>
                    <Option value={projectId}>Project '{projectId.slice(0, 3).toUpperCase()}'</Option>
                )}</Select>
                : <Button type="primary" onClick={() => setState(startNewProject)}>Start new project</Button>,
            state.planning.currentProject && [
                <Button type="primary"
                    onClick={() => setState(implementProject)}
                >Implement</Button>,
                <Toolbar id="planning-history-toolbar"
                    options={{
                        undo: { description: "Undo", disabled: !state.planning.projects[state.planning.currentProject] || !state.planning.projects[state.planning.currentProject].undoable_history.length },
                        redo: { description: "Redo", disabled: !state.planning.projects[state.planning.currentProject] || !state.planning.projects[state.planning.currentProject].redoable_history.length },
                    }}
                    onChange={value => value == "undo" ? setState(undo) : setState(redo)}
                />,
                state.planning.currentProject &&
                <Toolbar id="planning-toolbar"
                    options={{ roads: { description: "Roads" }, zoning: { description: "Zoning" } }}
                    value={state.planning.planningMode}
                    onChange={(value) => setState(oldState => update(oldState, {
                        planning: {
                            planningMode: { $set: value },
                            canvasMode: { intent: { $set: value == "roads" ? { Road: { n_lanes_forward: 1, n_lanes_backward: 1 } } : null } }
                        }
                    }))} />,
                state.planning.currentProject && state.planning.planningMode == "zoning" &&
                <Toolbar id="zoning-toolbar"
                    options={{
                        Residential: { description: "Residential", color: toCSS(fromLinFloat(colors["Residential"])) },
                        Commercial: { description: "Commercial", color: toCSS(fromLinFloat(colors["Commercial"])) },
                        Industrial: { description: "Industrial", color: toCSS(fromLinFloat(colors["Industrial"])) },
                        Agricultural: { description: "Agricultural", color: toCSS(fromLinFloat(colors["Agricultural"])) },
                        Recreational: { description: "Recreational", color: toCSS(fromLinFloat(colors["Recreational"])) },
                        Administrative: { description: "Administrative", color: toCSS(fromLinFloat(colors["Administrative"])) }
                    }}
                    value={state.planning.canvasMode.intent && state.planning.canvasMode.intent.Zone && state.planning.canvasMode.intent.Zone.LandUse}
                    onChange={newLandUse => setState(oldState => update(oldState, {
                        planning: {
                            canvasMode: {
                                intent: { $set: { Zone: { LandUse: newLandUse } } }
                            }
                        }
                    }))} />
            ]
        ]
    ];
}

export function bindInputs(state, setState) {
    const inputActions = {
        "implementProject": () => setState(implementProject),
        "undo": () => setState(undo),
        "redo": () => setState(redo)
    }

    Mousetrap.bind(state.settings.planning.implementProjectKey.key, inputActions["implementProject"]);
    Mousetrap.bind(state.settings.planning.undoKey.key, inputActions["undo"]);
    Mousetrap.bind(state.settings.planning.redoKey.key, inputActions["redo"]);
}