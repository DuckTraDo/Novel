# 📖 Sodarie Novel · User Guide

Welcome! This is a "you steer, the model rows" machine for writing long-form fiction.

The core idea is simple: **you tell it in one sentence what should happen in a chapter, and it drafts the whole chapter at once. The things that usually fall apart over a long book — characters contradicting themselves, forgotten foreshadowing, a setting from Chapter 3 that has mutated by Chapter 30 — are watched over by "long-term memory".**

Five minutes and you'll have it. ☕

---

## 🌀 The whole flow in one minute

```
Prepare memory → write a one-line chapter idea → generate → review/edit → consistency check → update memory → next chapter
```

Every chapter goes around this loop. The characters, events and foreshadowing you wrote earlier get recorded in "memory" and fed into later chapters — so the book gets *more* consistent as it grows, not messier.

> Mantra: **the idea decides what this chapter is about; the memory decides whether it lines up with everything before.**

---

## 🍱 Before you start: feed the Memory

The Memory is this book's "setting bible + ledger". Before writing each chapter, the model reads through it. The more care you put in, the more it reads like *your* book instead of generic web-fiction.

Edit it under the **🧠 Memory** tab. For a new book, fill at least the first three (Story Bible, Characters, Book Outline); the rest can grow as you write.

### 1. Story Bible `story_bible.yaml` — the constitution

World rules, themes, hard constraints. If the model breaks something here, the consistency check will flag it.

```yaml
world_rules:        # what can / cannot happen in this world
  - A near-future county town in southern China; no supernatural powers
  - The dead stay dead; injuries take time to heal
writing_rules:      # the prose style you want
  - Imply emotion through action and detail; avoid flat lines like "he was sad"
  - Keep dialogue short and true to each character
forbidden_patterns: # lazy phrasings to avoid
  - Don't open paragraphs with "suddenly"
  - No "time flew by" hand-waving
```

> 💡 Stuck? Listing what must **never** appear in this book is often more useful than listing its virtues.

### 2. Characters `characters.yaml` — who's who, and what they know

The single most important file. Fields:

| Field | Meaning | Why it matters |
|---|---|---|
| `name` | Character name | Memory updates find people by name — **keep it consistent** |
| `current_status` | Current state / situation | Tells the model where they are and what they're doing |
| `knows` | What they **already know** | Stops a character "knowing what they shouldn't" |
| `secrets` | Secrets they hide | Prevents premature reveals |
| `relationships` | Relations with others | Keeps interactions sensible |
| `constraints` | Things they can't / won't do | Holds the character's persona |

```yaml
characters:
  - name: Lin Chuan
    current_status: Just back in his hometown, staying at the old house
    knows:
      - His father died three years ago
    secrets:
      - He was actually the one who left home first
    relationships:
      - Estranged from his cousin Lin Wan
    constraints:
      - Bad at lying; avoids eye contact when nervous
```

> ⚠️ `knows` and `secrets` are the key to "no spoilers". Put the villain's true identity only in someone's `secrets`, and the model won't let other characters blurt it out early.

### 3. Book Outline `book_outline.yaml` — the big picture

The overall direction, volumes, key beats. It needn't be detailed — just a "map" so the model doesn't drop a Chapter-5 climax into Chapter 1.

### 4. Foreshadowing `foreshadowing.yaml` — plant it, don't forget it

Split into `active` (planted, not yet paid off) and `resolved`. Usually you don't write this by hand — running "Update memory" registers it for you. Manual entries work too:

```yaml
active:
  - id: fs001
    description: A locked wooden box in the attic of the old house
    planned_resolution: Volume 2 reveals it holds the mother's diary
resolved: []
```

### 5. Style Bank `style_bank.jsonl` — let the model copy your voice

One JSON object per line, holding prose you like (your own, or admired samples). The model mimics this tone.

```json
{"id": "style01", "text": "Rain hammered the tin roof, like someone overhead endlessly counting coins."}
```

> 💡 This is where "voice" comes from. Three to five snippets is plenty — quality over quantity.

### 6. Auto-maintained files (hands off) 🤚

`Chapter Summaries / Events / Timeline / Relationships` — **don't edit these by hand** in general. They're the ledger generated automatically each time you run "Update memory". Leave them to the program unless you need to fix a specific mistake.

---

## ✍️ The Workbench: what every button does

Open the **✍️ Workbench**. Top to bottom:

### Select chapter / New chapter ID / Auto number
- **Select chapter**: pick an existing chapter to view or rewrite.
- **New chapter ID**: when writing a new chapter, type the id here (e.g. `ch001`). **Auto number** bumps it by +1 based on existing chapters so you don't have to count.

### Chapter idea — one line (highest priority)
Tell the model what happens this chapter. **It has the highest priority** — even if it conflicts with memory, the idea wins (potential conflicts are only *noted* in the report, never forced into the text).

Example:
> Ch.1 — the hero returns home, finds a letter left by his father, and decides to dig into an old case.

No need to write a full outline; one or two sentences naming "who, where, what they do, what turns" is enough. The more specific, the less drift.

### Target length (chars)
Roughly how many characters this chapter should be, default 4000. It's a **target, not a hard cap** — the actual length varies.

