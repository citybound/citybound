import React from 'react';
import { Toolbar } from './toolbar';
import { Settings } from './settings';
import { Collapse, Checkbox, Tabs, Progress } from 'antd';
import aePlayLogo from '../assets/ae_play.png';

const Panel = Collapse.Panel;
const TabPane = Tabs.TabPane;

const EL = React.createElement;

export const initalState = {
    visible: true,
    tabKey: "about"
}

export function render(state, settingsSpecs, setState) {
    let tools = EL(Toolbar, {
        id: "menu-toolbar",
        options: { menu: { description: "Menu" } },
        value: state.menu.visible && "menu",
        onChange: () =>
            setState(oldState => update(oldState, { menu: { visible: { $apply: old => !old } } }))
    });

    let svg = <svg width="197px" height="57px" viewBox="0 0 197 57" version="1.1" xmlns="http://www.w3.org/2000/svg">
        <g id="Citybound" stroke="none" stroke-width="1" fill="none" fill-rule="evenodd">
            <path d="M7.6501651,43.129 C3.5631651,41.99 0.749165098,38.305 0.749165098,34.084 L0.749165098,11.304 C0.749165098,7.083 3.5631651,3.398 7.6501651,2.326 C11.7371651,1.187 16.0251651,2.996 18.1021651,6.614 C18.5041651,7.284 18.3031651,8.021 17.6331651,8.423 C17.0301651,8.758 16.2261651,8.557 15.8911651,7.954 C14.3501651,5.274 11.2681651,4.001 8.3201651,4.805 C5.3721651,5.542 3.2951651,8.289 3.2951651,11.304 L3.2951651,34.084 C3.2951651,37.166 5.3721651,39.846 8.3201651,40.583 C11.2681651,41.387 14.3501651,40.114 15.8911651,37.434 C16.2261651,36.831 17.0301651,36.63 17.6331651,36.965 C18.3031651,37.367 18.5041651,38.104 18.1021651,38.774 C16.0251651,42.392 11.7371651,44.201 7.6501651,43.129 Z M27.5400285,7.15 C26.5350285,7.15 25.6640285,6.279 25.6640285,5.274 C25.6640285,4.269 26.5350285,3.398 27.5400285,3.398 C28.6120285,3.398 29.4160285,4.269 29.4160285,5.274 C29.4160285,6.279 28.6120285,7.15 27.5400285,7.15 Z M28.7460285,42.124 C28.7460285,42.794 28.2100285,43.33 27.5400285,43.33 C26.8700285,43.33 26.3340285,42.794 26.3340285,42.124 L26.3340285,15.324 C26.3340285,14.654 26.8700285,14.118 27.5400285,14.118 C28.2100285,14.118 28.7460285,14.654 28.7460285,15.324 L28.7460285,42.124 Z M36.9108919,16.798 C36.2408919,16.798 35.7048919,16.262 35.7048919,15.592 C35.7048919,14.922 36.2408919,14.386 36.9108919,14.386 L41.0648919,14.386 L41.0648919,1.924 C41.0648919,1.254 41.6008919,0.718 42.2708919,0.718 C42.9408919,0.718 43.4768919,1.254 43.4768919,1.924 L43.4768919,14.386 L48.9708919,14.386 C49.6408919,14.386 50.1768919,14.922 50.1768919,15.592 C50.1768919,16.262 49.6408919,16.798 48.9708919,16.798 L43.4768919,16.798 L43.4768919,35.424 C43.4768919,38.439 45.9558919,40.918 48.9708919,40.918 C49.6408919,40.918 50.1768919,41.454 50.1768919,42.124 C50.1768919,42.794 49.6408919,43.33 48.9708919,43.33 C44.6158919,43.33 41.0648919,39.779 41.0648919,35.424 L41.0648919,16.798 L36.9108919,16.798 Z M55.2597553,56.73 C54.5897553,56.73 54.0537553,56.194 54.0537553,55.524 C54.0537553,54.854 54.5897553,54.318 55.2597553,54.318 C56.4657553,54.318 57.6047553,53.916 58.5427553,53.246 C59.4807553,52.509 60.1507553,51.571 60.5527553,50.432 L63.0317553,42.124 L55.0587553,15.659 C54.8577553,15.056 55.2597553,14.386 55.8627553,14.185 C56.5327553,13.984 57.2027553,14.319 57.4037553,14.989 L64.2377553,37.903 L71.1387553,14.989 C71.3397553,14.319 72.0097553,13.984 72.6797553,14.185 C73.2827553,14.386 73.6847553,15.056 73.4837553,15.659 L65.4437553,42.459 L62.8307553,51.102 C62.3617553,52.71 61.3567553,54.117 59.9497553,55.189 C58.6097553,56.194 57.0017553,56.73 55.2597553,56.73 Z M82.2516188,43.062 C81.5816188,43.062 81.0456188,42.526 81.0456188,41.856 L81.0456188,1.924 C81.0456188,1.254 81.5816188,0.718 82.2516188,0.718 C82.9216188,0.718 83.4576188,1.254 83.4576188,1.924 L83.4576188,14.386 L88.9516188,14.386 C93.3066188,14.386 96.8576188,17.937 96.8576188,22.292 L96.8576188,35.156 C96.8576188,39.511 93.3066188,43.062 88.9516188,43.062 L82.2516188,43.062 Z M88.9516188,16.798 L83.4576188,16.798 L83.4576188,40.65 L88.9516188,40.65 C91.9666188,40.65 94.4456188,38.171 94.4456188,35.156 L94.4456188,22.292 C94.4456188,19.277 91.9666188,16.798 88.9516188,16.798 Z M113.062482,43.33 C108.707482,43.33 105.156482,39.779 105.156482,35.424 L105.156482,22.024 C105.156482,17.669 108.707482,14.118 113.062482,14.118 C117.417482,14.118 120.968482,17.669 120.968482,22.024 L120.968482,35.424 C120.968482,39.779 117.417482,43.33 113.062482,43.33 Z M113.062482,16.53 C110.047482,16.53 107.568482,19.009 107.568482,22.024 L107.568482,35.424 C107.568482,38.439 110.047482,40.918 113.062482,40.918 C116.077482,40.918 118.556482,38.439 118.556482,35.424 L118.556482,22.024 C118.556482,19.009 116.077482,16.53 113.062482,16.53 Z M137.441346,43.062 C133.086346,43.062 129.535346,39.511 129.535346,35.156 L129.535346,15.324 C129.535346,14.654 130.071346,14.118 130.741346,14.118 C131.411346,14.118 131.947346,14.654 131.947346,15.324 L131.947346,35.156 C131.947346,38.171 134.426346,40.65 137.441346,40.65 L142.935346,40.65 L142.935346,15.324 C142.935346,14.654 143.471346,14.118 144.141346,14.118 C144.811346,14.118 145.347346,14.654 145.347346,15.324 L145.347346,41.856 C145.347346,42.526 144.811346,43.062 144.141346,43.062 L137.441346,43.062 Z M156.460209,43.33 C155.790209,43.33 155.254209,42.794 155.254209,42.124 L155.254209,15.592 C155.254209,14.922 155.790209,14.386 156.460209,14.386 L163.160209,14.386 C167.515209,14.386 171.066209,17.937 171.066209,22.292 L171.066209,42.124 C171.066209,42.794 170.530209,43.33 169.860209,43.33 C169.190209,43.33 168.654209,42.794 168.654209,42.124 L168.654209,22.292 C168.654209,19.277 166.175209,16.798 163.160209,16.798 L157.666209,16.798 L157.666209,42.124 C157.666209,42.794 157.130209,43.33 156.460209,43.33 Z M187.539072,43.062 C183.184072,43.062 179.633072,39.511 179.633072,35.156 L179.633072,22.292 C179.633072,17.937 183.184072,14.386 187.539072,14.386 L193.033072,14.386 L193.033072,1.924 C193.033072,1.254 193.569072,0.718 194.239072,0.718 C194.909072,0.718 195.445072,1.254 195.445072,1.924 L195.445072,41.856 C195.445072,42.526 194.909072,43.062 194.239072,43.062 L187.539072,43.062 Z M187.539072,16.798 C184.524072,16.798 182.045072,19.277 182.045072,22.292 L182.045072,35.156 C182.045072,38.171 184.524072,40.65 187.539072,40.65 L193.033072,40.65 L193.033072,16.798 L187.539072,16.798 Z"
                id="Citybound" fill="#000000"></path>
        </g>
    </svg>;

    let windows = state.menu.visible && EL("div", { key: "debug", className: "window menu" }, [
        EL("a", { className: "close-window", onClick: () => setState(oldState => update(oldState, { menu: { visible: { $set: false } } })) }, "×"),
        EL(Tabs, { type: "card", size: "large", activeKey: state.menu.tabKey, onChange: newTabKey => setState(oldState => update(oldState, { menu: { tabKey: { $set: newTabKey } } })) }, [
            EL(TabPane, { tab: "About", key: "about" }, [
                svg,
                EL("a", { className: "become-patron", href: "https://patreon.com/citybound", target: "_blank" }, " "),
                EL("h2", {}, window.cbversion),
                EL("p", {}, "THIS IS A LIVE BUILD OF CITYBOUND AND THUS NOT A STABLE RELEASE."),
                EL("p", { style: { width: "30em" } }, "Expect nothing to work and a lot to be missing. See the issues below (from Github) to get an overview of the most glaring known problems and remaining tasks for the currently upcoming release."),
                EL("p", {}, EL(UpdateChecker)),
                EL("h3", {}, "Upcoming Release:"),
                EL(GithubMilestone, {})
            ]),
            EL(TabPane, { tab: "Credits", key: "credits" }, [
                svg,
                EL("a", { className: "become-patron", href: "https://patreon.com/citybound", target: "_blank" }, " "),
                EL("p", {}, ["is being developed by:"]),
                EL("p", {}, [EL("img", { src: aePlayLogo, width: 60 }), " aka. Anselm Eickhoff"]),
                EL("h4", {}, "With the generous support of these Patrons:"),
                EL("p", {},
                    EL(PatronCredits, {})
                ),
                EL("h4", {}, "Icons by icons8.com"),
                EL("h4", {}, "Cities I developed Citybound in:"),
                EL("ul", {}, [
                    EL("li", {}, "Munich"),
                    EL("li", {}, "Saint Petersburg"),
                    EL("li", {}, "Reykjavík"),
                    EL("li", {}, "Bangkok"),
                    EL("li", {}, "Singapore"),
                    EL("li", {}, "Denpasar"),
                    EL("li", {}, "Kuala Lumpur"),
                ])
            ]),
            EL(TabPane, { tab: "Tutorial", key: "tutorial" }, [
                EL("p", {}, "Please note that this tutorial is super bare-bones, but it should get you going."),
                EL("p", {}, "(You can open and close this whole window while following the tutorial by clicking the menu icon)"),
                EL("h3", {}, "Click the pencil icon to go into planning mode."),
                EL("p", {}, "Click the empty dropdown and choose the only existing proposal."),
                EL("h2", {}, "Planning Roads"),
                EL("p", {}, "Go to road planning mode by clicking the road icon."),
                EL("p", {}, "Start a new road by clicking on the map. Double click finishes a stroke. You can move control points of existing points around, but nothing more yet (no undo, delete, extend road yet)."),
                EL("h2", {}, "Planning Zones"),
                EL("p", {}, "Go to zone planning mode by clicking the zone icon next to the road icon. Draw zone shapes by selecting a zone type, then clicking on the map to define its corners. Double clicking finishes a shape. (A zone has to touch a road to become useable)"),
                EL("p", {}, "Changes to zones need to be implemented to become effective."),
                EL("h2", {}, "Implementing Proposals"),
                EL("p", {}, "Press implement to implement your proposal plan."),
                EL("h2", {}, "Further Steps"),
                EL("p", {}, "Speed up time using the slider next to the clock in the top left corner and see what happens."),
                EL("p", {}, "You can also start a new proposal by choosing the only existing proposal in the dropdown again."),
                EL("p", {}, "Roads that lead further away automatically get a neighboring town connection (white diamond). These move as you expand your town."),
            ]),
            EL(TabPane, { tab: "Settings & Controls", key: "settings" }, [
                EL(Settings, { currentSettings: state.settings, specs: settingsSpecs, setState }),
            ])
        ])
    ]);
    return { tools, windows };
}

