import React from 'react';
import update from 'immutability-helper';
import { fmtId } from '../browser_utils/Utils';

export const initialState = {
    buildingPositions: {},
    buildingShapes: {},
    inspectedBuilding: null,
    inspectedBuildingState: null,
    householdInfo: {},
};

export function render(state, setState) {
    if (state.uiMode != "inspection") {
        return {}
    }

    const { inspectedBuilding, inspectedBuildingState, householdInfo } = state.households;

    const closeWindow = () => setState(oldState => update(oldState, {
        households: {
            inspectedBuilding: { $set: null },
            inspectedBuildingState: { $set: null },
        }
    }));

    const windows = inspectedBuilding && <BuildingInfo {...{ inspectedBuilding, inspectedBuildingState, householdInfo, closeWindow }} />;

    const interactables = Object.keys(state.households.buildingShapes).map(buildingId => {
        const buildingShape = state.households.buildingShapes[buildingId];

        return {
            id: buildingId,
            shape: {
                type: "polygon",
                area: buildingShape,
            },
            zIndex: 2,
            cursorHover: "pointer",
            cursorActive: "pointer",
            onEvent: e => {
                if (e.drag && e.drag.end) {
                    setState(oldState => update(oldState, {
                        households: { inspectedBuilding: { $set: buildingId } }
                    }))
                }
            }
        }
    })

    return { windows, interactables };
}

class BuildingInfo extends React.Component {
    constructor(props) {
        super(props);

        this.refresh = () => {
            cbRustBrowser.get_building_info(this.props.inspectedBuilding);
            if (this.props.inspectedBuildingState) {
                for (let householdId of this.props.inspectedBuildingState.households) {
                    cbRustBrowser.get_household_info(householdId);
                }
            }
        }
    }

    componentWillMount() {
        this.refresh();
        this.refreshInterval = setInterval(this.refresh, 1000);
    }

    componentWillUnmount() {
        clearInterval(this.refreshInterval);
    }

    render() {
        return <div className="window building">
            <p>{fmtId(this.props.inspectedBuilding)}</p>
            <a className="close-window" onClick={this.props.closeWindow}>×</a>
            {this.props.inspectedBuildingState && [
                <h1>{this.props.inspectedBuildingState.style}</h1>,
                this.props.inspectedBuildingState.households.map(id => [
                    <h3>{fmtId(id)}</h3>,
                    this.props.householdInfo[id] && <HouseholdInfo core={this.props.householdInfo[id].core} id={id} here={this.props.inspectedBuilding} />
                ])
            ]}
        </div>;
    }
}

function HouseholdInfo(props) {
    const { resources, member_resources, member_tasks } = props.core;

    return [
        resources.entries.map(([resource, amount]) =>
            <p>{resource}: {amount.toFixed(2)}</p>
        ),
        member_resources.map((memberResources, memberI) =>
            [
                <h4>Member {memberI}</h4>,
                <p><StateAndGoal here={props.here} state={member_tasks[memberI].state} goal={member_tasks[memberI].goal} /></p>,
                memberResources.entries.map(([resource, amount]) =>
                    <p>{resource}: {amount.toFixed(2)}</p>
                ),
            ]
        )
    ]
}

function StateAndGoal(props) {
    let statePart;
    let goalGerund = false;

    if (props.state) {
        if (props.state.IdleAt) {
            if (props.state.IdleAt == props.here) {
                statePart = "Idle here";
            } else {
                statePart = "Idle at " + fmtId(props.state.IdleAt) + (props.goal ? " after" : "");
                goalGerund = true;
            }
        } else if (props.state.StartedAt) {
            statePart = "Currently";
            goalGerund = true;
        } else if (props.state.InTrip) {
            statePart = "On the way to"
        } else {
            statePart = JSON.stringify(props)
        }

        let goalPart;

        if (props.goal) {
            if (props.goal[0] == "Money") {
                goalPart = (goalGerund ? "working at " : "work at ") + fmtId(props.goal[1].household)
            } else if (props.goal[0] == "Wakefulness") {
                goalPart = (goalGerund ? "sleeping at " : "sleep at ") + fmtId(props.goal[1].household)
            } else {
                goalPart = (goalGerund ? "getting " : "get ") + props.goal[0].toLowerCase() + " at " + fmtId(props.goal[1].household)
            }
            return statePart + " " + goalPart + ".";
        } else {
            return statePart + ".";
        }
    } else {
        return "Gone missing?"
    }
}