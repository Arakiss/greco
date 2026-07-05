# T2 Movability

Friction profile: unnecessary reads of large generated files and wasted turns
when a small index already points at the exact file and line.

Hypothesis: a Layer A cached procedure that says "consult `INDEX.md` before
opening generated files" should reduce read volume and turns while preserving
objective success.