class UpdateChecker extends React.Component {
    constructor(props) {
        super(props);
        this.state = { versions: [] };
    }

    componentDidMount() {
        fetch("http://citybound.livebuilds.s3-eu-west-1.amazonaws.com/?delimiter=/").then(response =>
            response.text().then(text =>
                this.setState({ versions: text.match(/citybound-v\d.\d.\d-\d+-\w+-osx/g).map(str => str.replace("citybound-", "").replace("-osx", "")) })
            )).catch(() => this.setState({ failed: true }));;
    }

    render() {
        if (this.state.failed) {
            return "Couldn't check for newest live builds."
        } else {
            let allVersions = this.state.versions.concat([window.cbversion]);
            allVersions.sort();
            if (allVersions[allVersions.length - 1] == window.cbversion) {
                return "You have the newest live build."
            } else {
                return EL("h3", {}, ["Newer live build available: ", EL("a", { href: "http://aeplay.co/citybound-livebuilds" }, allVersions[allVersions.length - 1])])
            }
        }
    }
}

class PatronCredits extends React.Component {
    constructor(props) {
        super(props);

        this.state = { patrons: [] };
    }

    componentDidMount() {
        fetch("https://cb-patrons-app.now.sh/v1/members").then(response =>
            response.json().then(patrons => {
                patrons.sort((a, b) => b.attributes.lifetime_support_cents - a.attributes.lifetime_support_cents);
                this.setState({ patrons: patrons })
            })).catch(() => this.setState({ failed: true }));
    }

