import Prism from 'prismjs';

Prism.languages.futil = {
    'diff-addition': {
        pattern: /^\+.*$/m
    },
    'diff-deletion': {
        pattern: /^-.*$/m
    },
    'comment': Prism.languages.clike.comment,
    'string': {
        pattern: /(["])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1/,
        greedy: true
    },
    'namespace': {
        pattern: /\b(?:extern|component|primitive)\b/,
        lookbehind: true,
    },
    'function': {
        pattern: /\b(?:cells|wires|control|group|comb)\b/,
        lookbehind: true,
    },
    'keyword': {
        pattern: /\b(?:|seq|par|if|while|with|invoke)\b/,
        lookbehind: true,
    },
    'number': [
        {
            pattern: /\b[0-9]+'b[0-1]+\b/
        },
        {
            pattern: /\b[0-9]+'d[0-9]+\b/
        },
        {
            pattern: /\b[0-9]+'x[0-9A-Fa-f]+\b/
        },
        {
            pattern: /\b[0-9]+'o[0-7]+\b/
        },
        {
            pattern: /\b(?:[0-9]+)(?!')\b/
        }
    ],
};
