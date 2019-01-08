export const smallWindow = {
    vertices: new Float32Array([
        -0.7, 0.1, 1.2,
        0.7, 0.1, 1.2,
        0.7, 0.1, 2.5,
        -0.7, 0.1, 2.5,
    ]),
    indices: new Uint16Array([
        0, 1, 2,
        0, 2, 3
    ])
}

export const narrowDoor = {
    vertices: new Float32Array([
        -0.6, 0.1, 0.0,
        0.6, 0.1, 0.0,
        0.6, 0.1, 2.4,
        -0.6, 0.1, 2.4,
    ]),
    indices: new Uint16Array([
        0, 1, 2,
        0, 2, 3
    ])
}

export const wideDoor = {
    vertices: new Float32Array([
        -1.0, 0.1, 0.0,
        1.0, 0.1, 0.0,
        1.0, 0.1, 2.4,
        -1.0, 0.1, 2.4,
    ]),
    indices: new Uint16Array([
        0, 1, 2,
        0, 2, 3
    ])
}

export const shopWindowBanner = {
    vertices: new Float32Array([
        -1.5, 0.1, 0.5,
        1.5, 0.1, 0.5,
        1.5, 0.1, 1.5,
        -1.5, 0.1, 1.5,
    ]),
    indices: new Uint16Array([
        0, 1, 2,
        0, 2, 3
    ])
}

export const shopWindowGlass = {
    vertices: new Float32Array([
        -1.5, 0.1, 1.5,
        1.5, 0.1, 1.5,
        1.5, 0.1, 2.6,
        -1.5, 0.1, 2.6,
    ]),
    indices: new Uint16Array([
        0, 1, 2,
        0, 2, 3
    ])
}