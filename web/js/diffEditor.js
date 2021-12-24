import * as Diff from 'diff';
import Prism from 'prismjs';
import './prism-futil.js';
import 'prismjs/plugins/keep-markup/prism-keep-markup';
import 'prismjs/plugins/line-numbers/prism-line-numbers';

function wordDiff(diff, next, srcDiv, destDiv) {
    // if diffs cover different number of lines,
    // add empty lines to compensate
    let wordDiff = Diff.diffWordsWithSpace(diff.value, next.value);
    for (let change of wordDiff) {
        if (change.added == null && change.removed == null) {
            srcDiv.appendChild(document.createTextNode(change.value));
            destDiv.appendChild(document.createTextNode(change.value));
        } else if (change.added) {
            let span = document.createElement('span');
            span.innerHTML = change.value;
            span.classList = 'token diff-addition';
            destDiv.appendChild(span);
        } else if (change.removed) {
            let span = document.createElement('span');
            span.innerHTML = change.value;
            span.classList = 'token diff-deletion';
            srcDiv.appendChild(span);
        }
    }

    let count = diff.count - next.count;
    if (count > 0) {
        // more lines in diff than next
        let span = document.createElement('span');
        span.innerHTML = "\n".repeat(count);
        span.classList = 'line diff-empty';
        destDiv.appendChild(span);
    } else if (count < 0) {
        // more lines in diff than next
        let span = document.createElement('span');
        span.innerHTML = "\n".repeat(-count);
        span.classList = 'line diff-empty';
        srcDiv.appendChild(span);
    }
}

function lineDiff(diff, srcDiv, destDiv) {
    // reset src and dest
    srcDiv.innerHTML = "";
    destDiv.innerHTML = "";
    // compute line diff
    for (let i = 0; i < diff.length; i++) {
        let change = diff[i];
        if (change.added == null && change.removed == null) {
            // neither added nor removed. add to both
            srcDiv.appendChild(
                document.createTextNode(change.value)
            );
            destDiv.appendChild(
                document.createTextNode(change.value)
            );
        } else if (change.removed) {
            // check if next item is an addition
            if (i + 1 < diff.length && diff[i + 1].added) {
                wordDiff(change, diff[i + 1], srcDiv, destDiv);
                // skip next iteration because we've already handled it
                i++;
            } else {
                let span = document.createElement('span');
                span.innerHTML = change.value;
                span.classList = 'line diff-deletion';
                srcDiv.appendChild(span);
                if (change.count > 0) {
                    let span = document.createElement('span');
                    span.innerHTML = "\n".repeat(change.count);
                    span.classList = 'line diff-empty';
                    destDiv.appendChild(span);
                } else {
                    // editorStr.appendChild(document.createTextNode("\n".repeat(change.count)));
                }
            }
        } else if (change.added) {
            // present in dest, not source
            let span = document.createElement('span');
            span.innerHTML = change.value;
            span.classList.add('line');
            span.classList.add('diff-addition');
            destDiv.appendChild(span);
            if (change.count > 0) {
                let span = document.createElement('span');
                span.contentEditable = false;
                span.innerHTML = "\n".repeat(change.count);
                span.classList = 'line diff-empty';
                srcDiv.appendChild(span);
            } else {
                // let commentStr = "\n".repeat(Math.abs(change.count));
                // compiledStr.append(document.createTextNode(commentStr));
            }
        }
    }
}

export function updateDiffEditor(editor, sourceCode, compiledCode) {
    let srcDiv = editor.querySelector("#input");
    let destDiv = editor.querySelector("#output");

    destDiv.innerHTML = compiledCode;
    Prism.highlightElement(srcDiv);
    Prism.highlightElement(destDiv);

    /* if (compiledCode.startsWith("Error:")) {
        // there was compilation error. show that
        destDiv.innerHTML = compiledCode;
    } else {
        let diff = Diff.diffLines(sourceCode, compiledCode);
        lineDiff(diff, srcDiv, destDiv);

        // syntax highlighting
        Prism.highlightElement(srcDiv);
        Prism.highlightElement(destDiv);
    } */
}
