import Monet from 'monet';
import React from 'react';
import ReactDOM from 'react-dom';
import { vec3, mat4 } from 'gl-matrix';
import ContainerDimensions from 'react-container-dimensions';
import msgpack from 'msgpack-lite';

class CityboundClient extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            meshes: {},
            view: {
                eye: [-150, -150, 150],
                target: [0, 0, 0],
                verticalFov: 0.3 * Math.PI
            }
        }

        this.socket = new WebSocket("ws://localhost:9999");
        this.socket.binaryType = 'arraybuffer';

        this.socket.onopen = () => {
            this.socket.send(msgpack.encode(["INIT"]));

            setInterval(() => this.socket.send(msgpack.encode(["GET_ALL_PLANS"])), 1000)
        }

        this.socket.onmessage = (event) => {
            const [command, options] = msgpack.decode(new Uint8Array(event.data));

            console.log(command, options);

            if (command == "ADD_MESH") {
                this.setState((oldState) => Object.assign(oldState, {
                    meshes: Object.assign(oldState.meshes, {
                        [options.name]: {
                            vertices: options.vertices,
                            indices: options.indices,
                        }
                    })
                }));
            } else if (command == "UPDATE_ALL_PLANS") {
                this.setState((oldState) => Object.assign(oldState, {
                    planning: options
                }));
            }
        }
    }

    render() {
        const gesturePointInstances = [];

        if (this.state.planning) {
            for (let gesture of Object.values(this.state.planning.master)) {
                for (let i = 0 ; i < gesture.points.length; i += 2) {
                    gesturePointInstances.push.apply(gesturePointInstances, [
                        gesture.points[i], gesture.points[i + 1], 0,
                        1.0, 0.0,
                        1.0, 0.0, 0.0
                    ])
                }
            }
        }

        const layers = [
            {
                batches: [
                    {
                        mesh: this.state.meshes.GestureDot,
                        instances: new Float32Array(gesturePointInstances)
                    }
                ]
            }
        ];
        //const {viewMatrix, perspectiveMatrix} = this.state.view;
        const { eye, target, verticalFov } = this.state.view;

        return React.createElement("div", {
            style: { width: "100%", height: "100%" },
            onWheel: e => {
                const forward = vec3.sub(vec3.create(), target, eye);
                forward[2] = 0;
                vec3.normalize(forward, forward);
                const sideways = vec3.rotateZ(vec3.create(), forward, vec3.create(), Math.PI / 2.0);

                const delta = vec3.scaleAndAdd(vec3.create(), vec3.scale(vec3.create(), forward, -e.deltaY), sideways, -e.deltaX);

                this.setState(oldState => ({
                    view: Object.assign(oldState.view, {
                        eye: vec3.add(vec3.create(), oldState.view.eye, delta),
                        target: vec3.add(vec3.create(), oldState.view.target, delta),
                    })
                }));

                e.preventDefault();
                return false;
            }
        },
            React.createElement(ContainerDimensions, { style: { width: "100%", height: "100%" } }, ({ width, height }) => {
                const viewMatrix = mat4.lookAt(mat4.create(), eye, target, [0, 0, 1]);
                const perspectiveMatrix = mat4.perspective(mat4.create(), verticalFov, width / height, 50000, 0.1);
                return React.createElement(Monet, { width, height, layers, viewMatrix, perspectiveMatrix, clearColor: [0.6, 0.75, 0.4, 1.0] })
            })
        );
    }
}

ReactDOM.render(React.createElement(CityboundClient), document.getElementById('app'));