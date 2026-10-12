import assert from 'node:assert/strict';
import test from 'node:test';
import * as commonmark from 'commonmark';
import { renderResearchMarkdown } from './render-research-markdown.mjs';

const render = markdown => renderResearchMarkdown(new commonmark.Parser().parse(markdown));

test('headings retain formatting with numbered repeated headings and plain-text TOC labels', () => {
  const result = render('# Café & *tea* `code`\n\n## Café & *tea* `code`\n\n### Detail');
  assert.match(result.html, /<h1 id="café--tea-code">Café &amp; <em>tea<\/em> <code>code<\/code><\/h1>/);
  assert.match(result.html, /<h2 id="café--tea-code-1">/);
  assert.deepEqual(result.headings, [
    '<a href="#café--tea-code">Café &amp; tea code</a>',
    '<a href="#café--tea-code-1">Café &amp; tea code</a>',
  ]);
});

test('hostile and malformed heading markup cannot create executable HTML or TOC attributes', () => {
  for (const input of [
    '# &lt;script&gt;alert(1)&lt;/script&gt;',
    '# <<script>alert(1)<</script>',
    '# <img src=x onerror="alert(1)"> text',
    '# `"><img src=x onerror=alert(1)>`',
    '# [click](javascript:alert%281%29)',
    '# [click](data:text/html;base64,PHNjcmlwdD4=)',
  ]) {
    const { html, headings } = render(input);
    const output = html + headings.join('');
    assert.doesNotMatch(output, /<script|<img|href="(?:javascript|data):|id="[^"]*"\s+on\w+=/i, input);
    assert.doesNotMatch(headings.join(''), /<(?!\/?a(?:\s|>))/i, input);
  }
});

test('safe ordinary links are preserved in body but TOC contains no nested links', () => {
  const result = render('# [Read & learn](chapter.html#intro)');
  assert.match(result.html, /href="chapter.html#intro">Read &amp; learn<\/a>/);
  assert.deepEqual(result.headings, ['<a href="#read--learn">Read &amp; learn</a>']);
});
