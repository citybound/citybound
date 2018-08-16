function toLinFloat(rgb) {
    return [
        Math.pow(rgb[0]/256, 2.2),
        Math.pow(rgb[1]/256, 2.2),
        Math.pow(rgb[2]/256, 2.2),
    ]
}

export default {
    grass: [0.79, 0.88, 0.67],
    asphalt: [0.7, 0.7, 0.7],
    roadMarker: [1.0, 1.0, 1.0],
    wall: [0.95, 0.95, 0.95],
    flatRoof: [0.5, 0.5, 0.5],
    brickRoof: [0.8, 0.5, 0.2],
    field: [0.7, 0.7, 0.2],

    plannedAsphalt: [1.0, 1.0, 1.0],
    plannedRoadMarker: [0.6, 0.6, 0.6],
    destructedAsphalt: [1.0, 0.0, 0.0],

    controlPointMaster: [0.3, 0.3, 1.0],
    controlPointCurrentProposal: [0.0, 0.061, 1.0],//[0, 72, 255]
    controlPointHover: [1.0, 1.0, 1.0],

    residential: toLinFloat([234, 203, 82]),
    commercial: toLinFloat([213, 94, 0]),
    offices: toLinFloat([0, 0, 0]),
    industrial: toLinFloat([119, 66, 95]),
    agricultural: toLinFloat([136, 136, 108]),
    recreational: toLinFloat([124, 192, 124]),
    official: toLinFloat([39, 150, 221]),
}