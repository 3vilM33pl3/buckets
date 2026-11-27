#!/usr/bin/env python3
"""End-to-end smoke test of the real app through tauri-driver (Linux).

Setup (once):
    cargo install tauri-driver --locked
    sudo apt install webkit2gtk-driver xvfb
    pip install selenium

Run against a fresh demo repository (scripts/make_demo_repo.py):
    tauri-driver &                      # listens on :4444
    BUCKETS_REPO=/tmp/demo/canalgame python3 tests/e2e.py target/debug/buckets-desktop /tmp/shots

Screenshots of every step are written to the shots folder.
"""
import os
import sys
import time
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.common.exceptions import StaleElementReferenceException

APP = os.path.abspath(sys.argv[1])
SHOTS = Path(sys.argv[2] if len(sys.argv) > 2 else "/tmp/shots")
SHOTS.mkdir(parents=True, exist_ok=True)

opts = ArgOptions()
opts.set_capability("browserName", "wry")
opts.set_capability("tauri:options", {"application": APP})
d = webdriver.Remote("http://127.0.0.1:4444", options=opts)
d.set_window_size(1600, 1000)
step = 0


def shot(name):
    global step
    step += 1
    time.sleep(0.6)
    d.save_screenshot(str(SHOTS / f"{step:02d}_{name}.png"))


def wait_for(css, timeout=20):
    end = time.time() + timeout
    while time.time() < end:
        try:
            found = d.find_elements(By.CSS_SELECTOR, css)
            if found and found[0].is_displayed():
                return found[0]
        except StaleElementReferenceException:
            pass
        time.sleep(0.2)
    raise AssertionError(f"timed out waiting for {css}")


def retry(fn, tries=5):
    """The UI re-renders after every refresh, so retry on stale elements."""
    for i in range(tries):
        try:
            return fn()
        except StaleElementReferenceException:
            if i == tries - 1:
                raise
            time.sleep(0.3)


def click(css):
    retry(lambda: wait_for(css).click())


def text(css):
    return retry(lambda: wait_for(css).get_attribute("innerText"))


def open_bucket(name):
    for el in d.find_elements(By.CSS_SELECTOR, '.sidebar .nav-item[data-act="open-bucket"]'):
        if el.get_attribute("textContent").strip().startswith(name):
            el.click()
            end = time.time() + 20
            while time.time() < end:
                ready = d.execute_script(
                    "return document.querySelector('.page-head h1')?.textContent === arguments[0]"
                    " && !document.querySelector('.main .loading')", name)
                if ready:
                    return
                time.sleep(0.2)
            raise AssertionError(f"bucket {name} did not load")
    raise AssertionError(f"no bucket {name}")


def errors():
    return d.execute_script("return window.__errors || []")


