# Heimlern proposal registrations

Every new learning or policy-adjustment proposal is registered as `proposals/<proposal-id>.json` and must validate against `contracts/learning.proposal.registration.v1.schema.json`.

The registration fails closed unless it names:

- a downstream consumer and concrete use;
- the reviewed decision and its owner;
- a reproducible measure plus success and falsification thresholds;
- review and expiry timestamps;
- deterministic `promote`, `reject`, or `archive` closure;
- proposal-only boundaries that prohibit automatic policy, routing, queue, or runtime effects.

Historical reports outside `proposals/` remain readable but are not treated as active registrations. Copy `_template.json`, replace every placeholder, and keep the resulting file until one declared closure outcome is recorded and reviewed.
