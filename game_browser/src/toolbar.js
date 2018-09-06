import React from 'react';
const EL = React.createElement;

export function makeToolbar(id, descriptions, prefix, uiMode, setMode, colorMap) {
    if (uiMode.startsWith(prefix)) {
        return [EL("div", { id, className: "toolbar" }, descriptions.map(description => {
            const descriptionSlug = description;
            return EL("button", {
                id: descriptionSlug,
                key: descriptionSlug,
                alt: description,
                className: uiMode.startsWith(prefix + "/" + descriptionSlug) ? "active" : "",
                style: colorMap ? { backgroundColor: colorMap(descriptionSlug) } : {},
                onClick: () => setMode(prefix + "/" + descriptionSlug),
            })
        }))]
    } else {
        return []
    }
}