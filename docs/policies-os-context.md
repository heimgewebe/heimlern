# heimlern: Policies für OS-Kontext

## Ziele
- Consent erzwingen
- Sensitive Kontexte blocken
- Rate-Limits & PII-Gate
- Automations (Deep-Work, Selbstheilung)

## Kern-Policies (YAML-Skizze)
```yaml
consent:
  text_capture: false   # muss aktiv vom Nutzer gesetzt werden

pii_gate:
  min_confidence: 0.85
  on_violation: drop_and_shred

rate_limits:
  embed_per_app_per_min: 12
  on_exceed: drop

allow_block:
  allow_apps: [code, obsidian]
  allow_domains: ["localhost", "dev.local"]
  block_apps: ["org.keepassxc.KeePassXC", "com.bank.app"]
  block_domains: ["login.microsoftonline.com", "accounts.google.com"]

modes:
  deep_work:
    enter_if:
      - os.context.state.focus == true
      - hauski_audio.vibe in ["fokussiert", "neutral"]
      - app in ["code", "obsidian"]
    actions:
      - hausKI.hold_notifications
    exit_if:
      - focus == false OR inactivity > 10m
    exit_actions:
      - hausKI.release_notifications
```

## Selbstheilung

- Metriken aus hausKI/wgx beobachten; bei Silence/Latenz → Entscheidung: `wgx doctor`/Restart (lokal), auditierbar.
