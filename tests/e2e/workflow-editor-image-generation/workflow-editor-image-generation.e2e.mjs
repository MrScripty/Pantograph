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

    await browser.waitUntil(
      async () => (await browser.getTitle()).includes('Pantograph'),
      {
        timeout: 30000,
        timeoutMsg: 'Pantograph desktop window title did not appear',
      },
    );

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
    await browser.waitUntil(
      async () => !(await submitButton.getAttribute('disabled')),
      {
        timeout: 120000,
        timeoutMsg: 'Workflow submit button did not become enabled',
      },
    );
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
  });
});
