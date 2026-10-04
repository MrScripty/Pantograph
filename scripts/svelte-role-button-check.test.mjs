import assert from 'node:assert/strict';
import test from 'node:test';
import { collectRoleButtonViolations } from './svelte-role-button-check.mjs';

const check = (source) => collectRoleButtonViolations(source, 'Fixture.svelte');
const rules = (source) => check(source).map((violation) => violation.rule);

test('arrow callbacks do not hide later keyboard and name attributes', () => {
  assert.deepEqual(check('<g role="button" tabindex="0" onclick={() => select()} onkeydown={(event) => activate(event)} aria-label="Select" />'), []);
});

test('quoted greater-than signs and legacy event directives retain complete attributes', () => {
  assert.deepEqual(check('<div role="button" title="a > b" tabindex="0" on:click={() => select()} on:keydown={activate} aria-labelledby="label" />'), []);
});

test('missing keyboard activation remains a violation after an arrow callback', () => {
  assert.deepEqual(rules('<g role="button" tabindex="0" onclick={() => select()} aria-label="Select" />'), ['role-button-keydown']);
});

test('all required attributes are checked independently with exact location', () => {
  const violations = check('\n<div role="button" />');
  assert.deepEqual(violations.map(({ rule }) => rule), ['role-button-tabindex', 'role-button-keydown', 'role-button-accessible-name']);
  assert.ok(violations.every(({ file, line }) => file === 'Fixture.svelte' && line === 2));
});

test('comments and script strings are not rendered elements', () => {
  assert.deepEqual(check('<script>const example = \'<div role="button" />\';</script><!-- <div role="button" /> -->'), []);
});

test('nested control-flow elements are inspected and native controls remain exempt', () => {
  assert.deepEqual(rules('{#if visible}<div role="button" tabindex="0" aria-label="Select" />{/if}<button role="button">Select</button>'), ['role-button-keydown']);
});

test('malformed Svelte fails closed instead of silently skipping checks', () => {
  assert.throws(() => check('<div role="button" onclick={() => } />'));
});


test('visible descendant text supplies a name, but handler expressions do not', () => {
  assert.deepEqual(check('<div role="button" tabindex="0" onkeydown={activate}><span>{label}</span></div>'), []);
  assert.deepEqual(rules('<div role="button" tabindex="0" onkeydown={() => activate()}><!-- Select --></div>'), ['role-button-accessible-name']);
});

test('an unused snippet declaration is not rendered name evidence', () => {
  assert.deepEqual(rules('<div role="button" tabindex="0" onkeydown={activate}>{#snippet label()}Select{/snippet}</div>'), ['role-button-accessible-name']);
});

// This static guard deliberately does not resolve snippet calls or their bindings.
const renderOnlyCases = [
  ['empty local snippet', '{#snippet label()}{/snippet}{@render label()}'],
  ['whitespace and comments', '{#snippet label()} \n<!-- Select -->{/snippet}{@render label()}'],
  ['nonempty local snippet', '{#snippet label()}Select{/snippet}{@render label()}'],
  ['nested unused snippet', '{#snippet label()}{#snippet unused()}Select{/snippet}{/snippet}{@render label()}'],
  ['nested render call', '{#snippet inner()}Select{/snippet}{#snippet label()}{@render inner()}{/snippet}{@render label()}'],
  ['shadowed local snippet', '{#snippet label()}Select{/snippet}{#if visible}{#snippet label()}{/snippet}{@render label()}{/if}'],
  ['snippet parameter', '{#snippet wrapper(label)}{@render label()}{/snippet}{@render wrapper(label)}'],
  ['self recursion', '{#snippet label()}{@render label()}{/snippet}{@render label()}'],
  ['mutual recursion', '{#snippet first()}{@render second()}{/snippet}{#snippet second()}{@render first()}{/snippet}{@render first()}'],
  ['unknown snippet', '{@render unknown()}'],
  ['optional snippet', '{@render unknown?.()}'],
  ['member call', '{@render snippets.label()}'],
];

for (const [name, content] of renderOnlyCases) {
  test(`render-only content requires independent name evidence: ${name}`, () => {
    const violations = check(`\n<div role="button" tabindex="0" onkeydown={activate}>${content}</div>`);
    assert.deepEqual(violations.map(({ rule }) => rule), ['role-button-accessible-name']);
    assert.equal(violations[0].file, 'Fixture.svelte');
    assert.equal(violations[0].line, 2);
  });
}

for (const attribute of ['aria-label="Select"', 'aria-labelledby="label"']) {
  test(`explicit ${attribute} supports render-only content`, () => {
    for (const [, content] of renderOnlyCases) {
      assert.deepEqual(check(`<div role="button" tabindex="0" onkeydown={activate} ${attribute}>${content}</div>`), []);
    }
  });
}

for (const name of ['Select', '<span>Select</span>', '{label}', '<span>{label}</span>']) {
  test(`adjacent rendered evidence remains accepted: ${name}`, () => {
    for (const [, content] of renderOnlyCases) {
      assert.deepEqual(check(`<div role="button" tabindex="0" onkeydown={activate}>${content}${name}</div>`), []);
    }
  });
}
