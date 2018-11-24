export function fmtId(id) {
    let parts = id.split(/[_\.@]/g);
    let type = parseInt(parts[0], 16);
    let fullType = window.cbTypeIdMapping[type];
    let typeSplit = fullType.split("::");
    let shortType = typeSplit[typeSplit.length - 1];
    let instance = parseInt(parts[1], 16);
    let version = parts[2];
    let machine = parts[3];

    return shortType + " #" + instance;// + "v" + version + "@" + machine;
}