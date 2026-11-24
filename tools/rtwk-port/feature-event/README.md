add_event.py: gives every feature slice `type Event = Never<Self>;` (ADR-012 "the stream", fw-hubs-b P4), after its
`type Interest`. Idempotent; a slice with its own Event is skipped. Run from the repo root: `python3 -I tools/rtwk-port/feature-event/add_event.py`.
