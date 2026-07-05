# T3 Movability

Friction profile: edit-without-read failures, repeated tool errors, and
retracements when multiple edits target nearby lines in the same file.

Hypothesis: a Layer A cached procedure that re-reads a file immediately before
editing it again after other tool activity should reduce edit-conflict counters
without hurting objective success.
