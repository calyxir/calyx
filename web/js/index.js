import * as calyx from 'calyx';
import config from "../data/config.json";
import passes from "../data/passes.json";
import calyx_info from "../calyx_hash.json";
import { updateDiffEditor } from './diffEditor.js';

import Prism from 'prismjs';
import './prism-futil.js';
import 'prismjs/plugins/keep-markup/prism-keep-markup';
import 'prismjs/plugins/line-numbers/prism-line-numbers';

var LIBRARIES = {};
var CURRENT_CODE = {};
var EDIT_MODE = false;

config.url_prefix = config.url_prefix + calyx_info.version;

// =========== Pass Selector =================
function buttonSet(pass, value) {
    pass.active = value;
    if (value) {
        pass.button.classList.replace("off", "on");
    } else {
        pass.button.classList.replace("on", "off");
    }
}

function createToggle(pass) {
    let button = document.createElement("button");
    button.classList.add("toggle");
    button.classList.add("off");
    button.innerHTML = pass.title;
    button.onclick = function () {
        buttonSet(pass, !pass.active);
    };
    return button;
}

const passDiv = document.getElementById("passes");
for (let pass of passes.passes) {
    let button = createToggle(pass);
    pass.button = button;
    passDiv.appendChild(button);
}

// ============= Compile ===============
function getActivePasses() {
    return passes.passes
        .filter(p => p.active)
        .map(p => p.name);
}

function selectPasses(item) {
    if ("passes" in item) {
        for (let p of passes.passes) {
            buttonSet(p, item.passes.includes(p.name));
        }
    }
}

document.getElementById("compile").onclick = function () {
    compile();
};

function compile() {
    EDIT_MODE = false;
    // get passes to run
    let passList = getActivePasses();
    // collect libraries into a single string
    let libraryCode = CURRENT_CODE.libraries.map(x => x.code).join("\n");
    // compile the code
    var compiledCode = calyx.run(
        passList,
        libraryCode,
        CURRENT_CODE.code
    );
    // update the diff editor
    const editor = document.getElementById("diffEditor");
    const srcDiv = editor.querySelector("#input");
    const destDiv = editor.querySelector("#output");

    destDiv.innerHTML = compiledCode;
    Prism.highlightElement(srcDiv);
    Prism.highlightElement(destDiv);
}


async function getLibrary(library, root) {
    if (library in LIBRARIES) {
        return await LIBRARIES[library];
    } else {
        let url = `${config.url_prefix}${root}${library}`;
        let request = await fetch(url);
        let code = await request.text();
        // if code has more imports, import those and append them to this lib
        if (/import/g.test(code)) {
            let prefix = library.split('/').slice(0, -1).join("/");
            let names = Array.from(code.matchAll(/import "(.*)";/g)).map(x => x[1]);
            let res = await fetchLibs(names, `${root}/${prefix}/`);
            let importRegex = /import "(.*)";/g;
            code = code.replaceAll(importRegex, "");
            for (let r of res) {
                code += r.code;
            }
        }
        LIBRARIES[library] = code;
        return code;
    }
}

async function fetchLibs(names, root) {
    let proms = names.map(async function (lib) {
        let code = await getLibrary(lib, root);
        return { name: lib, code: code };
    });
    return await Promise.all(proms);
}

const input = document.getElementById("input");
input.oninput = function () {
    CURRENT_CODE.code = input.innerText;
};


// ============ Examples ==============
// Add examples for the selector
const examples_select = document.getElementById("examples-select");
for (let item of config.examples) {
    let option = document.createElement('option');
    option.text = item.name;
    option.value = JSON.stringify(item);
    examples_select.add(option);
}

// Load example from the github repository
async function getExample(name, root) {
    let url = `${config.url_prefix}${root}${name}`;
    let response = await fetch(url);
    let code = await response.text();
    let importRegex = /import "(.*)";/g;
    let names = Array.from(code.matchAll(importRegex)).map(x => x[1]);
    let res = await fetchLibs(names, root);
    code = code.replaceAll(importRegex, "");
    return {
        code: code.trim(),
        libraries: res,
    };
}

// Define onchange method for example selector.
examples_select.onchange = function () {
    const input = document.getElementById("input");
    const output = document.getElementById("output");
    input.innerHTML = "loading...";
    output.innerHTML = "Compile `compile` to generate output.";
    let value = JSON.parse(examples_select.value);
    getExample(value.file, value.root)
        .then(t => CURRENT_CODE = t)
        .then(() => {
            input.innerHTML = CURRENT_CODE.code;
            const editor = document.getElementById("diffEditor");
            const srcDiv = editor.querySelector("#input");
            Prism.highlightElement(srcDiv);
        })
        .then(() => selectPasses(value));
};

// Call once to load example on page load.
examples_select.onchange()

// =============== Footer ===============
// Append Calyx version to the footer
const ver_div = document.getElementById("calyx-version");
const git_link = document.createElement('a');
git_link.appendChild(document.createTextNode(calyx_info.version.slice(0, 8)));
git_link.href = "https://github.com/calyxir/calyx/tree/" + calyx_info.version;
ver_div.appendChild(document.createTextNode("Built with Calyx version "));
ver_div.appendChild(git_link);

