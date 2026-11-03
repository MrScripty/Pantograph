/* global describe, it, $, browser, process, document, HTMLImageElement */
import assert from 'node:assert/strict';

describe('workflow editor image-generation desktop path', () => {
  const workflowId = process.env.PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID;

  function testIdSelector(testId) {
    return `[data-testid="${testId}"]`;
  }

  function attributeValue(value) {
    return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  }

  function workflowOptionSelector(id) {
    return `${testIdSelector('workflow-graph-selector-option')}[data-workflow-id="${attributeValue(id)}"]`;
  }

  function artifactRowSelector(artifactId) {
    return `${testIdSelector('io-artifact-row')}[data-artifact-id="${attributeValue(artifactId)}"]`;
  }

  function artifactCardSelector(artifactId) {
    return `${testIdSelector('io-artifact-card')}[data-artifact-id="${attributeValue(artifactId)}"]`;
  }

  // The I/O Inspector shows a node/port tree and ONE artifact card. Select
  // tree rows in order until the card shows an image artifact; the image only
  // appears once the run has produced it, so keep polling the tree.
  async function selectImageArtifactRow(timeout) {
    let imageArtifactId = null;
    await browser.waitUntil(
      async () => {
        const artifactIds = await browser.execute(
          (rowSelector) =>
            Array.from(document.querySelectorAll(rowSelector), (row) => row.getAttribute('data-artifact-id') ?? ''),
          testIdSelector('io-artifact-row'),
        );
        for (const artifactId of new Set(artifactIds.filter(Boolean))) {
          const row = await $(artifactRowSelector(artifactId));
          if (!(await row.isExisting())) {
            continue;
          }
          await row.scrollIntoView();
          await row.click();
          const artifactCard = await $(artifactCardSelector(artifactId));
          if (
            (await artifactCard.isExisting()) &&
            (await artifactCard.getAttribute('data-artifact-media-family')) === 'image'
          ) {
            imageArtifactId = artifactId;
            return true;
          }
        }
        return false;
      },
      {
        timeout,
        interval: 1000,
        timeoutMsg: 'No I/O Inspector tree row selected an image artifact card',
      },
    );
    return imageArtifactId;
  }

  it('submits a configured workflow and reads the retained image artifact through I/O Inspector', async () => {
    if (!workflowId) {
      throw new Error('PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID is required');
    }

    const graphNavigation = await $(testIdSelector('workbench-nav-graph'));
    await graphNavigation.waitForDisplayed({ timeout: 30000 });
    await graphNavigation.click();

    const graphPage = await $(testIdSelector('workflow-editor-graph-page'));
    await graphPage.waitForDisplayed({ timeout: 30000 });

    const graphSelector = await $(testIdSelector('workflow-graph-selector-toggle'));
    await graphSelector.waitForDisplayed({ timeout: 30000 });
    await graphSelector.click();

    const workflowOption = await $(workflowOptionSelector(workflowId));
    await workflowOption.waitForDisplayed({ timeout: 30000 });
    await workflowOption.click();

    const submitButton = await $(testIdSelector('workflow-submit-button'));
    await submitButton.waitForDisplayed({ timeout: 30000 });
    let lastSubmitDisabledReason = null;
    try {
      await browser.waitUntil(
        async () => {
          const disabledReason = await $(testIdSelector('workflow-submit-disabled-reason'));
          lastSubmitDisabledReason = (await disabledReason.isExisting())
            ? await disabledReason.getText()
            : null;
          const currentSubmitButton = await $(testIdSelector('workflow-submit-button'));
          return (
            (await currentSubmitButton.isExisting()) &&
            !(await currentSubmitButton.getAttribute('disabled'))
          );
        },
        {
          timeout: 120000,
          timeoutMsg: 'Workflow submit button did not become enabled',
        },
      );
    } catch (error) {
      throw new Error(
        `Workflow submit button did not become enabled: ${
          lastSubmitDisabledReason ?? 'no disabled reason was projected'
        }`,
        { cause: error },
      );
    }
    await submitButton.click();

    const ioInspectorPage = await $(testIdSelector('io-inspector-page'));
    await ioInspectorPage.waitForDisplayed({ timeout: 300000 });

    const imageArtifactId = await selectImageArtifactRow(300000);
    const imageCardSelector = `${artifactCardSelector(imageArtifactId)}[data-artifact-media-family="image"]`;
    const imageArtifact = await $(imageCardSelector);
    await imageArtifact.waitForDisplayed({ timeout: 30000 });
    assert.equal(
      await $(artifactRowSelector(imageArtifactId)).getAttribute('aria-current'),
      'true',
      'The selected tree row must be the artifact shown in the card',
    );

    const readButton = await $(`${imageCardSelector} ${testIdSelector('io-artifact-read-button')}`);
    await readButton.waitForClickable({ timeout: 30000 });
    await readButton.click();

    const imageSelector = `${imageCardSelector} ${testIdSelector('io-artifact-image-preview')}`;
    const imagePreview = await $(imageSelector);
    await imagePreview.waitForDisplayed({ timeout: 120000 });

    const decodedImage = await browser.executeAsync((selector, done) => {
      const image = document.querySelector(selector);
      if (!(image instanceof HTMLImageElement)) {
        done({ error: 'Rendered image preview is unavailable' });
        return;
      }
      Promise.resolve()
        .then(() => image.decode())
        .then(() =>
          done({
            complete: image.complete,
            naturalWidth: image.naturalWidth,
            naturalHeight: image.naturalHeight,
            readByteLength: Number(image.dataset.previewByteLength),
            bodyComplete: image.dataset.previewComplete === 'true',
          }),
        )
        .catch((error) => done({ error: String(error) }));
    }, imageSelector);
    assert.equal(decodedImage.error, undefined, decodedImage.error);
    assert.equal(decodedImage.complete, true);
    assert.equal(decodedImage.bodyComplete, true, 'Inspector must mark the retained image body complete');
    assert.ok(decodedImage.readByteLength > 0, 'Decoded image must have a nonempty retained body');
    assert.ok(decodedImage.naturalWidth > 0, 'Decoded image must have positive naturalWidth');
    assert.ok(decodedImage.naturalHeight > 0, 'Decoded image must have positive naturalHeight');
    assert.equal(
      await $(`${imageCardSelector} ${testIdSelector('io-artifact-access-error')}`).isExisting(),
      false,
      'A decoded image must not retain a read or decode error',
    );
  });
});
