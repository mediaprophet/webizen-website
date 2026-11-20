# Webizen website

The public website for a **human-centric computing ecosystem**, grounded in the natural world and its intersection with the infosphere: physics, nature, the biosphere, biology, sociology and the liberal arts.

This repository owns the website's editorial content, world design, scene recipes, publication data, accessible reading experience, module and demonstration indexes, migration and deployment. Component implementation stays in its owning repository.

## Current state

Local repository initialised on 16 November 2026. No remote, publication or domain change. The initial three abstract scene studies were rejected and are not the production foundation. The site-wide overhaul remains in design review.

- [Human rights, purposes, needs and support demonstrations](plans/HUMAN-SUPPORT-DEMONSTRATIONS.md)
- [Observer-relative `.q42`/Vibe path architecture](plans/OBSERVER-PATHS.md)
- [Controlling world design brief](plans/WORLD-DESIGN-BRIEF.md)
- [Comprehensive renewal plan](plans/renewal/README.md)
- [Full source-page and demonstration inventory](data/demos.json)
- [Module coverage](data/modules.json)
- [World/discipline mapping](data/places.json)
- [150-operation capability inventory](data/capabilities.json)
- [Current Synth MCP verification](evidence/current-synth-mcp.json)

## Implementation plan and revised review world

The current swarm-ready plan is [SWARM-IMPLEMENTATION-PLAN.md](plans/swarm/SWARM-IMPLEMENTATION-PLAN.md), with [18 dispatchable packages](plans/swarm/work-packages.json), a [Lab preservation audit](plans/swarm/lab-audit.md) and an [architecture audit](plans/swarm/architecture-audit.md). Package states are queued; writing the plan does not mean full implementation is complete. Australian English (`en-AU`) is the default.

Run `python tooling/world_server.py --port 8800`, then open http://127.0.0.1:8800/. The revised natural-world prototype has six places, current native Synth production experiments and a QualiaPortal terrain inspection route. The main world currently uses Three.js. Native producer/render identity and unfinished q42/observer integration are described in the plan and in the site's About view.

The earlier review index remains at `/inventory.html`; original QualiaDB sources are served under `/source/`. The source inventory is not a claim that every demo has passed runtime qualification. The Infosphere Lab plan preserves or qualifies each workload before retiring its prior UI.

The language layer accepts Unicode locales and supports region/script fallback, RTL, locale-aware formatting and explicit English fallback. Six draft language families are provided; this is not a claim of complete reviewed translations. A remembered explicit locale overrides the Australian-English default.

## Licensing

Copyright (c) 2026 Timothy Charles Holborn  
https://www.linkedin.com/in/ubiquitous/ | timothy.holborn@gmail.com  
**License:** Attribution-NonCommercial-NoDerivatives 4.0 International (`CC BY-NC-ND 4.0`) — see [`LICENSE`](LICENSE) and [Creative Commons Legal Code](https://creativecommons.org/licenses/by-nc-nd/4.0/legalcode).

## Ownership

Keep source-generated capability facts and human-authored explanations separate. Every module explains its purpose, distinctive approach, human significance, ecosystem relationships and actual availability. Every demo identifies its workload, runtime profile, evidence and accessible route. Every existing page gets a migration disposition.

Normative specifications, benchmarks and API sources remain with the responsible components. This website consumes pinned, validated publication bundles. Large generated media is published as immutable artifacts with receipts rather than copied into Git history.
