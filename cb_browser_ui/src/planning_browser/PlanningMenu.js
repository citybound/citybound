import colors, { toCSS, fromLinFloat } from '../colors';
import React from 'react';
import { Button, Select } from 'antd';
const Option = Select.Option;

import { Toolbar } from '../toolbar';

// STATE MUTATING ACTIONS

function switchToProposal(proposalId) {
    console.log("switching to", proposalId);

    return oldState => update(oldState, {
        planning: { currentProposal: { $set: proposalId } }
    })
}

function implementProposal(oldState) {
    cbRustBrowser.implement_proposal(oldState.planning.currentProposal);
    return update(oldState, {
        planning: {
            $unset: ['currentProposal'],
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
                    placeholder="Open a proposal"
                    optionFilterProp="children"
                    onChange={(value) => setState(switchToProposal(value))}
                    value={state.planning.currentProposal || undefined}
                >{Object.keys(state.planning.proposals).map(proposalId =>
                    <Option value={proposalId}>Proposal '{proposalId.split("-")[0]}'</Option>
                )}</Select>,
                state.planning.currentProposal && [
                    <Button type="primary"
                        onClick={() => setState(implementProposal)}
                    >Implement</Button>,
                    <Toolbar id="planning-history-toolbar"
                        options={{
                            undo: { description: "Undo", disabled: !state.planning.proposals[state.planning.currentProposal].undoable_history.length },
                            redo: { description: "Redo", disabled: !state.planning.proposals[state.planning.currentProposal].redoable_history.length },
                        }}
                        onChange={value => value == "undo" ? cbRustBrowser.undo(state.planning.currentProposal) : cbRustBrowser.redo(state.planning.currentProposal)}
                    />,
                    state.planning.currentProposal &&
                    <Toolbar id="planning-toolbar"
                        options={{ roads: { description: "Roads" }, zoning: { description: "Zoning" } }}
                        value={state.planning.planningMode}
                        onChange={(value) => setState(oldState => update(oldState, {
                            planning: {
                                planningMode: { $set: value },
                                canvasMode: { intent: { $set: value == "roads" ? { Road: { n_lanes_forward: 2, n_lanes_backward: 2 } } : null } }
                            }
                        }))} />,
                    state.planning.currentProposal && state.planning.planningMode == "zoning" &&
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
        "implementProposal": () => setState(implementProposal),
        "undo": () => setState(oldState => { cbRustBrowser.undo(oldState.planning.currentProposal); return oldState }),
        "redo": () => setState(oldState => { cbRustBrowser.redo(oldState.planning.currentProposal); return oldState })
    }

    Mousetrap.bind(state.settings.planning.implementProposalKey.key, inputActions["implementProposal"]);
    Mousetrap.bind(state.settings.planning.undoKey.key, inputActions["undo"]);
    Mousetrap.bind(state.settings.planning.redoKey.key, inputActions["redo"]);
}