try:
    wait_for(".stage")
    assert "Pipeline" in text(".page-head h1")
    stages = [e.get_attribute("textContent").strip() for e in d.find_elements(By.CSS_SELECTOR, ".stage-name")]
    print("stages:", stages)
    assert stages.index("concept") < stages.index("modelling") < stages.index("engine")
    assert stages.index("texturing") < stages.index("modelling")
    shot("pipeline")

    # Texture diff: the brick base colour was repainted in the working copy.
    open_bucket("texturing")
    click('.list-item[data-path="brick/T_Brick_BaseColor.png"]')
    wait_for("#viewer .layer img")
    foot = text("#viewer .viewer-foot")
    print("diff footer:", foot.replace("\n", " "))
    assert "pixels changed" in foot
    shot("texture_side_by_side")
    click('#viewer [data-mode="swipe"]')
    wait_for("#viewer .swipe-handle")
    shot("texture_swipe")
    click('#viewer [data-mode="diff"]')
    shot("texture_difference")
    click('#viewer [data-ch="r"]')
    shot("texture_red_channel")
    click('#viewer [data-ch="rgb"]')
    click('#viewer [data-mode="side"]')

    # New file shows a single image.
    click('.list-item[data-path="brick/T_Brick_Moss_BaseColor.png"]')
    wait_for("#viewer .layer img")
    assert not d.find_elements(By.CSS_SELECTOR, '#viewer [data-mode="swipe"]')

    # Commit the texturing work.
    retry(lambda: wait_for("#commit-msg").send_keys("Moss variant and warmer base colour"))
    retry(lambda: wait_for("#commit-msg").send_keys(Keys.CONTROL, Keys.ENTER))
    wait_for(".toast.ok")
    print("toast:", text(".toast"))
    wait_for(".empty .big")
    assert "clean" in text(".empty .big").lower()
    shot("texturing_committed")

    # History of texturing: three commits, newest first, with a diff.
    click('.tab[data-tab="history"]')
    wait_for(".commit-item")
    commits = retry(lambda: [c.get_attribute("innerText") for c in d.find_elements(By.CSS_SELECTOR, ".commit-item")])
    print("history:", [c.split("\n")[0] for c in commits])
    assert len(commits) == 3 and "Moss variant" in commits[0]
    wait_for("#viewer .layer img")
    shot("texturing_history")

    # Text diff in concept.
    open_bucket("concept")
    click('.tab[data-tab="changes"]')
    click('.list-item[data-path="brief.md"]')
    wait_for("#viewer .textdiff .insert")
    assert "lanterns" in text("#viewer .textdiff .insert")
    shot("text_diff")

    # Locks: lock a file in the Files tab of engine, see it in the Locks view.
    open_bucket("engine")
    click('.tab[data-tab="files"]')
    click('.list-item[data-path="Config/DefaultGame.ini"]')
    click('#viewer [data-v="lock"]')
    wait_for('.list-item[data-path="Config/DefaultGame.ini"] .pill.lock')
    shot("engine_files_locked")
    click('.nav-item[data-view="locks"]')
    wait_for(".table")
    rows = retry(lambda: len(d.find_elements(By.CSS_SELECTOR, ".table tbody tr")))
    print("locks:", rows)
    assert rows == 4
    shot("locks")

    # Finalize one request in modelling, which marks it delivered for engine.
    click('.nav-item[data-view="pipeline"]')
    wait_for(".stage")
    def stage_named(name):
        for s in d.find_elements(By.CSS_SELECTOR, ".stage"):
            if s.find_element(By.CSS_SELECTOR, ".stage-name").get_attribute("textContent").strip() == name:
                return s
        raise AssertionError(name)

    retry(lambda: stage_named("modelling").find_element(By.CSS_SELECTOR, '[data-act="pick-pebble"]').click())
    retry(lambda: stage_named("modelling").find_element(By.CSS_SELECTOR, '[data-act="finalize"]').click())
    shot("finalize_confirm")
    click(".modal .btn.primary")
    wait_for(".toast.ok")
    time.sleep(1)
    assert retry(lambda: any("Delivered" in p.get_attribute("textContent") for p in d.find_elements(By.CSS_SELECTOR, ".pill.done")))
    shot("pipeline_after_finalize")

    # New expectation through the dialog.
    click('.page-head [data-act="new-expectation"]')
    wait_for('.modal textarea[name="desc"]').send_keys("Night lighting pass for the market")
    shot("new_expectation")
    click(".modal .btn.primary")
    wait_for(".toast.ok")
    time.sleep(1)
    assert "Night lighting pass" in d.find_element(By.CSS_SELECTOR, ".board").get_attribute("textContent")

    click('.nav-item[data-view="activity"]')
    wait_for(".timeline-item")
    shot("activity")

    # Light theme.
    click('[data-act="settings"]')
    click('.modal [data-theme="light"]')
    click(".modal .btn.primary")
    click('.nav-item[data-view="pipeline"]')
    wait_for(".stage")
    shot("pipeline_light")
    open_bucket("texturing")
    click('.tab[data-tab="history"]')
    wait_for("#viewer .layer img")
    shot("history_light")
    # Back to dark, and set a remote store, then push to it.
    click('[data-act="settings"]')
    click('.modal [data-theme="dark"]')
    remote = SHOTS / "remote-store"
    wait_for('.modal input[name="remote"]').send_keys(str(remote))
    shot("settings")
    click(".modal .btn.primary")
    wait_for('[data-act="push"]:not([disabled])')
    click('[data-act="push"]')
    end = time.time() + 20
    while time.time() < end and not any("Pushed" in t.get_attribute("textContent") for t in d.find_elements(By.CSS_SELECTOR, ".toast")):
        time.sleep(0.3)
    toasts = [t.get_attribute("textContent") for t in d.find_elements(By.CSS_SELECTOR, ".toast")]
    print("push:", toasts)
    assert any("Pushed" in t for t in toasts)
    assert (remote / "objects").exists() or any(remote.iterdir())

    errs = errors()
    print("script errors:", errs)
    assert not errs
    print("E2E OK")
except Exception:
    d.save_screenshot(str(SHOTS / "FAILED.png"))
    print("script errors:", errors())
    raise
finally:
    d.quit()
