const path = INPUT_PATH || "";

function editConfig() {
    fetch("/", {
        method: "put",
        body: JSON.stringify({
            EditConfig: {
                path
            }
        }),
    });
}

function editCaption() {
    fetch("/", {
        method: "put",
        body: JSON.stringify({
            EditCaption: {
                path
            }
        }),
    });
}

addEventListener("DOMContentLoaded", () => {
    document
        .getElementById("edit_config")
        .addEventListener("click", editConfig);
    document
        .getElementById("edit_caption")
        .addEventListener("click", editCaption);
});