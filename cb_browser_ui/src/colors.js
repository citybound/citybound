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

function shuffle(array) {
    let counter = array.length;

    // While there are elements in the array
    while (counter > 0) {
        // Pick a random index
        let index = Math.floor(Math.random() * counter);

        // Decrease counter by 1
        counter--;

        // And swap the last element with it
        let temp = array[counter];
        array[counter] = array[index];
        array[index] = temp;
    }

    return array;
}

const grass = [0.79, 0.88, 0.67];

export default {
    grass,
    trunks: [0.4, 0.3, 0.2],
    canopies: [0.3, 0.5, 0.2],
    asphalt: [0.6, 0.6, 0.6],
    roadMarker: [1.0, 1.0, 1.0],

    WhiteWall: [0.95, 0.95, 0.95],
    FlatRoof: [0.5, 0.5, 0.5],
    TiledRoof: [0.8, 0.5, 0.2],
    FieldWheat: [0.7, 0.7, 0.2],
    FieldMeadow: [0.49, 0.68, 0.37],
    FieldRows: [0.62, 0.56, 0.5],
    FieldPlant: [0.39, 0.58, 0.27],
    WoodenFence: [0.9, 0.8, 0.7],
    MetalFence: [0.8, 0.8, 0.8],
    LotAsphalt: [0.7, 0.7, 0.7],

    plannedAsphalt: [1.0, 1.0, 1.0],
    plannedRoadMarker: [0.6, 0.6, 0.6],
    destructedAsphalt: [1.0, 0.0, 0.0],
    buildingOutlines: [0.0, 0.0, 0.0],

    controlPointMaster: [0.3, 0.3, 1.0],
    controlPointCurrentProject: [0.0, 0.061, 1.0],//[0, 72, 255]
    controlPointHover: [0.3, 0.361, 1.0],

    Residential: mix(toLinFloat([234, 203, 82]), grass, 0.9),
    Commercial: mix(toLinFloat([213, 94, 0]), grass, 0.9),
    Offices: mix(toLinFloat([0, 0, 0]), grass, 0.9),
    Industrial: mix(toLinFloat([119, 66, 95]), grass, 0.9),
    Agricultural: mix(toLinFloat([136, 136, 108]), grass, 0.9),
    Recreational: mix(toLinFloat([124, 192, 124]), grass, 0.9),
    Administrative: mix(toLinFloat([39, 150, 221]), grass, 0.9),

    carColors: shuffle([
        [30.0, 45.0, 45.0],    // black
        [45.0, 30.0, 45.0],    // black
        [45.0, 45.0, 30.0],    // black
        [45.0, 45.0, 45.0],    // black
        [50.0, 45.0, 45.0],    // black
        [45.0, 50.0, 45.0],    // black
        [45.0, 45.0, 50.0],    // black
        [50.0, 50.0, 45.0],    // black
        [45.0, 30.0, 30.0],    // black
        [250.0, 250.0, 250.0], // white
        [250.0, 240.0, 250.0], // white
        [250.0, 250.0, 250.0], // white
        [100.0, 122.0, 122.0], // dark silver
        [122.0, 100.0, 122.0], // dark silver
        [122.0, 122.0, 100.0], // dark silver
        [122.0, 122.0, 122.0], // dark silver
        [130.0, 122.0, 122.0], // dark silver
        [122.0, 130.0, 122.0], // dark silver
        [122.0, 122.0, 130.0], // dark silver
        [130.0, 130.0, 122.0], // dark silver
        [122.0, 122.0, 100.0], // dark silver
        [160.0, 179.0, 179.0], // bright silver
        [179.0, 160.0, 179.0], // bright silver
        [179.0, 179.0, 160.0], // bright silver
        [179.0, 179.0, 179.0], // bright silver
        [190.0, 179.0, 179.0], // bright silver
        [179.0, 190.0, 179.0], // bright silver
        [179.0, 179.0, 190.0], // bright silver
        [190.0, 190.0, 179.0], // bright silver
        [179.0, 190.0, 190.0], // bright silver
        [160.0, 160.0, 179.0], // bright silver
        [179.0, 160.0, 160.0], // bright silver
        [59.0, 116.0, 183.0],  // dark blue
        [59.0, 116.0, 183.0],  // dark blue
        [59.0, 116.0, 183.0],  // dark blue
        [59.0, 116.0, 183.0],  // dark blue
        [59.0, 116.0, 183.0],  // dark blue
        [121.0, 177.0, 230.0], // bright blue
        [121.0, 177.0, 230.0], // bright blue
        [121.0, 177.0, 230.0], // bright blue
        [154.0, 205.0, 215.0], // cold turquoise
        [154.0, 205.0, 215.0], // cold turquoise
        [154.0, 205.0, 215.0], // cold turquoise
        [101.0, 164.0, 122.0], // dark green
        [183.0, 171.0, 139.0], // sand
        [183.0, 171.0, 139.0], // sand
        [183.0, 171.0, 139.0], // sand
        [146.0, 122.0, 92.0],  // brown
        [146.0, 122.0, 92.0],  // brown
        [146.0, 122.0, 92.0],  // brown
        [198.0, 130.0, 103.0], // brick red
        [198.0, 130.0, 103.0], // brick red
        [198.0, 130.0, 103.0], // brick red
        [223.0, 150.0, 137.0], // tomato red
        [198.0, 138.0, 160.0], // wine purple
        [146.0, 99.0, 130.0],  // eggplant purple
        [88.0, 78.0, 154.0],   // blue grey purple
        [79.0, 100.0, 154.0],  // dark grey blue
        [79.0, 100.0, 154.0],  // dark grey blue
        [170.0, 159.0, 159.0], // red medium silver
        [170.0, 159.0, 159.0], // red medium silver
        [170.0, 159.0, 159.0], // red medium silver
        [170.0, 159.0, 159.0], // red medium silver
        [233.0, 233.0, 249.0], // blueish off-white
        [233.0, 233.0, 249.0], // blueish off-white
        [98.0, 127.0, 95.0],   // forest green
        [219.0, 182.0, 108.0], // soft orange
        [146.0, 52.0, 64.0],   // dark red
    ].map(toLinFloat))
}