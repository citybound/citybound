function toLinFloat(rgb) {
    return [
        Math.pow(rgb[0] / 256, 2.2),
        Math.pow(rgb[1] / 256, 2.2),
        Math.pow(rgb[2] / 256, 2.2),
    ]
}

export function fromLinFloat(rgb) {
    return [
        Math.pow(rgb[0], 1 / 2.2) * 256,
        Math.pow(rgb[1], 1 / 2.2) * 256,
        Math.pow(rgb[2], 1 / 2.2) * 256,
    ]
}

export function toCSS(rgb) {
    return `rgb(${Math.round(rgb[0])}, ${Math.round(rgb[1])}, ${Math.round(rgb[2])})`
}

function mix(a, b, alpha) {
    return [
        a[0] * alpha + b[0] * (1 - alpha),
        a[1] * alpha + b[1] * (1 - alpha),
        a[2] * alpha + b[2] * (1 - alpha),
    ]
}

const grass = [0.79, 0.88, 0.67];

export default {
    grass,
    asphalt: [0.7, 0.7, 0.7],
    roadMarker: [1.0, 1.0, 1.0],
    wall: [0.95, 0.95, 0.95],
    flatRoof: [0.5, 0.5, 0.5],
    brickRoof: [0.8, 0.5, 0.2],
    field: [0.7, 0.7, 0.2],

    plannedAsphalt: [1.0, 1.0, 1.0],
    plannedRoadMarker: [0.6, 0.6, 0.6],
    destructedAsphalt: [1.0, 0.0, 0.0],
    buildingOutlines: [0.0, 0.0, 0.0],

    controlPointMaster: [0.3, 0.3, 1.0],
    controlPointCurrentProposal: [0.0, 0.061, 1.0],//[0, 72, 255]
    controlPointHover: [1.0, 1.0, 1.0],

    Residential: mix(toLinFloat([234, 203, 82]), grass, 0.9),
    Commercial: mix(toLinFloat([213, 94, 0]), grass, 0.9),
    Offices: mix(toLinFloat([0, 0, 0]), grass, 0.9),
    Industrial: mix(toLinFloat([119, 66, 95]), grass, 0.9),
    Agricultural: mix(toLinFloat([136, 136, 108]), grass, 0.9),
    Recreational: mix(toLinFloat([124, 192, 124]), grass, 0.9),
    Official: mix(toLinFloat([39, 150, 221]), grass, 0.9),
}