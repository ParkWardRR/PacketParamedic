# Project Personas

This document outlines the primary user personas for **PacketParamedic**. Understanding these personas helps guide feature development, UI design, and documentation.

## Persona Map

| Persona | Real-world label | Core motivation | What they “buy” PacketParamedic for | Primary adoption risk |
|---|---|---|---|---|
| **“Home tech interested noob”** | Home server beginner vibe | Learn + regain control without deep expertise | “Tell me whose fault it is” + simple health checks | CLI/config feels scary; Pi 5 requirement |
| **“Home normal user”** | Non-hobbyist household | Reliability for work/streaming | Proof to send to ISP + fewer “is it down?” mysteries | Anything that looks like a project, not an appliance |
| **“Tech home expert”** | Homelab / network-savvy | Observability + automation | Local dataset + API + evidence export + schedule control | Missing UI polish |

---

## Persona 1: Home tech‑interested noob (“Home server energy”)

**Name & vibe**: Alex (29), curious, proud of running a home server, but still guesses wrong about “Wi‑Fi vs ISP vs website.”  
**Home setup** (typical): consumer router + unmanaged switch + a small always-on box; willing to add a Pi if it’s “one more little thing”.

### What “pain” triggers adoption
Alex’s trigger is the recurring pattern: “Netflix buffers + Zoom glitches, but speedtest.net sometimes looks fine,” so the household debates whether to reboot the router, call the ISP, or blame the service. PacketParamedic’s pitch lands because it’s explicitly built to answer that question by running multiple probe types over time and producing a blame verdict rather than a single snapshot test.

### What they actually do with PacketParamedic (workflow)
Alex treats PacketParamedic like a specialized diagnostic appliance: plug it in, let default schedules run, check in only when something feels off, then look for a single sentence answer (“isp” vs “me” vs “service”).  
They rely on the shipped default schedules (minute-by-minute gateway ping, 5‑minute DNS+HTTP checks, nightly speed test, weekly blame check) rather than designing their own measurement plan.  
When the internet feels broken, they run a manual blame-check, then export a support bundle so they can paste one ZIP’s worth of evidence into an ISP ticket without understanding every metric.

### Features they use (and ignore)
Alex uses the daemon/API mode primarily so “it’s just running,” and they only touch the CLI for a few commands (serve, blame-check, export-bundle, maybe selftest once).  
They mostly ignore hardware acceleration details and the deeper roadmap ideas (BLE admin, iOS companion app, advanced RF diagnostics) until those become “push-button” experiences.

### Success criteria (what “good” looks like)
Alex is successful if PacketParamedic reduces random rebooting and replaces it with a repeatable answer plus a confidence level they trust enough to act on.  
They also feel successful if the tool helps them avoid self-blame (“it’s not my Wi‑Fi”) or avoid pointless ISP calls (“service is down”).

### Alex’s “minimum viable commands”
```bash
# keep it running as a network appliance (daemon + scheduler + API)
packetparamedic serve --bind 0.0.0.0:8080

# quick “who broke it?” check
packetparamedic blame-check

# export evidence for support / future you
packetparamedic export-bundle --output bundle.zip
```

### Likely friction + how they work around it
Alex will get stuck on “where does the database live / what env vars do I need,” so they’ll prefer defaults and copy `.env` patterns if provided. They may also bounce if they don’t have a Pi 5 specifically, because the project is explicitly positioned as “Raspberry Pi 5 only.”

### Estimated hardware cost
If Alex doesn’t already own a suitable always-on Linux box, they’ll likely price a Raspberry Pi 5 plus storage/power/case; typically in the $90–$180 range depending on accessories.

---

## Persona 2: Home normal user (“just make the internet work”)

**Name & vibe**: Jamie (41), WFH + kids + streaming household, allergic to tinkering, but willing to buy one box if it ends arguments and reduces downtime.  
**Trust model**: Jamie doesn’t trust dashboards; they trust a clear verdict and a timestamped history they can show someone else.

### What “pain” triggers adoption
Jamie’s trigger is high-stakes connectivity moments: a work call drop, a kid’s online test, or “the TV is buffering again,” followed by the ambiguity of whether it’s the ISP, the router, or the service. PacketParamedic fits because it’s designed as an “appliance” that continually gathers evidence (scheduled probes + speed tests) and stores it locally, rather than a one-time test.

### How Jamie uses it (behavioral pattern)
Jamie’s ideal interaction is “set-and-forget,” so PacketParamedic must behave like a background appliance (daemon + scheduler) with occasional checks of “incidents” and “latest speed test” when something feels wrong.  
They will mainly consume outputs indirectly: a spouse/partner or friend might run the CLI, or Jamie might only view results through the Web UI or API client.

### The “ISP phone call” scenario (where it shines)
Jamie uses PacketParamedic as a credibility tool: “Here’s the history, here’s when it failed, here’s what the box thinks is ISP vs me vs service,” plus an exported bundle to attach to a ticket. This is specifically aligned with the project’s emphasis on blame analysis and evidence/support bundle export, not just raw ping graphs.

### What they need before they’ll keep it
Jamie needs the system to avoid disrupting the network (e.g., heavy tests not overlapping). They also need privacy confidence; the system runs entirely on the Pi 5 with no cloud subscription.

### Estimated hardware cost
Jamie will only adopt if the appliance cost feels like “one-time utility purchase”; likely tolerating $150–$300 if it prevents recurring outages and support calls.

---

## Persona 3: Tech home expert (“homelab / network engineer at home”)

**Name & vibe**: Sam (35), runs VLANs, knows DNS, owns a rack or at least a structured wiring panel, and wants *observability* not vibes.  
**Why this project is interesting to them**: It combines scheduled active measurements (probes + throughput) with local persistence (SQLite WAL) and a programmatic API surface, which can be integrated into a broader monitoring stack.

### What “pain” triggers adoption
Sam’s trigger is not “internet down,” it’s “internet degraded intermittently and I need hard attribution,” especially when multiple layers could be responsible (AP roaming, bufferbloat, ISP congestion, upstream DNS issues). PacketParamedic is appealing because it explicitly models attribution (“me/isp/service”) and is designed to accumulate longitudinal data for anomaly detection and classification.

### How Sam uses it (deep usage)
Sam will customize schedules aggressively: keep lightweight probes frequent, constrain bandwidth-heavy tests to known quiet windows, and cap automated test data usage using env vars.  
They’ll likely treat the REST API (`/api/v1/*`) as the contract, pulling “incidents,” “speed-test history,” and “self-test latest” into their own dashboards/alerts.  
They will run hardware self-tests when commissioning the box and after upgrades.

### What Sam will contribute or extend
Sam is the persona most likely to build from source, cross-compile, package (deb/container), and potentially contribute to probe breadth, throughput engine choices, or acceleration paths. They’ll also care about “evidence export” as an incident artifact they can archive.

### Sam’s “integration-first” commands
```bash
# run the service on a stable host address so other systems can query the API
packetparamedic serve --bind 0.0.0.0:8080

# see what schedules are configured and validate a 24h run plan
packetparamedic schedule list
packetparamedic schedule dry-run --hours 24
```

### Estimated hardware cost
Sam will optimize for reliability and 24/7 operation; likely dedicating a Pi 5 or repurposing a low-power mini PC.