### ☑ Use long-term memory — when to check, when not to
- **Checked (default, recommended)**: feeds the memory (characters, foreshadowing, recent recaps…) to the model → output stays consistent with earlier chapters. **For normal serialized writing, keep it on.**
- **Unchecked**: writes from this one idea only, ignoring all prior setup. Good for:
  - the **very first chapter**, before any memory exists;
  - a **side story / flashback / standalone chapter** you don't want constrained by the main canon;
  - simply letting the model "free-run" to try a style.

### ☑ Overwrite existing text
This chapter already has text and you want to **regenerate** → check it; otherwise the program blocks you to prevent accidental deletion. For a brand-new chapter it doesn't matter.

### Generate / Regenerate
Click it and the model writes the whole chapter. The button spins — **be patient**: a local model writing 4000 characters usually takes tens of seconds to a few minutes. The text then appears below, along with a Generation report.

### Chapter text (editable) + Save text
The generated text sits in the box and **you can edit it freely** (fix typos, tweak sentences, cut paragraphs). Click **Save text** when done. Not happy overall? Regenerate from above.

### Consistency check
Makes the model play editor: it checks this chapter against the character files, world rules, foreshadowing and timeline, and flags issues (info shown too early, setting drift, repeated events, AI-ish phrasing…). Results go into the Consistency report under **📋 Reports**.
> It only gives **suggestions** — it never edits your text. Acting on them is up to you.

### Update memory — don't skip this!
After the chapter is final, click it. The model **extracts** from the text: a chapter summary, events, character status changes, new foreshadowing, relationship changes — and writes them back to the Memory.
> This is what makes the book "more consistent over time". **Skip it and later chapters won't know what happened here.**
>
> Recommended order: **Generate → (edit) → Consistency check → Update memory once you're happy.** Don't update before it's final, or memory records a version you'll change.

> 🔑 **Key point: the ONLY entrance to memory is this button.**
> **Generating, regenerating, and saving text do NOT write to memory.** So feel free to regenerate over and over —
> e.g. the first take is "the auntie whacks the hero with a wok", you regenerate and it becomes "the auntie cooks the hero fried rice".
> As long as you **never clicked "Update memory"**, the overwritten version never entered memory —
> **nothing to clean up, no need to tick "Incl. memory".** Click "Update memory" only when you're satisfied, and only the final version is recorded.

### ☑ Incl. memory + Reset chapter — look before you click 🧨
**Reset chapter** = delete this chapter's text and its reports.

The adjacent **☑ Incl. memory** decides whether to also wipe **this chapter's traces in the ledger**:
- **Unchecked**: only deletes the text and reports, keeping the memory records. Good when you want to **rewrite the text but keep the already-extracted memory**, or when you **haven't run "Update memory" yet** (memory has nothing for this chapter anyway).
- **Checked**: on top of text and reports, also removes this chapter's records from **Chapter Summaries / Events / Timeline**. Good when the chapter is **completely scrapped and rewritten from scratch**, so old traces don't pollute later chapters.

> ⚠️ Important (to avoid misunderstanding): "Incl. memory" only cleans the **summaries / events / timeline** ledgers. It does **not** automatically roll back the changes "Update memory" made to **character status, foreshadowing, or the relationship graph** — those currently need a manual check in 🧠 Memory. So before scrapping a chapter, know what personas it changed.
>
> Reset can't be undone; you'll be asked to confirm once.

---

## 📋 Reports

The **📋 Reports** tab shows three reports per chapter:
- **Generation**: which idea was used, length, model info, raw output.
- **Consistency**: the editor's continuity issues and suggestions.
- **Memory update**: what was written to memory this chapter, plus continuity-risk notes.

---

## 🎯 Recommended rhythm

1. New book: fill **Story Bible + Characters + Book Outline**, drop a few **Style Bank** snippets.
2. In **Settings**, set the **LLM Base URL** (must include `http://`) and model name, save.
3. Workbench: new `ch001` → write a one-line idea → **Generate**.
4. Read it, tweak it, **Save text**.
5. **Consistency check**, read the report, revise.
6. Happy? **Update memory**.
7. New `ch002`, repeat 3–6. Keep going like that. ✨

---

## 🆘 FAQ

**Q: Generate fails with `builder error` / `cannot connect to model service`?**
A: Most likely the **LLM Base URL is missing its scheme**. Make it a full address like `http://127.0.0.1:18180/v1` (your own port). Still failing → your local model service isn't running.

**Q: `HTTP 404`?**
A: Wrong path. Most local servers expose the API under `/v1`; if yours is at the root, drop the trailing `/v1`.

**Q: `JSON parse failed` when updating memory?**
A: The model didn't return valid JSON this time. Retrying usually fixes it; if it persists, try a more obedient model. The failed raw output is saved to the reports folder for inspection. (Output is no longer capped during extraction, so truncation should be rare.)

**Q: Generated chapters are always too short?**
A: Raise `max_output_tokens` in **Settings / `config.yaml`** (set `0` for unlimited), or split a long chapter into two generations.

**Q: Where is my manuscript stored?**
A: Under the **project directory** (full path shown at the bottom of Settings): `chapters/` for text, `memory/` for memory, `outputs/reports/` for reports. Change the **project directory** in Settings to move it.

---

Happy writing. 🖋️ Remember: the model is your co-pilot — your hands are always on the wheel.
