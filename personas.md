You’ve got three clear “home” persona buckets for **PacketParamedic**: (1) a Pi-hole-style curious noob who wants answers without becoming a network engineer, (2) a “just make Wi‑Fi work” normal household user who wants simple blame + receipts for the ISP, and (3) a homelab/network power user who will wire PacketParamedic into dashboards/automation and tune schedules, probes, and evidence exports. 

## What I read (repo facts)
PacketParamedic is a Rust program that turns a **Raspberry Pi 5** into a local-only network diagnostic appliance that runs scheduled probes (ICMP/TCP/DNS/HTTP), speed tests, anomaly detection, and a “blame classifier” that tries to determine whether issues are “me / isp / service,” with results stored in a local SQLite (WAL) database and exposed via a REST API.   
The repo documents a CLI (`packetparamedic`) for serving the daemon/API, self-test, blame-check, speed tests, schedule management, and support-bundle export, plus default schedules shipped in `config/schedules.toml`.   
Key repo files I used: [README.md](https://github.com/ParkWardRR/PacketParamedic/blob/8f0fcbeb52fa2f25e7f056496635c635c49f4bb5/README.md), [config/schedules.toml](https://github.com/ParkWardRR/PacketParamedic/blob/8f0fcbeb52fa2f25e7f056496635c635c49f4bb5/config/schedules.toml), and [docs/ACCELERATION_STRATEGY.md](https://github.com/ParkWardRR/PacketParamedic/blob/8f0fcbeb52fa2f25e7f056496635c635c49f4bb5/docs/ACCELERATION_STRATEGY.md). 

## Persona map (space-efficient)
| Persona | Real-world label | Core motivation | What they “buy” PacketParamedic for | Primary adoption risk |
|---|---|---|---|---|
| “Home tech interested noob” | Pi-hole majority vibe | Learn + regain control without deep expertise | “Tell me whose fault it is” + simple health checks | CLI/config feels scary; Pi 5 requirement |
| “Home normal user” | Non-hobbyist household | Reliability for work/streaming | Proof to send to ISP + fewer “is it down?” mysteries | Anything that looks like a project, not an appliance |
| “Tech home expert” | Homelab / network-savvy | Observability + automation | Local dataset + API + evidence export + schedule control | Missing UI polish (roadmap phases) |

(Everything in this table is persona synthesis; the feature claims it maps to are from the repo.) 

## Persona 1: Home tech‑interested noob (“Pi-hole user energy”)
**Name & vibe**: Alex (29), curious, proud of running Pi-hole, but still guesses wrong about “Wi‑Fi vs ISP vs website.”  
**Home setup** (typical): consumer router + unmanaged switch + a small always-on box; willing to add a Pi if it’s “one more little thing” (I’m guessing the physical setup because the repo doesn’t dictate topology). 

### What “pain” triggers adoption
Alex’s trigger is the recurring pattern: “Netflix buffers + Zoom glitches, but speedtest.net sometimes looks fine,” so the household debates whether to reboot the router, call the ISP, or blame the service.   
PacketParamedic’s pitch lands because it’s explicitly built to answer that question by running multiple probe types over time and producing a blame verdict rather than a single snapshot test. 

### What they actually do with PacketParamedic (workflow)
Alex treats PacketParamedic like “Pi-hole for diagnosis”: plug it in, let default schedules run, check in only when something feels off, then look for a single sentence answer (“isp” vs “me” vs “service”).   
They rely on the shipped default schedules (minute-by-minute gateway ping, 5‑minute DNS+HTTP checks, nightly speed test, weekly blame check) rather than designing their own measurement plan.   
When the internet feels broken, they run a manual blame-check, then export a support bundle so they can paste one ZIP’s worth of evidence into an ISP ticket without understanding every metric. 

### Features they use (and ignore)
Alex uses the daemon/API mode primarily so “it’s just running,” and they only touch the CLI for a few commands (serve, blame-check, export-bundle, maybe selftest once).   
They mostly ignore hardware acceleration details and the deeper roadmap ideas (BLE admin, iOS companion app, advanced RF diagnostics) until those become “push-button” experiences. 

### Success criteria (what “good” looks like)
Alex is successful if PacketParamedic reduces random rebooting and replaces it with a repeatable answer plus a confidence level they trust enough to act on.   
They also feel successful if the tool helps them avoid self-blame (“it’s not my Wi‑Fi”) or avoid pointless ISP calls (“service is down”). 

### Alex’s “minimum viable commands” (copy/paste)
```bash
# keep it running as a network appliance (daemon + scheduler + API)
packetparamedic serve --bind 0.0.0.0:8080

# quick “who broke it?” check
packetparamedic blame-check

# export evidence for support / future you
packetparamedic export-bundle --output bundle.zip
```
(These commands are taken directly from the repo README’s CLI examples.) 

### Likely friction + how they work around it
Alex will get stuck on “where does the database live / what env vars do I need,” so they’ll prefer defaults and copy `.env` patterns if provided (the repo documents env vars like bind addr, db path, data dir, bandwidth budget, speed-test window).   
They may also bounce if they don’t have a Pi 5 specifically, because the project is explicitly positioned as “Raspberry Pi 5 only.” 

### Approx cost (explicitly a guess)
If Alex doesn’t already own a suitable always-on Linux box, they’ll likely price a Raspberry Pi 5 plus storage/power/case; I’m **guessing** a typical all-in range of about $90–$180 depending on accessories and RAM/storage choices (verify current retail pricing). 

## Persona 2: Home normal user (“just make the internet work”)
**Name & vibe**: Jamie (41), WFH + kids + streaming household, allergic to tinkering, but willing to buy one box if it ends arguments and reduces downtime.  
**Trust model**: Jamie doesn’t trust dashboards; they trust a clear verdict and a timestamped history they can show someone else. 

### What “pain” triggers adoption
Jamie’s trigger is high-stakes connectivity moments: a work call drop, a kid’s online test, or “the TV is buffering again,” followed by the ambiguity of whether it’s the ISP, the router, or the service.   
PacketParamedic fits because it’s designed as an “appliance” that continually gathers evidence (scheduled probes + speed tests) and stores it locally, rather than a one-time test. 

### How Jamie uses it (behavioral pattern)
Jamie’s ideal interaction is “set-and-forget,” so PacketParamedic must behave like a background appliance (daemon + scheduler) with occasional checks of “incidents” and “latest speed test” when something feels wrong.   
They will mainly consume outputs indirectly: a spouse/partner or friend might run the CLI, or Jamie might only view results through whatever UI/API client exists, because the repo defines REST endpoints for health, incidents, probe status, speed-test latest/history, schedules, and network interfaces. 

### The “ISP phone call” scenario (where it shines)
Jamie uses PacketParamedic as a credibility tool: “Here’s the history, here’s when it failed, here’s what the box thinks is ISP vs me vs service,” plus an exported bundle to attach to a ticket.   
This is specifically aligned with the project’s emphasis on blame analysis and evidence/support bundle export, not just raw ping graphs. 

### What they need before they’ll keep it
Jamie needs the system to avoid disrupting the network (e.g., heavy tests not overlapping); the repo explicitly notes coordination so only one heavy throughput test runs at a time via a semaphore.   
They also need privacy confidence; the README states “no cloud subscription required” and frames it as running entirely on the Pi 5. 

### Approx cost (explicitly a guess)
Jamie will only adopt if the appliance cost feels like “one-time utility purchase”; I’m **guessing** they’ll tolerate something in the ~$150–$300 range if it prevents recurring outages and support calls, but pricing is outside what the repo states. 

## Persona 3: Tech home expert (“homelab / network engineer at home”)
**Name & vibe**: Sam (35), runs VLANs, knows DNS, owns a rack or at least a structured wiring panel, and wants *observability* not vibes.  
**Why this project is interesting to them**: It combines scheduled active measurements (probes + throughput) with local persistence (SQLite WAL) and a programmatic API surface, which can be integrated into a broader monitoring stack. 

### What “pain” triggers adoption
Sam’s trigger is not “internet down,” it’s “internet degraded intermittently and I need hard attribution,” especially when multiple layers could be responsible (AP roaming, bufferbloat, ISP congestion, upstream DNS issues).   
PacketParamedic is appealing because it explicitly models attribution (“me/isp/service”) and is designed to accumulate longitudinal data for anomaly detection and classification rather than only presenting raw metrics. 

### How Sam uses it (deep usage)
Sam will customize schedules aggressively: keep lightweight probes frequent, constrain bandwidth-heavy tests to known quiet windows, and cap automated test data usage using the env vars described for speed-test windows and daily bandwidth budget.   
They’ll likely treat the REST API (`/api/v1/*`) as the contract, pulling “incidents,” “speed-test history,” and “self-test latest” into their own dashboards/alerts, rather than waiting for the project’s htmx UI (which the roadmap shows as not started).   
They will run hardware self-tests when commissioning the box and after upgrades, because the project includes a self-test subsystem meant to validate “is this actually a Pi 5,” plus thermal/NIC checks per the roadmap framing. 

### What Sam will contribute or extend
Sam is the persona most likely to build from source, cross-compile, package (deb/container), and potentially contribute to probe breadth, throughput engine choices (iperf3 integration vs native), or acceleration paths (NEON/Vulkan/GLES) because the repo explicitly outlines these components and scripts.   
They’ll also care about “evidence export” as an incident artifact they can archive alongside other logs, because the repo treats export-bundle as a first-class CLI action. 

### Sam’s “integration-first” commands (copy/paste)
```bash
# run the service on a stable host address so other systems can query the API
packetparamedic serve --bind 0.0.0.0:8080

# see what schedules are configured and validate a 24h run plan
packetparamedic schedule list
packetparamedic schedule dry-run --hours 24
```
(These are directly from the README’s CLI examples.) 

### Approx cost (explicitly a guess)
Sam will optimize for reliability and 24/7 operation; I’m **guessing** they’ll either dedicate a Pi 5 as intended or repurpose a low-power mini PC, and they’ll think in “under $300 hardware, near-zero ongoing cost” terms (verify with your preferred hardware source). 

## If you want, I can tailor these to your intended UX
Do you want these personas to assume PacketParamedic is (A) a DIY Rust binary you compile, (B) a packaged .deb you install, or (C) a prebuilt Pi image “appliance”?
