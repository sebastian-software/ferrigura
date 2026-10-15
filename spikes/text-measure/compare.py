"""Compare Rust measurements with Chromium's.

Usage: python3 compare.py requests.json rust.json browser.json
"""
import json
import statistics
import sys
from collections import defaultdict

requests = json.load(open(sys.argv[1], encoding='utf-8'))
rust = json.load(open(sys.argv[2], encoding='utf-8'))
browser = json.load(open(sys.argv[3], encoding='utf-8'))
assert len(requests) == len(rust) == len(browser)

by_style = defaultdict(list)
worst = []
canvas_vs_svg = []
for req, r, b in zip(requests, rust, browser):
    diff = r['width'] - b['svgWidth']
    by_style[(req['style'], req['size'])].append(abs(diff))
    worst.append((abs(diff), diff, req['style'], req['size'], req['text']))
    canvas_vs_svg.append(abs(b['canvasWidth'] - b['svgWidth']))

print('Width difference Rust vs Chromium SVG getComputedTextLength (px)')
print(f"{'style':<11}{'size':>5}{'n':>6}{'max':>9}{'mean':>9}{'>0.01':>8}{'>0.5':>7}")
for (style, size), diffs in sorted(by_style.items()):
    print(
        f"{style:<11}{size:>5}{len(diffs):>6}{max(diffs):>9.4f}{statistics.mean(diffs):>9.4f}"
        f"{sum(d > 0.01 for d in diffs):>8}{sum(d > 0.5 for d in diffs):>7}"
    )
all_diffs = [w[0] for w in worst]
print(f"\nall: n={len(all_diffs)} max={max(all_diffs):.4f} mean={statistics.mean(all_diffs):.4f}")
print(f"canvas vs SVG in Chromium itself: max={max(canvas_vs_svg):.4f} mean={statistics.mean(canvas_vs_svg):.4f}")

print('\nLargest differences:')
for abs_diff, diff, style, size, text in sorted(worst, reverse=True)[:8]:
    print(f"  {diff:+8.4f}px  {style:<10} {size:>3}px  {text!r}")

height_diffs = []
top_diffs = []
for req, r, b in zip(requests, rust, browser):
    line = r['line']
    height_diffs.append(abs((line['ascender'] - line['descender']) - b['bboxHeight']))
    top_diffs.append(abs(-line['ascender'] - b['bboxTop']))
print(f"\nText box height (ascender - descender) vs SVG getBBox: max={max(height_diffs):.4f} mean={statistics.mean(height_diffs):.4f}")
print(f"Text box top (-ascender) vs SVG getBBox y:          max={max(top_diffs):.4f} mean={statistics.mean(top_diffs):.4f}")

wrap_total = wrap_same = 0
mismatches = []
for req, r, b in zip(requests, rust, browser):
    if 'wrapWidth' not in req:
        continue
    wrap_total += 1
    if r['lines'] == b['lines']:
        wrap_same += 1
    else:
        mismatches.append((req['wrapWidth'], req['text'], r['lines'], b['lines']))
print(f"\nLine breaking vs Chromium CSS: {wrap_same}/{wrap_total} identical")
m_total = m_same = 0
m_mismatches = []
for req, r, b in zip(requests, rust, browser):
    if 'wrapWidth' not in req:
        continue
    m_total += 1
    if r['mermaidLines'] == b['mermaidLines']:
        m_same += 1
    else:
        m_mismatches.append((req['wrapWidth'], req['text'], r['mermaidLines'], b['mermaidLines']))
print(f"Mermaid's SVG wrapping, Rust port vs JS in Chromium: {m_same}/{m_total} identical")
for width, text, rl, bl in m_mismatches[:6]:
    print(f"  width {width}: {text!r}\n    rust:   {rl}\n    chrome: {bl}")
for width, text, rl, bl in mismatches[:6]:
    print(f"  width {width}: {text!r}\n    rust:   {rl}\n    chrome: {bl}")
