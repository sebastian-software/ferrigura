"""Build measurement requests from real labels in the e2e fixtures plus edge cases.

Usage: python3 corpus.py <repo-root> > requests.json
"""
import glob
import json
import os
import random
import re
import sys

root = sys.argv[1]
labels = set()
patterns = [
    r'\["([^"\]]{1,80})"\]',  # A["label"]
    r'\[([^\[\]"()]{1,80})\]',  # A[label]
    r'\(([^()"\[\]]{1,80})\)',  # A(label)
    r'\{([^{}"]{1,80})\}',  # A{label}
    r'\|([^|]{1,60})\|',  # -->|edge label|
    r':\s*([A-Za-z][^:\n]{2,60})$',  # sequence/class messages
]
for path in glob.glob(os.path.join(root, 'e2e/diagrams/**/*.mmd'), recursive=True):
    text = open(path, encoding='utf-8').read()
    body = re.sub(r'^---\n.*?\n---\n', '', text, flags=re.S)
    for line in body.splitlines():
        for pattern in patterns:
            for match in re.findall(pattern, line.strip()):
                label = match.strip()
                if label and '<' not in label and '&' not in label and len(label) <= 80:
                    labels.add(label)

edge_cases = [
    'office fluffy fjord affinity',  # ligatures fi ff ffi fl ffl
    'AVATAR Type WAVE Yo Te LT',  # kerning pairs
    'Größenänderung Übermaß Ärger öffnen',  # umlauts
    '0123456789 10.5% $1,000.00',  # figures
    'Supercalifragilisticexpialidocious',  # one long word
    'a',
    ' leading and trailing spaces ',
    'Mixed: punctuation, (brackets) [and] {braces}!?',
    'CamelCaseIdentifierWithoutSpaces',
    'Données français: «guillemets» — tirets – et …',
]

random.seed(7)
sample = sorted(labels)
random.shuffle(sample)
sample = sample[:400] + edge_cases

requests = []
for text in sample:
    for size in (14, 16, 24):
        requests.append({'text': text, 'style': 'regular', 'size': size})
for text in sample[:120] + edge_cases:
    requests.append({'text': text, 'style': 'bold', 'size': 16})
    requests.append({'text': text, 'style': 'italic', 'size': 16})
    requests.append({'text': text, 'style': 'bolditalic', 'size': 16})
for text in sample[:60] + edge_cases:
    requests.append({'text': text, 'style': 'mono', 'size': 14})
long_texts = [t for t in sample if len(t) > 25][:80] + edge_cases
for text in long_texts:
    for width in (80, 140, 200):
        requests.append({'text': text, 'style': 'regular', 'size': 16, 'wrapWidth': width})

print(json.dumps(requests, ensure_ascii=False))
print(f'{len(labels)} labels found, {len(requests)} requests', file=sys.stderr)
