# Historical status

Status: **frozen historical reference**
Decision date: **2026-07-13**

## Decision

Heimlern is not continued as an effect evaluator, routing or pattern-recognition service, friction-analysis system, policy engine, dashboard backend or other runtime component. Existing source code, schemas, examples and reports remain available only as implementation and design history.

Generic deterministic observation-capture and effect-calculation utilities remain in Vibe-Lab as unpromoted review tooling. They are not transferred to Heimlern and have no automatic decision, routing, queue or policy authority.

## Evidence basis

The registered Vibe-Lab salvage experiment compared manual review with evaluator-assisted review. Its frozen treatment required an evaluator report before the treatment decision, while the implemented report could only be generated from the already captured and independently scored control and treatment decisions. The report was therefore downstream of the decision it was supposed to inform.

The experiment was archived in `heimgewebe/vibe-lab` PR #314. It recorded:

- zero completed pilots;
- zero observations;
- zero synthetic cases;
- zero retrospective cases;
- one practical blocker: causal-ordering circularity.

No data was invented to manufacture a usefulness result.

## Non-claims

This decision does not establish that manual review is superior, that deterministic summaries are generally useless, or that every future learning system is without value. It establishes only that the registered salvage experiment could not honestly test the merged intervention and that no independent evidence currently justifies Heimlern as an active product.

## Reactivation gate

Reactivation requires a new, separately registered experiment that:

1. defines a non-circular intervention available before the decision it may influence;
2. uses real prospective cases without duplicate productive side effects;
3. pre-registers the decision target, measurement and stop rule;
4. demonstrates material operator value beyond the existing manual process;
5. preserves proposal-only boundaries and adds no automatic routing or policy authority;
6. is reviewed and adopted outside this repository before implementation work begins.

Absent that evidence, only preservation, security fixes and documentation corrections are in scope.
