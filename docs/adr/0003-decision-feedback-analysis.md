# ADR-0003: Decision Feedback Analysis & Weight-Tuning

**Status:** Accepted  
**Datum:** 2026-01-04  
**Kontext:** Issue #5b – heimlern: Decision Feedback Analysis & Weight-Tuning

## Kontext und Problemstellung

hausKI trifft Entscheidungen basierend auf Policies (z.B. RemindBandit). Diese Entscheidungen haben Outcomes, die zeitversetzt bewertet werden können. Es fehlt ein Mechanismus, um:

1. Entscheidungs-Outcomes retrospektiv zu analysieren
2. Systematische Fehlgewichtungen zu erkennen
3. Kontrollierte Anpassungen vorzuschlagen (nicht durchzuführen!)
4. Vorschläge zu simulieren und zu auditieren

**Kernfrage:** Wie lernt heimlern, ohne die Integrität des Entscheidungssystems zu gefährden?

## Entscheidungstreiber

* **Rollentrennung:** heimlern analysiert, hausKI entscheidet und wendet an
* **Auditierbarkeit:** Jede Anpassung muss nachvollziehbar sein
* **Sicherheit:** Kein Auto-Apply, keine stillen Änderungen
* **Evidenz:** Vorschläge basieren auf analysierten Daten, nicht Spekulation
* **Reversibilität:** Anpassungen müssen zurücknehmbar sein

## Betrachtete Optionen

### Option 1: Online-Learning (abgelehnt)

Policy passt Gewichte direkt während der Laufzeit an.

**Pro:**
- Schnell reaktiv
- Einfache Implementierung

**Contra:**
- Keine Audit-Möglichkeit
- Gefahr von Feedback-Loops
- Keine Simulation möglich
- Rollenkonflikt (Entscheider ≠ Lerner)

### Option 2: Read-Only Analysis (teilweise)

heimlern analysiert nur, schlägt aber nie Änderungen vor.

**Pro:**
- Maximale Sicherheit
- Klare Rollentrennung

**Contra:**
- Keine automatische Verbesserung
- Manuelle Interpretation nötig
- Langsamer Lernzyklus

### Option 3: Proposal-Based Tuning (gewählt) ✓

heimlern erzeugt Anpassungsvorschläge mit Evidenz, hausKI entscheidet über Annahme.

**Pro:**
- Klare Rollentrennung
- Auditierbar und transparent
- Simulation vor Anwendung möglich
- Versionierung natürlich
- Menschliche Freigabe-Gate optional

**Contra:**
- Komplexer als Online-Learning
- Verzögerung zwischen Analyse und Anwendung

## Entscheidung

**Gewählt: Option 3 – Proposal-Based Tuning**

heimlern implementiert:

1. **Input-Pipeline:**
   - Konsumiert `decision.outcome.v1` (von hausKI/chronik)
   - Aggregiert nach Intent, Kontext, Trust-Level

2. **Feedback-Analyse (heuristisch, kein ML):**
   - Erkennt Muster (z.B. wiederholte Fehler bei trust_level=low)
   - Identifiziert Bias (z.B. alte Einträge überbewertet)
   - Berechnet Statistiken (Success-Rate, Average Reward)

3. **Weight-Delta-Vorschläge:**
   - Format: `policy.weight_adjustment.proposed.v1`
   - Enthält: Deltas, Confidence, Evidence, Reasoning
   - Kein Auto-Apply

4. **Simulation:**
   - Replay historischer Entscheidungen mit neuen Gewichten
   - Vergleich: Success-Rate, Varianz, Drift
   - Evidenz für Proposal

5. **Export:**
   - Versioniert als `policy.snapshot.vX`
   - Übergabe an hausKI
   - hausKI entscheidet über Annahme/Ablehnung

6. **Observability:**
   - Metriken: `learning_cycles_total`, `weight_adjustments_proposed_total`, `weight_adjustments_accepted_total`
   - Logs: Begründungen, Ablehnungen, Rollbacks
   - Audit-Trail für alle Vorschläge

## Konsequenzen

### Positiv

* ✅ **Rollentrennung:** heimlern lernt, hausKI handelt
* ✅ **Sicherheit:** Keine stillen Änderungen, menschliche Gates möglich
* ✅ **Auditierbarkeit:** Jede Anpassung dokumentiert und begründet
* ✅ **Reversibilität:** Versionierung ermöglicht Rollbacks
* ✅ **Simulation:** Risikominimierung durch Vorab-Tests

### Negativ

* ⚠️ **Latenz:** Verzögerung zwischen Erkennung und Anwendung
* ⚠️ **Komplexität:** Mehr bewegliche Teile als direktes Learning
* ⚠️ **Simulationsgüte:** Vereinfachte Simulation kann optimistisch sein

### Neutral

* ℹ️ **Heuristik statt ML:** Bewusste Wahl für Transparenz, kann später erweitert werden
* ℹ️ **Min-Threshold:** Proposals nur bei genug Evidenz (vermeidet Noise-Overfitting)

## Implementierung

Neue Crate: `heimlern-feedback`

Kerntypen:
- `DecisionOutcome`: Input (von hausKI)
- `WeightAdjustmentProposal`: Output (an hausKI)
- `FeedbackAnalyzer`: Analyse-Engine
- `OutcomeStatistics`: Aggregierte Metriken

Beispiel:
```bash
cargo run -p heimlern-feedback --example feedback_analysis
```

## Compliance

Verträge (contracts/):
- `decision.outcome.schema.json` (Input)
- `policy.weight_adjustment.schema.json` (Output)

JSON-Schema-Validierung in CI.

**Hinweis zur Contract-Ownership:**
Die Schemas sind aktuell im heimlern-Repo definiert (Payload-Strukturen).
Idealerweise würden diese im metarepo als Single Source of Truth verwaltet,
mit Synchronisierungsmechanismus zu konsumierenden Repos. Dies erlaubt:
- Zentrale Versionskontrolle
- Vermeidung von Schema-Divergenz
- Klare Ownership-Struktur

Für die initiale Implementierung bleiben die Schemas hier, mit dem 
Verständnis, dass eine spätere Migration ins metarepo sinnvoll sein kann.

Die Schemas definieren explizit **Payload-Strukturen**, nicht Event-Envelopes.
Event-Transport über chronik/plexer erfordert separate Envelope-Spezifikation.

## Offene Fragen

1. **Meta-Learning:** Wann sollte heimlern NICHT lernen?
   - Zu wenig Daten → warten
   - Zu viel Drift → menschliche Prüfung
   - Widersprüchliche Signale → Warnung

2. **Simulation-Genauigkeit:** Wie realistisch ist das Replay?
   - Aktuell: Vereinfachte Schätzung
   - Zukünftig: Vollständiges Replay mit modifizierten Weights

3. **Confidence-Kalibrierung:** Ist die Konfidenzberechnung robust?
   - Aktuell: Heuristik (Sample-Size + Pattern-Count)
   - Zukünftig: Kalibrierung gegen Ground-Truth

## Referenzen

* Issue: heimlern #5b (Decision Feedback Analysis & Weight-Tuning)
* Organismus-Kontext: metarepo/docs/heimgewebe-organismus.md
* ADR-0001: Policy-Trait Design
* ADR-0002: Policy Snapshot Persistenz

## Zitat aus dem Issue

> hausKI handelt.  
> heimlern bewertet.  
> Und Lernen entsteht erst dort,  
> wo Fehler nicht verborgen, sondern ausgewertet werden.

Lernen ist Spurverstärkung, nicht Beliebigkeitsanpassung.
