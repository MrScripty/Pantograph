import assert from 'node:assert/strict';

describe('workflow editor image-generation desktop path', () => {
  const workflowId = process.env.PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID;

  function testIdSelector(testId) {
    return `[data-testid="${testId}"]`;
  }

  function workflowOptionSelector(id) {
    const escapedId = id.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    return `${testIdSelector('workflow-graph-selector-option')}[data-workflow-id="${escapedId}"]`;
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

    const imageArtifact = await $(
      `${testIdSelector('io-artifact-card')}[data-artifact-media-family="image"]`,
    );
    await imageArtifact.waitForDisplayed({ timeout: 300000 });

    const readButton = await imageArtifact.$(testIdSelector('io-artifact-read-button'));
    await readButton.waitForClickable({ timeout: 30000 });
    await readButton.click();

    const imagePreview = await imageArtifact.$(testIdSelector('io-artifact-image-preview'));
    await imagePreview.waitForDisplayed({ timeout: 120000 });

    const imageSelector = `${testIdSelector('io-artifact-card')}[data-artifact-media-family="image"] ${testIdSelector('io-artifact-image-preview')}`;
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
      await imageArtifact.$(testIdSelector('io-artifact-access-error')).isExisting(),
      false,
      'A decoded image must not retain a read or decode error',
    );
  });
});
