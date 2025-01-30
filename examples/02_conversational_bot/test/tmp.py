from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto("https://127.0.0.1:3001/")
    print(page.title())
    browser.close()