    render() {
        return this.state.patrons.map(patron => {
            const { currently_entitled_amount_cents, full_name,
                lifetime_support_cents, patron_status, pledge_relationship_start } = patron.attributes;
            return !!lifetime_support_cents && EL("span", {
                className: "patron",
                style: {
                    fontSize: 2 * Math.log2(lifetime_support_cents),
                }
            }, full_name);
        }) || (this.state.failed ? "Couldn't load patrons." : "Loading patrons...")
    }
}

class GithubMilestone extends React.Component {
    constructor(props) {
        super(props);

        this.state = {};
    }

    componentDidMount() {
        fetch("https://cb-github-app.now.sh/v1/current_milestone").then(response =>
            response.json().then(json => json.data && this.setState(json.data.repository.milestone))
        ).catch(() => this.setState({ failed: true }));
    }

    render() {
        const renderIssueEdge = open => edge => {
            let checkable = (edge.node.bodyHTML.match(/task-list-item-checkbox/g) || []).length;
            let checked = (edge.node.bodyHTML.match(/checked/g) || []).length;
            return EL(Panel, {
                header: EL("div", { style: { pointerEvents: "none" } },
                    EL(Checkbox, { checked: !open },
                        edge.node.title + (checkable ? " (" + checked + "/" + checkable + ")" : "")
                    )
                ), key: edge.node.id
            }, EL("div", { dangerouslySetInnerHTML: { __html: edge.node.bodyHTML } }))
        };

        return [
            EL("h2", {}, this.state.title || (this.state.failed ? "Couldn't load milestone." : "loading milestone...")),
            EL(Progress, { percent: this.state.open ? Math.floor(this.state.closed.edges.length / (this.state.closed.edges.length + this.state.open.edges.length) * 100) : 0 }),
            EL("h3", {}, "TODO:"),
            this.state.open && EL(Collapse, {}, this.state.open.edges.map(renderIssueEdge(true))),
            EL("h3", {}, "DONE:"),
            this.state.closed && EL(Collapse, {}, this.state.closed.edges.map(renderIssueEdge(false)))
        ]
    }
}