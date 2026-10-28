# 0017 Reproduce the reference, quirks included

**Scope:** which behavior ferrigura takes where the reference ([0014](./0014-upstream-reference-at-a-pin.md)) does something odd: a grammar quirk, a rounding, a wrong default, an error. It does not cover where a difference is recorded (that is 0014) or what ferrigura leaves out on purpose, such as HTML labels ([0009](./0009-svg-text-labels.md)).

## Decision

ferrigura reproduces what the reference does, also where it looks like a bug. The same text renders the same diagram, and fails where the reference fails.

- A quirk that changes the output is reproduced, and the probe that showed it becomes a test.
- A reference bug is fixed in ferrigura only together with a patch on the pinned commit that fixes it in the reference too, so that both sides agree again ([0014](./0014-upstream-reference-at-a-pin.md)).
- Where the reference crashes, or its result depends on the page it runs in (a random draw, the clock, the screen; [0018](./0018-deterministic-output.md)), ferrigura does not reproduce that. The comparison classes such an input as one the reference failed on, or leaves it out by a rule with its reason. Odd results that are deterministic, `NaN` geometry included, are reproduced.
- An input the reference rejects is rejected too. Errors that come from a diagram's DB keep the reference's message; parse errors are ferrigura's own, as its parsers are ([0007](./0007-hand-written-parsers.md)).

## Why

The promise is compatibility: a user moves a diagram from Mermaid to ferrigura and gets the same picture. A quirk that ferrigura fixes on its own breaks that promise for exactly the texts that depend on it, and makes the comparison with the reference report a difference that nobody can tell from a regression. A patch on the reference keeps a fix visible, checkable and ready to go upstream.

Weighed and not chosen: fixing what looks wrong, which loses the comparison as a check; and reproducing everything, garbage included, which ties ferrigura to output nobody can use.

## Consequences

- The easy mistake in a port is to fix a quirk because it looks like a bug; reviews check for it.
- Quirks are documented where the code reproduces them, and on the type's page under [diagrams/](../diagrams/README.md).
- When the pin moves past an upstream fix, the patch goes and ferrigura follows the new behavior.

## Related

[0014](./0014-upstream-reference-at-a-pin.md), [0015](./0015-differential-testing.md), [0018](./0018-deterministic-output.md); [LEARNINGS.md](../LEARNINGS.md#reproduce-quirks-and-name-the-exceptions).
