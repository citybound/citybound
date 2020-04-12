import * as React from 'react';
import { useState } from 'react';
import { Toolbar } from './toolbar';
import { Settings } from './settings';
import { Collapse, Checkbox, Tabs, Progress } from 'antd';
import aePlayLogo from '../assets/ae_play.png';
import { ToToolPortal, ToWindowPortal } from './citybound';

const Panel = Collapse.Panel;
const TabPane = Tabs.TabPane;

function CBLogo() {
    return <svg width="188" height="71" viewBox="0 0 188 71" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path
            d="M20 22.3759C19.3528 20.5898 18.2412 19.0527 16.8245 17.9355C15.2815 16.7187 13.3766 16 11.3153 16C6.1706 16 2 20.4772 2 26V44C2 49.5228 6.1706 54 11.3153 54C13.3766 54 15.2815 53.2813 16.8245 52.0645C17.8113 51.2863 18.65 50.3045 19.2869 49.1766"
            stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M40 18V47C40 51.5228 42.4772 54 47 54" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M71.2347 27.2284L59.7196 62.7865C58.5115 66.1059 55.7286 68.3968 52.5218 69.1228" stroke="black"
            stroke-width="3" stroke-linecap="round" />
        <path d="M52.5218 27.2284C52.5218 27.2284 58.658 46.1767 61.726 55.6509" stroke="black" stroke-width="3"
            stroke-linecap="round" />
        <path d="M28.3463 27.5V53.5" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path
            d="M29.5 18.5C29.5 19.0523 29.0523 19.5 28.5 19.5C27.9477 19.5 27.5 19.0523 27.5 18.5C27.5 17.9477 27.9477 17.5 28.5 17.5C29.0523 17.5 29.5 17.9477 29.5 18.5Z"
            fill="black" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M36.3206 27.5H46" stroke="black" stroke-width="3" stroke-linecap="round" />
        <rect x="78" y="27" width="15" height="27" rx="7.5" stroke="black" stroke-width="3" stroke-linecap="round" />
        <rect width="15" height="27" rx="7.5" transform="matrix(-1 0 0 1 186 27)" stroke="black" stroke-width="3"
            stroke-linecap="round" />
        <path d="M186 53V16" stroke="black" stroke-width="3" stroke-linecap="round" />
        <rect x="101" y="27" width="15" height="27" rx="7.5" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M139 27.5V45.5C139 50.1944 135.642 54 131.5 54C127.358 54 124 50.1944 124 45.5V27.5" stroke="black"
            stroke-width="3" stroke-linecap="round" />
        <path d="M163 53.5V36.5C163 31.8056 159.642 28 155.5 28C151.358 28 148 31.8056 148 36.5V53.5" stroke="black"
            stroke-width="3" stroke-linecap="round" />
        <path d="M78 54V16" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M148 38.5V28.5" stroke="black" stroke-width="3" stroke-linecap="round" />
        <path d="M139 53.5V43.5" stroke="black" stroke-width="3" stroke-linecap="round" />
    </svg>;
}

