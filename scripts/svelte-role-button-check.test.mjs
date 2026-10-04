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
  assert.deepEqual(check('<div role="button" tabindex="0" onkeydown={activate}>{#snippet label()}Select{/snippet}{@render label()}</div>'), []);
});
