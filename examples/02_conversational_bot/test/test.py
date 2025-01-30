import re
from playwright.sync_api import Page, expect
from playwright.sync_api import Playwright
import time

def test_has_title(playwright: Playwright):
    browser = playwright.chromium.launch(headless=False,
                                         args=[
                                             # bypasses Chrome's cam/mic permissions dialog
                                             "--use-fake-ui-for-media-stream",
                                             "--ignore-certificate-errors",
                                             "--use-fake-device-for-media-stream",
                                             "--use-file-for-fake-audio-capture=audio.wav"
                                             ])
    context = browser.new_context()
    context.grant_permissions(permissions=["microphone"])
    page = context.new_page()
    page.goto("https://127.0.0.1:3001/")
    expect(page).to_have_title(re.compile("TronApp"))
    page.get_by_role( 'button',  name='Start Conversation' ).click();
    time.sleep(15)
    page.get_by_role( 'button',  name='Stop Conversation' ).click();
    page.pause()
    context.close()
    browser.close()



