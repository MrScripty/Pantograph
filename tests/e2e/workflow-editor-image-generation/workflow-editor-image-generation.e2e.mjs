describe('workflow editor image-generation desktop path', () => {
  it('opens the desktop workflow editor shell with submit and artifact navigation visible', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()).includes('Pantograph'),
      {
        timeout: 30000,
        timeoutMsg: 'Pantograph desktop window title did not appear',
      },
    );

    const graphNavigation = await $('button[aria-label="Graph"]');
    await graphNavigation.waitForDisplayed({ timeout: 30000 });
    await graphNavigation.click();

    const submitButton = await $('//button[normalize-space(.)="Submit"]');
    await submitButton.waitForDisplayed({ timeout: 30000 });

    const ioInspectorNavigation = await $('button[aria-label="I/O Inspector"]');
    await ioInspectorNavigation.waitForDisplayed({ timeout: 30000 });
  });
});