export default function MainMenu(props: { state, setState, settingSpecs }) {
    const [visible, setVisible] = useState<boolean>(!localStorage["cb-hide-menu"]);
    const [tabKey, setTabKey] = useState<'about' | 'credits' | 'tutorial' | 'settings'>('about');


    return <>
        <ToToolPortal>
            <Toolbar id="menu-toolbar"
                options={{ menu: { description: "Menu" } }}
                value={visible && "menu"}
                onChange={() => setVisible(!visible)}
            />
        </ToToolPortal>

        {visible && <ToWindowPortal>
            <div key="debug" className="window menu">
                <a className="close-window" onClick={() => setVisible(false)}>×</a>
                <Tabs type="card" size="large" activeKey={tabKey} onChange={newTabKey => setTabKey(newTabKey as 'about' | 'credits' | 'tutorial' | 'settings')}>
                    <TabPane tab="About" key="about">
                        <CBLogo />
                        <a className="become-patron" href="https://patreon.com/citybound" target="_blank"> </a>
                        <h2>{window.cbversion}</h2>
                        <p>THIS IS A LIVE BUILD OF CITYBOUND AND THUS NOT A STABLE RELEASE.</p>
                        <p style={{ width: "30em" }}>Expect nothing to work and a lot to be missing. See the issues below (from Github) to get an overview of the most glaring known problems and remaining tasks for the currently upcoming release.</p>
                        <p><UpdateChecker /></p>
                        <h3>Upcoming Release:</h3>
                        <GithubMilestone />
                    </TabPane>
                    <TabPane tab="Credits" key="credits">
                        <CBLogo />
                        <a className="become-patron" href="https://patreon.com/citybound" target="_blank"> </a>
                        <p>is being developed by:</p>
                        <p><img src={aePlayLogo} width={60} /> aka. Anselm Eickhoff</p>
                        <h4>With the generous support of these Patrons:</h4>
                        <p><PatronCredits /></p>
                        <h4>Icons by icons8.com</h4>
                        <h4>Cities I developed Citybound in:</h4>
                        <ul>
                            <li>Munich</li>
                            <li>Saint Petersburg</li>
                            <li>Reykjavík</li>
                            <li>Bangkok</li>
                            <li>Singapore</li>
                            <li>Denpasar</li>
                            <li>Kuala Lumpur</li>
                            <li>Boston</li>
                        </ul>
                    </TabPane>
                    <TabPane tab="Tutorial" key="tutorial">
                        <p>Please note that this tutorial is super bare-bones, but it should get you going.</p>
                        <p>(You can open and close this whole window while following the tutorial by clicking the menu icon)</p>
                        <p><em>1) Click the pencil icon to go into planning mode.</em></p>
                        <p><em>2) Click the "Start a new project" button.</em></p>
                        <h2>Planning Roads</h2>
                        <p><em>1) Go to road planning mode by clicking the road icon.</em></p>
                        <p><em>2) Start a new road by clicking on the map</em> and continue to click to add road nodes.</p>
                        <p><em>3) To finish a road, double-click when placing the last node.</em></p>
                        <p>You can move control points of existing points around, but nothing more yet (delete, extend road yet).</p>
                        <p>Roads that lead further away automatically get a neighboring town connection (double arrow). These move as you expand your town.</p>
                        <h2>Planning Zones</h2>
                        <p><em>1) Go to zone planning mode by clicking the zone icon</em> next to the road icon.</p>
                        <p><em>2) Draw zone shapes by selecting a zone type, then clicking on the map</em> to define its corners.</p>
                        <p><em>3) Double clicking finishes a shape.</em> (A zone has to touch a road to become useable)</p>
                        <p>Changes to zones need to be implemented to become effective.</p>
                        <h2>Implementing Projects</h2>
                        <p>Press the "Implement" button to implement your project plan.</p>
                        <h2>Further Steps</h2>
                        <p><em>Speed up time using the slider next to the clock in the top left corner</em> and see what happens.</p>
                        <p><em>Click on the eye icon and hover/click on buildings to inspect them</em></p>
                    </TabPane>
                    <TabPane tab="Settings &amp; Controls" key="settings">
                        <Settings currentSettings={props.state.settings} specs={props.settingSpecs} setState={props.setState} />
                    </TabPane>
                </Tabs>
            </div>
        </ToWindowPortal>}
    </>;
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
                return <h3>Newer live build available: <a href="http://aeplay.co/citybound-livebuilds">{allVersions[allVersions.length - 1]}</a></h3>;
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
            return !!lifetime_support_cents && <span className="patron"
                style={{ fontSize: 2 * Math.log2(lifetime_support_cents), }}>
                {full_name}
            </span>
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
            return <Panel key={edge.node.id}
                header={<div style={{ pointerEvents: "none" }}>
                    <Checkbox checked={!open} >{edge.node.title + (checkable ? " (" + checked + "/" + checkable + ")" : "")}</Checkbox>
                </div>}>
                <div dangerouslySetInnerHTML={{ __html: edge.node.bodyHTML }}></div>
            </Panel>;
        };

        return [
            <h2>{this.state.title || (this.state.failed ? "Couldn't load milestone." : "loading milestone...")}</h2>,
            <Progress percent={this.state.open ? Math.floor(this.state.closed.edges.length / (this.state.closed.edges.length + this.state.open.edges.length) * 100) : 0} />,
            <h3>TODO:</h3>,
            this.state.open && <Collapse>{this.state.open.edges.map(renderIssueEdge(true))}</Collapse>,
            <h3>DONE:</h3>,
            this.state.closed && <Collapse>{this.state.closed.edges.map(renderIssueEdge(false))}</Collapse>
        ]
    }
}