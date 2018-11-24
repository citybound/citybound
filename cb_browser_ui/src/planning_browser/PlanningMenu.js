import colors, { toCSS, fromLinFloat } from '../colors';
import React from 'react';
import { Button, Select } from 'antd';
const Option = Select.Option;

import { Toolbar } from '../toolbar';

// STATE MUTATING ACTIONS

function switchToProject(projectId) {
    console.log("switching to", projectId);

    return oldState => update(oldState, {
        planning: { currentProject: { $set: projectId } }
    })
}

function implementProject(oldState) {
    cbRustBrowser.implement_project(oldState.planning.currentProject);
    return update(oldState, {
        planning: {
            $unset: ['currentProject'],
        }
    });
}

export function render(state, setState) {
    return {
        tools: [
            <Toolbar id="main-toolbar"
                options={{ inspection: { description: "Inspection" }, planning: { description: "Planning" } }}
                value={state.uiMode}
                onChange={newMode => setState({ uiMode: newMode })} />,
            state.uiMode == 'planning' && [
                <Select
                    style={{ width: 180 }}
                    showSearch={true}
                    placeholder="Open a project"
                    optionFilterProp="children"
                    onChange={(value) => setState(switchToProject(value))}
                    value={state.planning.currentProject || undefined}
                >{Object.keys(state.planning.projects).map(projectId =>
                    <Option value={projectId}>Project '{projectId.slice(0, 3).toUpperCase()}'</Option>
                )}</Select>,
                state.planning.currentProject && [
                    <Button type="primary"
                        onClick={() => setState(implementProject)}
                    >Implement</Button>,
                    <Toolbar id="planning-history-toolbar"
                        options={{
                            undo: { description: "Undo", disabled: !state.planning.projects[state.planning.currentProject].undoable_history.length },
                            redo: { description: "Redo", disabled: !state.planning.projects[state.planning.currentProject].redoable_history.length },
                        }}
                        onChange={value => value == "undo" ? cbRustBrowser.undo(state.planning.currentProject) : cbRustBrowser.redo(state.planning.currentProject)}
                    />,
                    state.planning.currentProject &&
                    <Toolbar id="planning-toolbar"
                        options={{ roads: { description: "Roads" }, zoning: { description: "Zoning" } }}
                        value={state.planning.planningMode}
                        onChange={(value) => setState(oldState => update(oldState, {
                            planning: {
                                planningMode: { $set: value },
                                canvasMode: { intent: { $set: value == "roads" ? { Road: { n_lanes_forward: 2, n_lanes_backward: 2 } } : null } }
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
                            Official: { description: "Official", color: toCSS(fromLinFloat(colors["Official"])) }
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
        ]
    };
}

export function bindInputs(state, setState) {
    const inputActions = {
        "implementProject": () => setState(implementProject),
        "undo": () => setState(oldState => { cbRustBrowser.undo(oldState.planning.currentProject); return oldState }),
        "redo": () => setState(oldState => { cbRustBrowser.redo(oldState.planning.currentProject); return oldState })
    }

    Mousetrap.bind(state.settings.planning.implementProjectKey.key, inputActions["implementProject"]);
    Mousetrap.bind(state.settings.planning.undoKey.key, inputActions["undo"]);
    Mousetrap.bind(state.settings.planning.redoKey.key, inputActions["redo"]);
}