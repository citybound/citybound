import React from 'react';
import update from 'immutability-helper';

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
            <p>Building ID {this.props.inspectedBuilding}</p>
            <a className="close-window" onClick={this.props.closeWindow}>×</a>
            {this.props.inspectedBuildingState && [
                <h1>{this.props.inspectedBuildingState.style}</h1>,
                this.props.inspectedBuildingState.households.map(id => [
                    <h3>Household {id}</h3>,
                    this.props.householdInfo[id] && <HouseholdInfo core={this.props.householdInfo[id].core} id={id} />
                ])
            ]}
        </div>;
    }
}

function HouseholdInfo(props) {
    const { resources, member_resources, member_tasks } = props.core;

    return [
        resources.entries.map(([resource, amount]) =>
            <p>{resource}: {amount}</p>
        ),
        member_resources.map((memberResources, memberI) =>
            [
                <h4>Member {memberI}</h4>,
                member_tasks[memberI].goal && <p>Goal: {JSON.stringify(member_tasks[memberI].goal)}</p>,
                member_tasks[memberI].state && <p>State: {JSON.stringify(member_tasks[memberI].state)}</p>,
                memberResources.entries.map(([resource, amount]) =>
                    <p>{resource}: {amount}</p>
                ),
            ]
        )
    ]
}