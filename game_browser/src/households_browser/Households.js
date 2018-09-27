import React from 'react';
import * as cityboundBrowser from '../../Cargo.toml';
import update from 'immutability-helper';

export const initialState = {
    buildingPositions: {},
    inspectedBuilding: null,
    inspectedBuildingState: null,
    householdInfo: {},
};

export function render(state, setState) {
    if (state.uiMode != "inspection") {
        return {}
    }

    const { inspectedBuilding, inspectedBuildingState, householdInfo } = state.households;

    const windows = inspectedBuilding && <BuildingInfo {...{ inspectedBuilding, inspectedBuildingState, householdInfo }} />;

    const interactables = Object.keys(state.households.buildingPositions).map(buildingId => {
        const buildingPosition2d = state.households.buildingPositions[buildingId];
        const buildingPosition = [buildingPosition2d[0], buildingPosition2d[1], 0];

        return {
            id: buildingId,
            shape: {
                type: "circle",
                center: buildingPosition,
                radius: 3
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
            cityboundBrowser.get_building_info(this.props.inspectedBuilding);
            if (this.props.inspectedBuildingState) {
                for (let householdId of this.props.inspectedBuildingState.households) {
                    cityboundBrowser.get_household_info(householdId);
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
                member_tasks[memberI].goal && <p>Goal: {member_tasks[memberI].goal}</p>,
                member_tasks[memberI].state && <p>Goal: {JSON.stringify(member_tasks[memberI].state)}</p>,
                memberResources.entries.map(([resource, amount]) =>
                    <p>{resource}: {amount}</p>
                ),
            ]
        )
    ]
}