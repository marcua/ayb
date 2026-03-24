#!/usr/bin/env node
// Post-process ayb.d.ts to remove duplicate JSDoc blocks that tsc
// copies verbatim from ayb.js (the export type declarations already
// carry this information).
const fs = require('fs');
const file = 'ayb.d.ts';
let s = fs.readFileSync(file, 'utf8');

// Match /** ... */ blocks: non-greedy within a single comment.
// A block is a "jsdoc comment" = starts with /**, ends with first */.
const COMMENT_BLOCK = /\/\*\*(?:[^*]|\*(?!\/))*\*\/\n?/g;

s = s.replace(COMMENT_BLOCK, (match) => {
    // Drop blocks containing @typedef (duplicate of the export type above)
    if (match.includes('@typedef')) return '';
    // Drop the file-level docstring (usage examples, not useful in .d.ts)
    if (match.includes('ayb.js - Client library')) return '';
    return match;
});

fs.writeFileSync(file, s);
