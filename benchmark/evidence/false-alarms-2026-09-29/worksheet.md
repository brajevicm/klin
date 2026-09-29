# Replay worksheet

Every gate that failed or erred in the 100 replay runs of `klin gate --json` with `{}`, one row each.

Label each row `appropriate` or `not-appropriate` in `labels.json`, with an optional note. Judge the row as it stood when it fired.

- Rows: 79
- complexity: 37
- dead-symbols: 8
- doc-size: 16
- escapes: 13
- public-api: 2
- reachability: 1
- stubs: 2
## R001

- Repository: `firecrawl/pdf-inspector` (Rust), change 2 of 10
- Commit: `bf6800920a1d`, judged against its first parent `3400b4725efa`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `8000`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(tounicode): skip a subset font's ToUnicode repairs where the font shows the CMap right (#605)
>
> * fix(tounicode): skip a subset font's ToUnicode repairs where the font shows the CMap right
>
> The repair through a /CIDToGIDMap takes the CMap to be keyed by glyph
> index. It was built for every Identity-H TrueType CID font with such a
> map whose CMap starts above code 2, and the text scoring was left to
> reject it for a CMap that is right as written. It is now skipped when the
> CMap has entries for more of the codes the map sends to other glyphs than
> for those glyphs' indexes: a CMap keyed by code, as the specification has
> it, keeps its reading.
>
> The renumbering of a subset whose width array looks renumbered is skipped
> when the embedded program, where it says what its glyphs are, reads more
> of the codes as the CMap has them than as renumbered. The program is read
> for such a font whether or not the fallbacks read it.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * fix(tounicode): the renumbering check reads a ligature as its letters, over the lowest 1024 codes
>
> A CMap writes a ligature glyph's text as its letters (`fi`) where a
> program names the glyph, or maps it in its cmap, as the ligature (U+FB01).
> The check now compares the two with the ligature spelled out, through the
> program's cmap and its glyph names alike. It reads the program at the
> lowest 1024 codes each CMap maps, which the two readings share, so the
> CMap of a whole large font costs no more to judge than a subset's.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> (26 more lines)

### Files the change touched

```text
 CHANGELOG.md               |  12 +++
 src/lib.rs                 |   9 +-
 src/tounicode.rs           | 395 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++------
 tests/integration_tests.rs | 186 ++++++++++++++++++++++++++++++++++++++++
 4 files changed, 571 insertions(+), 31 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":8000,"words":8111}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R002

- Repository: `firecrawl/pdf-inspector` (Rust), change 2 of 10
- Commit: `bf6800920a1d`, judged against its first parent `3400b4725efa`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 4,332 functions at 3400b47, floor 5; recorded scope: whole repository
- Derived lines: `80`, 95th percentile of 4,332 functions at 3400b47, floor 25; recorded scope: whole repository

### Commit message

> fix(tounicode): skip a subset font's ToUnicode repairs where the font shows the CMap right (#605)
>
> * fix(tounicode): skip a subset font's ToUnicode repairs where the font shows the CMap right
>
> The repair through a /CIDToGIDMap takes the CMap to be keyed by glyph
> index. It was built for every Identity-H TrueType CID font with such a
> map whose CMap starts above code 2, and the text scoring was left to
> reject it for a CMap that is right as written. It is now skipped when the
> CMap has entries for more of the codes the map sends to other glyphs than
> for those glyphs' indexes: a CMap keyed by code, as the specification has
> it, keeps its reading.
>
> The renumbering of a subset whose width array looks renumbered is skipped
> when the embedded program, where it says what its glyphs are, reads more
> of the codes as the CMap has them than as renumbered. The program is read
> for such a font whether or not the fallbacks read it.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * fix(tounicode): the renumbering check reads a ligature as its letters, over the lowest 1024 codes
>
> A CMap writes a ligature glyph's text as its letters (`fi`) where a
> program names the glyph, or maps it in its cmap, as the ligature (U+FB01).
> The check now compares the two with the ligature spelled out, through the
> program's cmap and its glyph names alike. It reads the program at the
> lowest 1024 codes each CMap maps, which the two readings share, so the
> CMap of a whole large font costs no more to judge than a subset's.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> (26 more lines)

### Files the change touched

```text
 CHANGELOG.md               |  12 +++
 src/lib.rs                 |   9 +-
 src/tounicode.rs           | 395 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++------
 tests/integration_tests.rs | 186 ++++++++++++++++++++++++++++++++++++++++
 4 files changed, 571 insertions(+), 31 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 80 lines).

1. `src/tounicode.rs:2074` new, `fn try_remap_subset_cmap<'p>(`, values `{"cc":17,"lines":113}`, ceiling cc 13, lines 80, nothing at the base matched

   ```text
   2071 | /// `program` — the descendant's, read only when a renumbering is in
   2072 | /// question — reads as the CMap has them more often than as renumbered is
   2073 | /// not stale (see [`program_reads_cmap_as_written`]).
   2074 | fn try_remap_subset_cmap<'p>(
   2075 |     cmap: ToUnicodeCMap,
   2076 |     font_dict: &lopdf::Dictionary,
   2077 |     doc: &Document,
   2078 |     obj_num: u32,
   2079 |     program: impl FnOnce() -> Option<&'p [u8]>,
   2080 | ) -> (ToUnicodeCMap, Option<ToUnicodeCMap>) {
   2081 |     // Only applies to Identity-H/V CID fonts
   2082 |     let encoding = font_dict
   2083 |         .get(b"Encoding")
   2084 |         .ok()
   2085 |         .and_then(|o| o.as_name().ok());
   2086 |     if encoding != Some(b"Identity-H") && encoding != Some(b"Identity-V") {
   2087 |         return (cmap, None);
   2088 |     }
   2089 | 
   2090 |     // CMap's minimum source CID must be > 2 (indicating old, non-sequential GIDs)
   2091 |     let min_cid = match cmap.min_source_cid() {
   2092 |         Some(c) if c > 2 => c,
   2093 |         _ => return (cmap, None),
   2094 |     };
   2095 | 
   2096 |     // Navigate to DescendantFonts[0]
   2097 |     let cid_font_dict = match get_descendant_cid_font(font_dict, doc) {
   2098 |         Some(d) => d,
   2099 |         None => return (cmap, None),
   2100 |     };
   2101 | 
   2102 |     // Both repair paths below assume CIDs are glyph indices that a subsetter can
   2103 |     // renumber, which is only true for CIDFontType2 (TrueType). For CIDFontType0
   2104 |     // (CFF), CIDs are resolved through the CFF charset, so a valid CMap stays valid
   2105 |     // after subsetting and renumbering it corrupts otherwise-correct text.
   2106 |     // CIDToGIDMap is likewise CIDFontType2-only (PDF 32000-1:2008, 9.7.4.2), so this
   2107 |     // also ignores a CIDToGIDMap that a malformed producer attached to a CFF font.
   2108 |     // /Subtype may be an indirect reference, so resolve it through the document.
   2109 |     // Only bail out when the descendant is *explicitly* something other than
   2110 |     // CIDFontType2: a missing or unresolvable /Subtype keeps the previous
   2111 |     // behaviour rather than silently disabling the repair.
   2112 |     let subtype = cid_font_dict.get(b"Subtype").ok().and_then(|o| match o {
   2113 |         Object::Reference(r) => doc.get_object(*r).ok().and_then(|o| o.as_name().ok()),
   2114 |         other => other.as_name().ok(),
   ```

2. `src/tounicode.rs:3305` worsened, `fn collect_cmaps_from_fonts_inner(`, values `{"cc":62,"lines":354}`, ceiling cc 13, lines 80, base site `src/tounicode.rs:3115` with `{"cc":62,"lines":350}`

   ```text
   3302 |         Self::collect_cmaps_from_fonts_inner(fonts, doc, by_obj_num, false);
   3303 |     }
   3304 | 
   3305 |     fn collect_cmaps_from_fonts_inner(
   3306 |         fonts: &std::collections::BTreeMap<Vec<u8>, &lopdf::Dictionary>,
   3307 |         doc: &Document,
   3308 |         by_obj_num: &mut HashMap<u32, CMapEntry>,
   3309 |         skip_truetype_fallback: bool,
   3310 |     ) {
   3311 |         // First pass: collect ToUnicode CMaps
   3312 |         for font_dict in fonts.values() {
   3313 |             let obj_ref = match font_dict
   3314 |                 .get(b"ToUnicode")
   3315 |                 .ok()
   3316 |                 .and_then(|o| o.as_reference().ok())
   3317 |             {
   3318 |                 Some(r) => r,
   3319 |                 None => continue,
   3320 |             };
   3321 |             let obj_num = obj_ref.0;
   3322 |             if by_obj_num.contains_key(&obj_num) {
   3323 |                 continue;
   3324 |             }
   3325 |             let stream = match doc.get_object(obj_ref).and_then(Object::as_stream) {
   3326 |                 Ok(s) => s,
   3327 |                 Err(_) => continue,
   3328 |             };
   3329 |             let data = match stream.decompressed_content() {
   3330 |                 Ok(d) => d,
   3331 |                 Err(_) => stream.content.clone(),
   3332 |             };
   3333 |             if let Some(cmap) = ToUnicodeCMap::parse(&data) {
   3334 |                 debug!(
   3335 |                     "CMap obj={:<6} code_byte_length={} char_map={} ranges={}",
   3336 |                     obj_num,
   3337 |                     cmap.code_byte_length,
   3338 |                     cmap.char_map.len(),
   3339 |                     cmap.ranges.len()
   3340 |                 );
   3341 |                 // Only build expensive fallbacks when the primary CMap is sparse.
   3342 |                 // build_fallback_cmap_for_type0 can take seconds on large embedded
   3343 |                 // TrueType fonts (decompressing + parsing 100K+ byte font files).
   3344 |                 // Skip entirely when the primary CMap is sufficient. A sparse
   3345 |                 // CMap takes the cheap fallback first — the CID collection's,
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R003

- Repository: `firecrawl/pdf-inspector` (Rust), change 3 of 10
- Commit: `3400b4725efa`, judged against its first parent `c48b7cacc7b0`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7900`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(extractor): a super- or subscript run is sized by its letters and digits (#603)
>
> A sign set from a symbol font can be a design size above the digits it
> goes with: TeX sets the minus of an exponent that way. Sized by its
> largest glyph, such a run passed the size a script may have, so an
> exponent that reads its minus sign lost its <sup> markup. The run's size
> is now that of its letters and digits. A sign more than a step above
> them (a chain growing a step at a time to the body size) and a run of
> signs alone still size the run by its largest glyph; the step is the one
> a chain allows between neighbouring glyphs, now SCRIPT_RUN_SIZE_STEP.
>
> Co-authored-by: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 CHANGELOG.md             | 11 +++++++++++
 src/extractor/scripts.rs | 68 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++-----
 2 files changed, 74 insertions(+), 5 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7900,"words":7960}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R004

- Repository: `firecrawl/pdf-inspector` (Rust), change 3 of 10
- Commit: `3400b4725efa`, judged against its first parent `c48b7cacc7b0`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 4,331 functions at c48b7ca, floor 5; recorded scope: whole repository
- Derived lines: `80`, 95th percentile of 4,331 functions at c48b7ca, floor 25; recorded scope: whole repository

### Commit message

> fix(extractor): a super- or subscript run is sized by its letters and digits (#603)
>
> A sign set from a symbol font can be a design size above the digits it
> goes with: TeX sets the minus of an exponent that way. Sized by its
> largest glyph, such a run passed the size a script may have, so an
> exponent that reads its minus sign lost its <sup> markup. The run's size
> is now that of its letters and digits. A sign more than a step above
> them (a chain growing a step at a time to the body size) and a run of
> signs alone still size the run by its largest glyph; the step is the one
> a chain allows between neighbouring glyphs, now SCRIPT_RUN_SIZE_STEP.
>
> Co-authored-by: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 CHANGELOG.md             | 11 +++++++++++
 src/extractor/scripts.rs | 68 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++-----
 2 files changed, 74 insertions(+), 5 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 80 lines).

1. `src/extractor/scripts.rs:227` worsened, `fn detect_script_runs(items: &[TextItem]) -> Vec<ScriptRun> {`, values `{"cc":25,"lines":121}`, ceiling cc 13, lines 80, base site `src/extractor/scripts.rs:223` with `{"cc":22,"lines":102}`

   ```text
   224 | }
   225 | 
   226 | /// Find every super/subscript glyph run and its anchor.
   227 | fn detect_script_runs(items: &[TextItem]) -> Vec<ScriptRun> {
   228 |     // Per page: every participant sorted by y (anchor lookup window) and the
   229 |     // glyph-shaped subset sorted by x (run assembly).
   230 |     let mut by_page: HashMap<u32, Vec<usize>> = HashMap::new();
   231 |     for (idx, item) in items.iter().enumerate() {
   232 |         if is_script_participant(item) {
   233 |             by_page.entry(item.page).or_default().push(idx);
   234 |         }
   235 |     }
   236 |     let mut pages: Vec<(u32, Vec<usize>)> = by_page.into_iter().collect();
   237 |     pages.sort_by_key(|(page, _)| *page);
   238 | 
   239 |     let mut runs = Vec::new();
   240 |     for (_, mut by_y) in pages {
   241 |         by_y.sort_by(|&a, &b| items[a].y.total_cmp(&items[b].y));
   242 |         let max_fs = by_y
   243 |             .iter()
   244 |             .map(|&i| items[i].font_size)
   245 |             .fold(0.0_f32, f32::max);
   246 |         let window = max_fs * SCRIPT_MAX_RAISE.max(SCRIPT_MAX_DROP);
   247 | 
   248 |         // Assemble candidate runs: glyph-shaped items on one baseline, of
   249 |         // similar size, touching in x.
   250 |         let mut glyphs: Vec<usize> = by_y
   251 |             .iter()
   252 |             .copied()
   253 |             .filter(|&i| is_script_glyph_text(&items[i].text))
   254 |             .collect();
   255 |         glyphs.sort_by(|&a, &b| {
   256 |             items[a]
   257 |                 .x
   258 |                 .total_cmp(&items[b].x)
   259 |                 .then(items[a].y.total_cmp(&items[b].y))
   260 |         });
   261 |         let mut chains: Vec<Vec<usize>> = Vec::new();
   262 |         for g in glyphs {
   263 |             let glyph = &items[g];
   264 |             let joined = chains.iter_mut().rev().find(|chain| {
   265 |                 let last = &items[*chain.last().unwrap()];
   266 |                 let fs = last.font_size.max(glyph.font_size);
   267 |                 let gap = glyph.x - item_right(last);
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R005

- Repository: `firecrawl/pdf-inspector` (Rust), change 5 of 10
- Commit: `ac275044ed04`, judged against its first parent `e1797b1f482a`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7750`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(extractor): compare a short string's readings without its short common words (#597)
>
> * fix(extractor): a string of one or two glyphs needs more than a short word to take a repaired CMap
>
> A subset font whose ToUnicode CMap has a repaired counterpart chooses
> between the two over its first strings, and each string before that choice
> reads through whichever scores better. The score counts common words, and a
> string of one or two glyphs is too short for them to be evidence: a wrong
> repair reading a glyph shown on its own as `a`, or a pair as `aT`, took the
> repair over the CMap's right letters. Such a string now takes the repair
> only on better characters or more common words of three letters or more.
> Longer strings, and the font's choice, weigh every common word as before.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * fix(extractor): compare a short string's readings without its short words
>
> A string of one or two glyphs, before the font's choice between its
> ToUnicode CMap and the repaired one, is too short for its short common
> words to be evidence, and a wrong repair spells them by chance: a glyph
> read as `a` where the CMap reads a letter, a dash or a space took the
> repair. The two readings of such a string are now compared without the
> points of those words, so the repair must win on the rest: a letter over a
> replacement character or a control, two letters over two symbols. The
> evidence helper goes, and a test covers a lone glyph read as a control and
> pins the short words a genuine repair now waits on.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * refactor(extractor): the short-word discount lives in TextScore; the program's own reading keeps every common word, and says why
> (6 more lines)

### Files the change touched

```text
 CHANGELOG.md               |  11 +++
 src/extractor/fonts.rs     | 106 +++++++++++++++++++++----
 tests/integration_tests.rs | 358 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 3 files changed, 460 insertions(+), 15 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7750,"words":7865}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R006

- Repository: `firecrawl/pdf-inspector` (Rust), change 5 of 10
- Commit: `ac275044ed04`, judged against its first parent `e1797b1f482a`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 4,317 functions at e1797b1, floor 5; recorded scope: whole repository
- Derived lines: `81`, 95th percentile of 4,317 functions at e1797b1, floor 25; recorded scope: whole repository

### Commit message

> fix(extractor): compare a short string's readings without its short common words (#597)
>
> * fix(extractor): a string of one or two glyphs needs more than a short word to take a repaired CMap
>
> A subset font whose ToUnicode CMap has a repaired counterpart chooses
> between the two over its first strings, and each string before that choice
> reads through whichever scores better. The score counts common words, and a
> string of one or two glyphs is too short for them to be evidence: a wrong
> repair reading a glyph shown on its own as `a`, or a pair as `aT`, took the
> repair over the CMap's right letters. Such a string now takes the repair
> only on better characters or more common words of three letters or more.
> Longer strings, and the font's choice, weigh every common word as before.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * fix(extractor): compare a short string's readings without its short words
>
> A string of one or two glyphs, before the font's choice between its
> ToUnicode CMap and the repaired one, is too short for its short common
> words to be evidence, and a wrong repair spells them by chance: a glyph
> read as `a` where the CMap reads a letter, a dash or a space took the
> repair. The two readings of such a string are now compared without the
> points of those words, so the repair must win on the rest: a letter over a
> replacement character or a control, two letters over two symbols. The
> evidence helper goes, and a test covers a lone glyph read as a control and
> pins the short words a genuine repair now waits on.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * refactor(extractor): the short-word discount lives in TextScore; the program's own reading keeps every common word, and says why
> (6 more lines)

### Files the change touched

```text
 CHANGELOG.md               |  11 +++
 src/extractor/fonts.rs     | 106 +++++++++++++++++++++----
 tests/integration_tests.rs | 358 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 3 files changed, 460 insertions(+), 15 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 81 lines).

1. `src/extractor/fonts.rs:3342` new, `fn text_score(text: &str) -> TextScore {`, values `{"cc":16,"lines":61}`, ceiling cc 13, lines 81, nothing at the base matched

   ```text
   3339 |     }
   3340 | }
   3341 | 
   3342 | fn text_score(text: &str) -> TextScore {
   3343 |     const COMMON_WORDS: [&str; 22] = [
   3344 |         "the", "and", "of", "to", "in", "a", "is", "that", "for", "with", "on", "as", "by", "from",
   3345 |         "this", "be", "are", "at", "or", "not", "it", "our",
   3346 |     ];
   3347 | 
   3348 |     let mut letters = 0i32;
   3349 |     let mut spaces = 0i32;
   3350 |     let mut digits = 0i32;
   3351 |     let mut other = 0i32;
   3352 |     let mut short_words = 0i32;
   3353 |     let mut long_words = 0i32;
   3354 |     let mut count_word = |word: &str| {
   3355 |         if COMMON_WORDS.contains(&word) {
   3356 |             if word.len() < 3 {
   3357 |                 short_words += 1;
   3358 |             } else {
   3359 |                 long_words += 1;
   3360 |             }
   3361 |         }
   3362 |     };
   3363 | 
   3364 |     let mut current = String::new();
   3365 |     for ch in text.chars() {
   3366 |         if ch.is_ascii_alphabetic() {
   3367 |             letters += 1;
   3368 |             current.push(ch.to_ascii_lowercase());
   3369 |         } else {
   3370 |             if !current.is_empty() {
   3371 |                 count_word(&current);
   3372 |                 current.clear();
   3373 |             }
   3374 |             if ch == ' ' {
   3375 |                 spaces += 1;
   3376 |             } else if ch.is_ascii_digit() {
   3377 |                 digits += 1;
   3378 |             } else if ch.is_control() || ch == '\u{FFFD}' {
   3379 |                 other += 3;
   3380 |             } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch)
   3381 |                 || ('\u{3040}'..='\u{309F}').contains(&ch)
   3382 |                 || ('\u{30A0}'..='\u{30FF}').contains(&ch)
   ```

2. `src/extractor/fonts.rs:2519` worsened, `pub(crate) fn extract_text_from_operand(`, values `{"cc":111,"lines":567}`, ceiling cc 13, lines 81, base site `src/extractor/fonts.rs:2519` with `{"cc":111,"lines":561}`

   ```text
   2516 | 
   2517 | /// Decode a PDF string and record whether legacy symbol cleanup changed a character.
   2518 | #[allow(clippy::too_many_arguments)]
   2519 | pub(crate) fn extract_text_from_operand(
   2520 |     obj: &Object,
   2521 |     current_font: &str,
   2522 |     base_font_name: Option<&str>,
   2523 |     font_cmaps: &FontCMaps,
   2524 |     font_tounicode_refs: &std::collections::HashMap<String, u32>,
   2525 |     inline_cmaps: &std::collections::HashMap<String, crate::tounicode::CMapEntry>,
   2526 |     font_encodings: &PageFontEncodings,
   2527 |     encoding_cache: &HashMap<String, Encoding<'_>>,
   2528 |     cmap_decisions: &mut CMapDecisionCache,
   2529 |     font_widths: &PageFontWidths,
   2530 |     font_kinds: &PageFontKinds,
   2531 | ) -> Option<(String, bool)> {
   2532 |     let is_type0_cid_font = font_widths
   2533 |         .get(current_font)
   2534 |         .is_some_and(|info| info.is_cid);
   2535 |     // A simple font (any font but Type0) shows one byte per code
   2536 |     // (PDF 32000-1:2008, 9.6), whatever byte width its ToUnicode CMap
   2537 |     // declares: a codespace written as `<0000> <FFFF>` over one-byte entries
   2538 |     // must not pair the bytes of a string into codes. The font's subtype
   2539 |     // decides, as it does for the detector; a font whose subtype is not a
   2540 |     // name keeps the CMap's reading there too.
   2541 |     let is_simple_font = font_kinds
   2542 |         .get(current_font)
   2543 |         .is_some_and(|&composite| !composite);
   2544 |     let use_cp1252_fallback =
   2545 |         should_use_cp1252_single_byte_fallback(base_font_name, is_type0_cid_font);
   2546 |     // The name the font's CMap coverage is counted under: the `/BaseFont`
   2547 |     // name, or the resource name when the font has none or an empty one.
   2548 |     let font_label = base_font_name
   2549 |         .filter(|name| !name.is_empty())
   2550 |         .unwrap_or(current_font);
   2551 |     let result = (|| -> Option<String> {
   2552 |         if let Object::String(bytes, _) = obj {
   2553 |             let mut decode_with_entry = |entry: &crate::tounicode::CMapEntry| -> Option<String> {
   2554 |                 // For single-byte CMaps, and for any CMap of a simple font,
   2555 |                 // merge CMap + Differences at the byte level: try CMap first,
   2556 |                 // then Differences, then Latin-1 fallback per byte. This
   2557 |                 // prevents partial CMap results from blocking the Differences
   2558 |                 // path.
   2559 |                 if entry.primary.code_byte_length == 1 || is_simple_font {
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R007

- Repository: `firecrawl/pdf-inspector` (Rust), change 6 of 10
- Commit: `e1797b1f482a`, judged against its first parent `9a055d5f3ccd`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7500`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(extractor): Type1 fonts read through their program's built-in encoding (#596)

### Files the change touched

```text
 CHANGELOG.md               |  21 +++++
 src/extractor/fonts.rs     | 326 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/extractor/mod.rs       |   1 +
 src/extractor/type1.rs     | 417 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/types.rs               |  22 +++--
 tests/integration_tests.rs | 315 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 6 files changed, 1090 insertions(+), 12 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7500,"words":7740}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R008

- Repository: `firecrawl/pdf-inspector` (Rust), change 6 of 10
- Commit: `e1797b1f482a`, judged against its first parent `9a055d5f3ccd`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 4,288 functions at 9a055d5, floor 5; recorded scope: whole repository
- Derived lines: `80`, 95th percentile of 4,288 functions at 9a055d5, floor 25; recorded scope: whole repository

### Commit message

> fix(extractor): Type1 fonts read through their program's built-in encoding (#596)

### Files the change touched

```text
 CHANGELOG.md               |  21 +++++
 src/extractor/fonts.rs     | 326 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/extractor/mod.rs       |   1 +
 src/extractor/type1.rs     | 417 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/types.rs               |  22 +++--
 tests/integration_tests.rs | 315 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 6 files changed, 1090 insertions(+), 12 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 80 lines).

1. `src/extractor/fonts.rs:1198` new, `fn type1_program_under_no_named_base(`, values `{"cc":14,"lines":20}`, ceiling cc 13, lines 80, nothing at the base matched

   ```text
   1195 | 
   1196 | /// The embedded program (`/FontFile`) of a Type 1 font whose `/Encoding`
   1197 | /// names no base encoding: absent, or a dictionary without `/BaseEncoding`.
   1198 | fn type1_program_under_no_named_base(
   1199 |     doc: &Document,
   1200 |     font_dict: &lopdf::Dictionary,
   1201 | ) -> Option<ObjectId> {
   1202 |     let subtype = font_dict.get(b"Subtype").ok()?.as_name().ok()?;
   1203 |     if subtype != b"Type1" && subtype != b"MMType1" {
   1204 |         return None;
   1205 |     }
   1206 |     if let Ok(encoding) = font_dict.get(b"Encoding") {
   1207 |         let encoding = match encoding {
   1208 |             Object::Reference(id) => doc.get_object(*id).ok()?,
   1209 |             other => other,
   1210 |         };
   1211 |         if encoding.as_dict().ok()?.has(b"BaseEncoding") {
   1212 |             return None;
   1213 |         }
   1214 |     }
   1215 |     let descriptor = resolve_dict(doc, font_dict.get(b"FontDescriptor").ok()?)?;
   1216 |     descriptor.get(b"FontFile").ok()?.as_reference().ok()
   1217 | }
   1218 | 
   ```

2. `src/extractor/type1.rs:25` new, `pub(crate) fn builtin_encoding(program: &[u8]) -> Option<BuiltinEncoding> {`, values `{"cc":41,"lines":83}`, ceiling cc 13, lines 80, nothing at the base matched

   ```text
   22 | /// to `readonly def`. `None` for anything else — no cleartext part, no
   23 | /// `/Encoding`, an entry in another form, a code outside a byte — so the
   24 | /// font reads as it would without one.
   25 | pub(crate) fn builtin_encoding(program: &[u8]) -> Option<BuiltinEncoding> {
   26 |     let mut tokens = Tokens {
   27 |         rest: cleartext(program)?,
   28 |     };
   29 |     // The font dictionary's own entry: at the level of the dictionary the
   30 |     // program begins, not in one nested in it (`/FontInfo … begin … end`).
   31 |     let mut depth = 0i32;
   32 |     loop {
   33 |         match tokens.next()? {
   34 |             Token::Word(b"begin") | Token::Open(b'<') => depth += 1,
   35 |             Token::Word(b"end") | Token::Close(b'>') => depth -= 1,
   36 |             Token::Name(b"Encoding") if depth == 1 => break,
   37 |             _ => {}
   38 |         }
   39 |     }
   40 |     match tokens.next()? {
   41 |         Token::Word(b"StandardEncoding") => {
   42 |             return ends_definition(&mut tokens).then_some(BuiltinEncoding::Standard);
   43 |         }
   44 |         Token::Word(size) if integer(size).is_some_and(|n| (1..=256).contains(&n)) => {}
   45 |         _ => return None,
   46 |     }
   47 |     if tokens.next()? != Token::Word(b"array") {
   48 |         return None;
   49 |     }
   50 |     let mut names: Vec<Option<String>> = vec![None; 256];
   51 |     loop {
   52 |         match tokens.next()? {
   53 |             Token::Word(b"dup") => {
   54 |                 let Token::Word(code) = tokens.next()? else {
   55 |                     return None;
   56 |                 };
   57 |                 let code = u8::try_from(integer(code)?).ok()?;
   58 |                 let Token::Name(name) = tokens.next()? else {
   59 |                     return None;
   60 |                 };
   61 |                 if tokens.next()? != Token::Word(b"put") {
   62 |                     return None;
   63 |                 }
   64 |                 names[usize::from(code)] =
   65 |                     (name != b".notdef").then(|| String::from_utf8_lossy(name).into_owned());
   ```

3. `src/extractor/type1.rs:167` new, `fn next(&mut self) -> Option<Token<'a>> {`, values `{"cc":20,"lines":68}`, ceiling cc 13, lines 80, nothing at the base matched

   ```text
   164 | impl<'a> Iterator for Tokens<'a> {
   165 |     type Item = Token<'a>;
   166 | 
   167 |     fn next(&mut self) -> Option<Token<'a>> {
   168 |         loop {
   169 |             let (&first, tail) = self.rest.split_first()?;
   170 |             match first {
   171 |                 b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0' => self.rest = tail,
   172 |                 b'%' => {
   173 |                     let end = tail
   174 |                         .iter()
   175 |                         .position(|&b| b == b'\n' || b == b'\r')
   176 |                         .unwrap_or(tail.len());
   177 |                     self.rest = &tail[end..];
   178 |                 }
   179 |                 _ => break,
   180 |             }
   181 |         }
   182 |         let (&first, tail) = self.rest.split_first()?;
   183 |         let token = match first {
   184 |             b'(' => {
   185 |                 // A literal string: balanced parentheses, backslash escapes.
   186 |                 let mut depth = 1;
   187 |                 let mut i = 0;
   188 |                 while depth > 0 {
   189 |                     match tail.get(i)? {
   190 |                         b'\\' => i += 1,
   191 |                         b'(' => depth += 1,
   192 |                         b')' => depth -= 1,
   193 |                         _ => {}
   194 |                     }
   195 |                     i += 1;
   196 |                 }
   197 |                 self.rest = &tail[i..];
   198 |                 Token::Str
   199 |             }
   200 |             b'<' if tail.first() == Some(&b'<') => {
   201 |                 self.rest = &tail[1..];
   202 |                 Token::Open(b'<')
   203 |             }
   204 |             b'>' if tail.first() == Some(&b'>') => {
   205 |                 self.rest = &tail[1..];
   206 |                 Token::Close(b'>')
   207 |             }
   ```

4. `tests/integration_tests.rs:10390` new, `fn test_type1_programs_encoding_yields_to_the_fonts_own_readings() {`, values `{"cc":3,"lines":103}`, ceiling cc 13, lines 80, nothing at the base matched
5. `src/extractor/fonts.rs:392` worsened, `fn base14_fallback_widths(`, values `{"cc":9,"lines":114}`, ceiling cc 13, lines 80, base site `src/extractor/fonts.rs:391` with `{"cc":8,"lines":95}`
6. `src/extractor/fonts.rs:900` worsened, `pub(crate) fn build_font_encodings(`, values `{"cc":12,"lines":106}`, ceiling cc 13, lines 80, base site `src/extractor/fonts.rs:880` with `{"cc":16,"lines":98}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R009

- Repository: `firecrawl/pdf-inspector` (Rust), change 7 of 10
- Commit: `9a055d5f3ccd`, judged against its first parent `2dbd16b3ea4e`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7400`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(extractor): simple fonts read one byte per code under a two-byte ToUnicode codespace (#595)
>
> * fix(extractor): simple fonts read one byte per code under a two-byte ToUnicode codespace
>
> A simple font's codes are one byte (PDF 32000-1:2008, 9.6). A ToUnicode
> CMap declaring `<0000> <FFFF>` over one-byte entries, with one entry
> spelled in four hex digits, stayed two bytes wide, so every even-length
> string of the font was read as two-byte codes the CMap has no entry for
> and lost its text. Such a font's strings now go through the byte-level
> reading; composite fonts keep their CMap's code width.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * test(extractor): decoder tests pass the font kinds of the fonts they describe
>
> The unit tests that call the decoder passed an empty font-kind map, which
> no page builds for a font whose subtype reads: each now passes the kind of
> the font it describes, the page-font helper builds the map from the page's
> fonts, and the two-byte CMap test on a simple font covers a font whose
> subtype cannot be read too.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> ---------
>
> Co-authored-by: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 CHANGELOG.md                      |  15 +++++++++
 src/detector.rs                   |  48 +++++++++++++++++++++++++---
 src/extractor/content_stream.rs   |  14 +++++++--
 src/extractor/fonts.rs            | 109 ++++++++++++++++++++++++++++++++++++++++++++++++---------------
 src/extractor/stale_cmap_tests.rs |  12 +++++++
 src/extractor/xobjects.rs         |  12 +++++--
 src/types.rs                      |   6 ++++
 tests/integration_tests.rs        | 134 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 8 files changed, 312 insertions(+), 38 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7400,"words":7492}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R010

- Repository: `firecrawl/pdf-inspector` (Rust), change 7 of 10
- Commit: `9a055d5f3ccd`, judged against its first parent `2dbd16b3ea4e`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 4,280 functions at 2dbd16b, floor 5; recorded scope: whole repository
- Derived lines: `80`, 95th percentile of 4,280 functions at 2dbd16b, floor 25; recorded scope: whole repository

### Commit message

> fix(extractor): simple fonts read one byte per code under a two-byte ToUnicode codespace (#595)
>
> * fix(extractor): simple fonts read one byte per code under a two-byte ToUnicode codespace
>
> A simple font's codes are one byte (PDF 32000-1:2008, 9.6). A ToUnicode
> CMap declaring `<0000> <FFFF>` over one-byte entries, with one entry
> spelled in four hex digits, stayed two bytes wide, so every even-length
> string of the font was read as two-byte codes the CMap has no entry for
> and lost its text. Such a font's strings now go through the byte-level
> reading; composite fonts keep their CMap's code width.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> * test(extractor): decoder tests pass the font kinds of the fonts they describe
>
> The unit tests that call the decoder passed an empty font-kind map, which
> no page builds for a font whose subtype reads: each now passes the kind of
> the font it describes, the page-font helper builds the map from the page's
> fonts, and the two-byte CMap test on a simple font covers a font whose
> subtype cannot be read too.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
>
> ---------
>
> Co-authored-by: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 CHANGELOG.md                      |  15 +++++++++
 src/detector.rs                   |  48 +++++++++++++++++++++++++---
 src/extractor/content_stream.rs   |  14 +++++++--
 src/extractor/fonts.rs            | 109 ++++++++++++++++++++++++++++++++++++++++++++++++---------------
 src/extractor/stale_cmap_tests.rs |  12 +++++++
 src/extractor/xobjects.rs         |  12 +++++--
 src/types.rs                      |   6 ++++
 tests/integration_tests.rs        | 134 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 8 files changed, 312 insertions(+), 38 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 80 lines).

1. `src/extractor/content_stream.rs:438` worsened, `pub(crate) fn extract_page_text_items_with_options(`, values `{"cc":301,"lines":2297}`, ceiling cc 13, lines 80, base site `src/extractor/content_stream.rs:438` with `{"cc":301,"lines":2289}`

   ```text
   435 | /// of them the CMap had no entry for, with the run's geometry in the
   436 | /// items' frame so a caller that leaves runs out can leave their codes
   437 | /// out too.
   438 | pub(crate) fn extract_page_text_items_with_options(
   439 |     doc: &Document,
   440 |     page_id: ObjectId,
   441 |     page_num: u32,
   442 |     font_cmaps: &FontCMaps,
   443 |     options: TextExtractionOptions,
   444 |     style_cache: &mut FontStyleCache,
   445 |     form_budget: &mut FormWalkBudget,
   446 | ) -> Result<(PageExtraction, bool, PageRotation, bool, Vec<RunCoverage>), PdfError> {
   447 |     let include_invisible = options.include_invisible;
   448 |     let mut items = Vec::new();
   449 |     let mut rects: Vec<PdfRect> = Vec::new();
   450 |     let mut clip_rects: Vec<PdfRect> = Vec::new();
   451 |     let mut lines: Vec<PdfLine> = Vec::new();
   452 |     let mut underline_lines: Vec<UnderlineLine> = Vec::new();
   453 |     // Indexes of items whose raw decoded text is a multi-character RTL run
   454 |     // that may be stored in visual order (see fix_visual_order_rtl), plus a
   455 |     // count of show ops whose glyph progression walked right-to-left —
   456 |     // evidence of logical-order storage.
   457 |     let mut rtl_visual_candidates: Vec<usize> = Vec::new();
   458 |     let mut rtl_logical_runs: Vec<usize> = Vec::new();
   459 |     let mut rtl_visual_runs: Vec<usize> = Vec::new();
   460 |     // Items whose text is logical whatever the page's storage order:
   461 |     // ActualText replacements.
   462 |     let mut logical_text_items: Vec<usize> = Vec::new();
   463 | 
   464 |     // Path construction state for m/l/h → S/s line extraction
   465 |     let mut path_subpath_start: Option<(f32, f32)> = None;
   466 |     let mut path_current: Option<(f32, f32)> = None;
   467 |     let mut pending_lines: Vec<(f32, f32, f32, f32)> = Vec::new();
   468 |     // Completed subpaths (each a vec of line segments) for f/f* rect extraction
   469 |     let mut pending_subpaths: Vec<Vec<(f32, f32, f32, f32)>> = Vec::new();
   470 |     let mut fill_rects: Vec<PdfRect> = Vec::new();
   471 |     // `re` rects awaiting a paint operator. Underline detection must only
   472 |     // see painted rects: a `re W n` clip path or `re n` no-op draws nothing
   473 |     // on the page, so treating every `re` as ink would underline text that
   474 |     // merely sits near an invisible clip boundary.
   475 |     let mut pending_re_rects: Vec<PdfRect> = Vec::new();
   476 |     let mut painted_rects: Vec<PdfRect> = Vec::new();
   477 | 
   478 |     // Get fonts for encoding
   ```

2. `src/extractor/fonts.rs:1442` worsened, `fn stale_identity_cmap_overrides(`, values `{"cc":48,"lines":137}`, ceiling cc 13, lines 80, base site `src/extractor/fonts.rs:1425` with `{"cc":49,"lines":134}`

   ```text
   1439 | /// A font whose CMap agrees with its Differences has no such slot and is
   1440 | /// left alone. Repairs are kept per font, since different encodings can
   1441 | /// share one CMap stream.
   1442 | fn stale_identity_cmap_overrides(
   1443 |     doc: &Document,
   1444 |     font_dict: &lopdf::Dictionary,
   1445 |     cmaps: &FontCMaps,
   1446 |     encoding: &EncodingResult,
   1447 | ) -> HashMap<u8, String> {
   1448 |     let verified = || -> Option<HashMap<u8, String>> {
   1449 |         if font_dict.get(b"Subtype").ok()?.as_name().ok()? != b"Type1" {
   1450 |             return None;
   1451 |         }
   1452 |         let cmap_ref = font_dict.get(b"ToUnicode").ok()?.as_reference().ok()?;
   1453 |         let entry = cmaps.get_by_obj(cmap_ref.0)?;
   1454 |         // A Type1 font's strings are read one byte per code whatever width
   1455 |         // its CMap declares (see `extract_text_from_operand`), so a CMap
   1456 |         // written as two bytes wide over one-byte entries is judged too.
   1457 |         if entry.remapped.is_some() {
   1458 |             return None;
   1459 |         }
   1460 |         let mapped = |code: u8| {
   1461 |             entry
   1462 |                 .primary
   1463 |                 .lookup(u16::from(code))
   1464 |                 .filter(|text| !text.is_empty() && !text.contains('\u{FFFD}'))
   1465 |         };
   1466 |         let mapped_codes: Vec<u8> = (0..=255u8).filter(|&code| mapped(code).is_some()).collect();
   1467 |         let elsewhere: std::collections::HashSet<String> = mapped_codes
   1468 |             .iter()
   1469 |             .filter_map(|&code| mapped(code))
   1470 |             .collect();
   1471 |         // The named ASCII slots the old CMap describes as their own
   1472 |         // occupant, and what each of those reads as by its name where the
   1473 |         // name says otherwise.
   1474 |         let mut described = 0usize;
   1475 |         let mut repairs: HashMap<u8, String> = HashMap::new();
   1476 |         for &code in &encoding.named_codes {
   1477 |             if !code.is_ascii_graphic() {
   1478 |                 continue;
   1479 |             }
   1480 |             let Some(old) = mapped(code) else {
   1481 |                 continue;
   1482 |             };
   ```

3. `src/extractor/fonts.rs:2308` worsened, `pub(crate) fn extract_text_from_operand(`, values `{"cc":111,"lines":561}`, ceiling cc 13, lines 80, base site `src/extractor/fonts.rs:2288` with `{"cc":110,"lines":549}`

   ```text
   2305 | 
   2306 | /// Decode a PDF string and record whether legacy symbol cleanup changed a character.
   2307 | #[allow(clippy::too_many_arguments)]
   2308 | pub(crate) fn extract_text_from_operand(
   2309 |     obj: &Object,
   2310 |     current_font: &str,
   2311 |     base_font_name: Option<&str>,
   2312 |     font_cmaps: &FontCMaps,
   2313 |     font_tounicode_refs: &std::collections::HashMap<String, u32>,
   2314 |     inline_cmaps: &std::collections::HashMap<String, crate::tounicode::CMapEntry>,
   2315 |     font_encodings: &PageFontEncodings,
   2316 |     encoding_cache: &HashMap<String, Encoding<'_>>,
   2317 |     cmap_decisions: &mut CMapDecisionCache,
   2318 |     font_widths: &PageFontWidths,
   2319 |     font_kinds: &PageFontKinds,
   2320 | ) -> Option<(String, bool)> {
   2321 |     let is_type0_cid_font = font_widths
   2322 |         .get(current_font)
   2323 |         .is_some_and(|info| info.is_cid);
   2324 |     // A simple font (any font but Type0) shows one byte per code
   2325 |     // (PDF 32000-1:2008, 9.6), whatever byte width its ToUnicode CMap
   2326 |     // declares: a codespace written as `<0000> <FFFF>` over one-byte entries
   2327 |     // must not pair the bytes of a string into codes. The font's subtype
   2328 |     // decides, as it does for the detector; a font whose subtype is not a
   2329 |     // name keeps the CMap's reading there too.
   2330 |     let is_simple_font = font_kinds
   2331 |         .get(current_font)
   2332 |         .is_some_and(|&composite| !composite);
   2333 |     let use_cp1252_fallback =
   2334 |         should_use_cp1252_single_byte_fallback(base_font_name, is_type0_cid_font);
   2335 |     // The name the font's CMap coverage is counted under: the `/BaseFont`
   2336 |     // name, or the resource name when the font has none or an empty one.
   2337 |     let font_label = base_font_name
   2338 |         .filter(|name| !name.is_empty())
   2339 |         .unwrap_or(current_font);
   2340 |     let result = (|| -> Option<String> {
   2341 |         if let Object::String(bytes, _) = obj {
   2342 |             let mut decode_with_entry = |entry: &crate::tounicode::CMapEntry| -> Option<String> {
   2343 |                 // For single-byte CMaps, and for any CMap of a simple font,
   2344 |                 // merge CMap + Differences at the byte level: try CMap first,
   2345 |                 // then Differences, then Latin-1 fallback per byte. This
   2346 |                 // prevents partial CMap results from blocking the Differences
   2347 |                 // path.
   2348 |                 if entry.primary.code_byte_length == 1 || is_simple_font {
   ```

4. `src/extractor/fonts.rs:5366` worsened, `fn an_odd_length_string_reads_a_control_destination_through_the_other_cmaps() {`, values `{"cc":3,"lines":110}`, ceiling cc 13, lines 80, base site `src/extractor/fonts.rs:5311` with `{"cc":3,"lines":109}`
5. `src/extractor/xobjects.rs:292` worsened, `fn extract_form_xobject_text_inner(`, values `{"cc":180,"lines":1211}`, ceiling cc 13, lines 80, base site `src/extractor/xobjects.rs:292` with `{"cc":180,"lines":1205}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R011

- Repository: `firecrawl/pdf-inspector` (Rust), change 8 of 10
- Commit: `2dbd16b3ea4e`, judged against its first parent `f06bb3407551`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7300`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> chore(release): bump package versions to 1.25.0 (#593)
>
> Bumps every package from 1.24.0 to 1.25.0 with scripts/version.py and dates
> the changelog's Unreleased section, adding each entry's pull request link
> and an entry for the linux-x64-gnu glibc floor fix (#586), which merged
> after 1.24.0.
>
> Co-authored-by: Claude Opus 5.5 (1M context) <noreply@anthropic.com>

### Files the change touched

```text
 CHANGELOG.md      | 13 ++++++++++++-
 Cargo.toml        |  2 +-
 napi/Cargo.lock   |  4 ++--
 napi/Cargo.toml   |  2 +-
 napi/bun.lock     | 12 ++++++------
 napi/package.json | 14 +++++++-------
 pyproject.toml    |  2 +-
 site/index.html   |  2 +-
 wasm/Cargo.lock   |  4 ++--
 wasm/Cargo.toml   |  2 +-
 10 files changed, 34 insertions(+), 23 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7300,"words":7368}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R012

- Repository: `firecrawl/pdf-inspector` (Rust), change 9 of 10
- Commit: `f06bb3407551`, judged against its first parent `f856d3481d41`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `7150`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `600`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1800`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `200`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(extractor): underline detection no longer goes quadratic on pages drawn from thin rects; add extractTextWithPositionsAsync (#592)
>
> * fix(extractor): classify only rules that can decorate text in underline detection
>
> Every thin filled rect and short horizontal stroke on a page is an
> underline rule candidate, and discard_repeated_ruling_rules compared each
> candidate with every other one (the segmented-row and repeated-span
> checks) before anything looked at the text. On a page of vector art drawn
> from about 200,000 thin rects with 25 text items that pass alone took
> about 40 s.
>
> Only a rule in the underline window or strike band of some text item can
> change the result: every later use of the kept rules (tabular rows,
> fraction bars, strikeouts, underlines) tests them against the page's
> items, never against each other. Those rules are classified first; every
> rule is still the context the repetition checks compare against, so a
> rule near text is judged exactly as before. The page now takes under
> 0.5 s. extract_text_with_positions output is byte-identical to before on
> the fixtures, the pdf-evals corpus and four large production PDFs
> (1.62M items, including 18k underline and 147 strikeout marks), and
> pdf-evals `bench.py test` passes 196/196 at 100%.
>
> Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
>
> * feat(napi): extractTextWithPositionsAsync
>
> The async variant of extractTextWithPositions, built like
> extractPagesMarkdownAsync: the extraction runs on the libuv thread pool
> and the call returns a promise, so a slow page no longer blocks the
> caller's event loop. Arguments and results are the same as the sync
> (13 more lines)

### Files the change touched

```text
 CHANGELOG.md               | 23 +++++++++++++++++++++++
 napi/README.md             | 11 ++++++++---
 napi/src/lib.rs            | 68 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--------
 napi/test.mjs              | 18 ++++++++++++++++++
 src/extractor/underline.rs | 38 ++++++++++++++++++++++++++++++++++++++
 5 files changed, 147 insertions(+), 11 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":7150,"words":7294}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R013

- Repository: `gfx-rs/wgpu` (Rust), change 5 of 10
- Commit: `fcd059f42329`, judged against its first parent `5a7601baf4ec`, exit 1
- Gate: public-api, FAIL
- Decision group: public-api

### Commit message

> Move `MapMode` into wgpu-types and use it instead of `HostMap` in wgpu-core (#10474)
>
> Signed-off-by: sagudev <16504129+sagudev@users.noreply.github.com>

### Files the change touched

```text
 deno_webgpu/buffer.rs            |  4 ++--
 deno_webgpu/device.rs            |  2 +-
 player/tests/player/main.rs      |  2 +-
 wgpu-core/src/device/mod.rs      | 18 +++++-------------
 wgpu-core/src/device/resource.rs |  8 ++++----
 wgpu-core/src/resource.rs        | 22 +++++++++++-----------
 wgpu-types/src/buffer.rs         | 12 ++++++++++++
 wgpu/src/api/buffer.rs           | 10 ----------
 wgpu/src/backend/wgpu_core.rs    |  7 ++-----
 wgpu/src/lib.rs                  |  2 +-
 10 files changed, 39 insertions(+), 48 deletions(-)
```

### Findings

Condition: where an external surface or item the base exposed is gone or its declared contract changed.

1. `wgpu:937` new, `MapMode (type)`, values `{"break":1,"kind":"removed","origin":"wgpu/src/api/buffer.rs:937"}`, nothing at the base matched
2. `wgpu_core:46` new, `device::HostMap (type)`, values `{"break":1,"kind":"removed","origin":"wgpu-core/src/device/mod.rs:46"}`, nothing at the base matched
3. `wgpu_core:275` new, `resource::BufferMapOperation (type)`, values `{"break":1,"kind":"changed","now":"struct BufferMapOperation { mode: MapMode, callback: Option<BufferMapCallback> }","origin":"wgpu-core/src/resource.rs:275","was":"struct BufferMapOperation { host: HostMap, callback: Option<BufferMapCallback> }"}`, nothing at the base matched

### Remedy klin printed

> Keep the surface, the item or the declared contract the base had where the task allows it. For a changed contract, a new item beside the unchanged one keeps the base's contract where that serves the task. Do not change what the task asked for only to satisfy this gate. If the break is intended, a person accepts it with an accepted entry in a reviewed commit, and until then CI refuses it.

## R014

- Repository: `gfx-rs/wgpu` (Rust), change 6 of 10
- Commit: `5a7601baf4ec`, judged against its first parent `2c3e503258dd`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived AGENTS.md: `1150`, the word count at the derivation commit, rounded up to the next 50
- Derived CHANGELOG.md: `42500`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `1150`, the word count at the derivation commit, rounded up to the next 50
- Derived CODE_OF_CONDUCT.md: `800`, the word count at the derivation commit, rounded up to the next 50
- Derived CONTRIBUTING.md: `1750`, the word count at the derivation commit, rounded up to the next 50
- Derived GOVERNANCE.md: `350`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `1150`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `550`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> Reject unsigned inputs to sign (#10451)
>
> Co-authored-by: Ferdon <notferdon@users.noreply.github.com>

### Files the change touched

```text
 CHANGELOG.md                            |  1 +
 naga/src/proc/overloads/mathfunction.rs |  3 ++-
 naga/src/proc/overloads/scalar_set.rs   |  4 ++++
 naga/tests/naga/validation.rs           | 70 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 naga/tests/naga/wgsl_errors.rs          | 55 +++++++++++++++++++++++++++++++++++++++++++++++++++++++
 5 files changed, 132 insertions(+), 1 deletion(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":42500,"words":42516}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R015

- Repository: `gfx-rs/wgpu` (Rust), change 6 of 10
- Commit: `5a7601baf4ec`, judged against its first parent `2c3e503258dd`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Reject unsigned inputs to sign (#10451)
>
> Co-authored-by: Ferdon <notferdon@users.noreply.github.com>

### Files the change touched

```text
 CHANGELOG.md                            |  1 +
 naga/src/proc/overloads/mathfunction.rs |  3 ++-
 naga/src/proc/overloads/scalar_set.rs   |  4 ++++
 naga/tests/naga/validation.rs           | 70 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 naga/tests/naga/wgsl_errors.rs          | 55 +++++++++++++++++++++++++++++++++++++++++++++++++++++++
 5 files changed, 132 insertions(+), 1 deletion(-)
```

### Findings

Condition: where the code opts out of a check.

1. `naga/tests/naga/validation.rs:472` new, `variant(Scalar::I32).expect("module should validate");`, values `{"count":1,"escape":"expect"}`, nothing at the base matched

   ```text
   469 |             .map_err(|err| Box::new(err.into_inner()))
   470 |     }
   471 | 
   472 |     variant(Scalar::I32).expect("module should validate");
   473 |     assert!(matches!(
   474 |         variant(Scalar::U32).map_err(|e| *e),
   475 |         Err(valid::ValidationError::Function {
   476 |             source: valid::FunctionError::Expression {
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R016

- Repository: `gfx-rs/wgpu` (Rust), change 6 of 10
- Commit: `5a7601baf4ec`, judged against its first parent `2c3e503258dd`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over
- Derived cc: `15`, 95th percentile of 8,883 functions at 2c3e503, floor 5; recorded scope: whole repository
- Derived lines: `107`, 95th percentile of 8,883 functions at 2c3e503, floor 25; recorded scope: whole repository

### Commit message

> Reject unsigned inputs to sign (#10451)
>
> Co-authored-by: Ferdon <notferdon@users.noreply.github.com>

### Files the change touched

```text
 CHANGELOG.md                            |  1 +
 naga/src/proc/overloads/mathfunction.rs |  3 ++-
 naga/src/proc/overloads/scalar_set.rs   |  4 ++++
 naga/tests/naga/validation.rs           | 70 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 naga/tests/naga/wgsl_errors.rs          | 55 +++++++++++++++++++++++++++++++++++++++++++++++++++++++
 5 files changed, 132 insertions(+), 1 deletion(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 15 or body > 107 lines).

1. `naga/src/proc/overloads/mathfunction.rs:15` worsened, `pub fn overloads(self) -> impl OverloadSet {`, values `{"cc":40,"lines":102}`, ceiling cc 15, lines 107, base site `naga/src/proc/overloads/mathfunction.rs:15` with `{"cc":39,"lines":101}`

   ```text
   12 | use crate::ir;
   13 | 
   14 | impl ir::MathFunction {
   15 |     pub fn overloads(self) -> impl OverloadSet {
   16 |         use ir::MathFunction as Mf;
   17 | 
   18 |         let set: AnyOverloadSet = match self {
   19 |             // Component-wise unary numeric operations
   20 |             Mf::Abs => regular!(1, SCALAR|VECN of NUMERIC).into(),
   21 |             Mf::Sign => regular!(1, SCALAR|VECN of SIGNED_NUMERIC).into(),
   22 | 
   23 |             // Component-wise binary numeric operations
   24 |             Mf::Min | Mf::Max => regular!(2, SCALAR|VECN of NUMERIC).into(),
   25 | 
   26 |             // Component-wise ternary numeric operations
   27 |             Mf::Clamp => regular!(3, SCALAR|VECN of NUMERIC).into(),
   28 | 
   29 |             // Component-wise unary floating-point operations
   30 |             Mf::Sin
   31 |             | Mf::Cos
   32 |             | Mf::Tan
   33 |             | Mf::Asin
   34 |             | Mf::Acos
   35 |             | Mf::Atan
   36 |             | Mf::Sinh
   37 |             | Mf::Cosh
   38 |             | Mf::Tanh
   39 |             | Mf::Asinh
   40 |             | Mf::Acosh
   41 |             | Mf::Atanh
   42 |             | Mf::Saturate
   43 |             | Mf::Radians
   44 |             | Mf::Degrees
   45 |             | Mf::Ceil
   46 |             | Mf::Floor
   47 |             | Mf::Round
   48 |             | Mf::Fract
   49 |             | Mf::Trunc
   50 |             | Mf::Exp
   51 |             | Mf::Exp2
   52 |             | Mf::Log
   53 |             | Mf::Log2
   54 |             | Mf::Sqrt
   55 |             | Mf::InverseSqrt => regular!(1, SCALAR|VECN of FLOAT).into(),
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R017

- Repository: `gfx-rs/wgpu` (Rust), change 7 of 10
- Commit: `2c3e503258dd`, judged against its first parent `babefc0d26f6`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `15`, 95th percentile of 8,883 functions at babefc0, floor 5; recorded scope: whole repository
- Derived lines: `107`, 95th percentile of 8,883 functions at babefc0, floor 25; recorded scope: whole repository

### Commit message

> test: Improve tracking of slow tests (#10429)
>
> Co-authored-by: Connor Fitzgerald <connorwadefitzgerald@gmail.com>

### Files the change touched

```text
 .config/nextest.toml                           | 16 ++++++++++++++--
 .github/actions/sample-host-metrics/action.yml | 20 ++++++++++++++++++++
 .github/actions/sample-host-metrics/sample.ps1 | 37 +++++++++++++++++++++++++++++++++++++
 .github/workflows/ci.yml                       | 22 ++++++++++++++++++++++
 Cargo.lock                                     |  1 +
 Cargo.toml                                     |  4 +++-
 benches/Cargo.toml                             |  1 +
 benches/src/lib.rs                             |  5 +++++
 examples/features/src/framework.rs             |  3 +++
 player/tests/player/main.rs                    |  3 ++-
 tests/src/init.rs                              |  6 +++++-
 tests/src/lib.rs                               |  2 +-
 tests/tests/wgpu-validation/api/instance.rs    |  2 +-
 xtask/src/test.rs                              |  7 +++++++
 14 files changed, 122 insertions(+), 7 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 15 or body > 107 lines).

1. `benches/src/lib.rs:117` worsened, `pub fn main(benchmarks: Vec<Benchmark>) {`, values `{"cc":29,"lines":153}`, ceiling cc 15, lines 107, base site `benches/src/lib.rs:117` with `{"cc":29,"lines":148}`

   ```text
   114 |     --no-capture                (Ignored)
   115 | ";
   116 | 
   117 | pub fn main(benchmarks: Vec<Benchmark>) {
   118 |     // The benchmarks run as tests in CI, where they occasionally time out with
   119 |     // no output at all to say where the time went. See
   120 |     // <https://github.com/gfx-rs/wgpu/issues/9248>.
   121 |     let _ = env_logger::builder().format_timestamp_millis().try_init();
   122 | 
   123 |     let mut args = Arguments::from_env();
   124 | 
   125 |     let help = args.contains(["-h", "--help"]);
   126 | 
   127 |     if help {
   128 |         println!("{HELP}");
   129 |         return;
   130 |     }
   131 | 
   132 |     let mut color: ColorChoice = args
   133 |         .opt_value_from_str("--color")
   134 |         .unwrap_or(None)
   135 |         .unwrap_or(ColorChoice::Auto);
   136 |     if color == ColorChoice::Auto && !std::io::stdin().is_terminal() {
   137 |         color = ColorChoice::Never;
   138 |     }
   139 | 
   140 |     let exact = args.contains("--exact");
   141 |     // We don't actually need this flag, but cargo-nextest passes it in
   142 |     // test mode, so we need to accept it.
   143 |     let _no_capture = args.contains("--no-capture");
   144 | 
   145 |     #[expect(clippy::manual_map, reason = "So much clearer this way")]
   146 |     let mut override_iterations = if let Some(iters) = args.opt_value_from_str("--iters").unwrap() {
   147 |         Some(LoopControl::Iterations(iters))
   148 |     } else if let Some(seconds) = args.opt_value_from_str("--time").unwrap() {
   149 |         Some(LoopControl::Time(Duration::from_secs_f64(seconds)))
   150 |     } else {
   151 |         None
   152 |     };
   153 | 
   154 |     let baseline_name: Option<String> = args.opt_value_from_str(["-b", "--baseline"]).unwrap();
   155 |     let write_baseline: Option<String> =
   156 |         args.opt_value_from_str(["-s", "--save-baseline"]).unwrap();
   157 | 
   ```

2. `xtask/src/test.rs:29` worsened, `pub fn run_tests(`, values `{"cc":17,"lines":134}`, ceiling cc 15, lines 107, base site `xtask/src/test.rs:29` with `{"cc":15,"lines":127}`

   ```text
   26 |     }
   27 | }
   28 | 
   29 | pub fn run_tests(
   30 |     shell: Shell,
   31 |     mut args: Arguments,
   32 |     passthrough_args: Option<Vec<OsString>>,
   33 | ) -> anyhow::Result<()> {
   34 |     let llvm_cov = args.contains("--llvm-cov");
   35 |     let list = args.contains("--list");
   36 |     let no_require_agility_sdk = args.contains("--no-require-agility-sdk");
   37 | 
   38 |     // Determine the build profile from arguments
   39 |     let is_release = args.contains("--release");
   40 |     let custom_profile = args
   41 |         .opt_value_from_str::<_, String>("--cargo-profile")
   42 |         .ok()
   43 |         .flatten();
   44 |     let profile = if is_release {
   45 |         "release"
   46 |     } else if let Some(ref p) = custom_profile {
   47 |         p.as_str()
   48 |     } else {
   49 |         "debug"
   50 |     };
   51 | 
   52 |     let mut cargo_args = flatten_args(args, passthrough_args);
   53 | 
   54 |     // Re-add profile flags that were consumed during argument parsing
   55 |     #[expect(clippy::manual_map, reason = "This is much clearer than using map()")]
   56 |     let profile_arg = if is_release {
   57 |         Some(OsString::from("--release"))
   58 |     } else if let Some(ref p) = custom_profile {
   59 |         Some(OsString::from(format!("--cargo-profile={p}")))
   60 |     } else {
   61 |         None
   62 |     };
   63 | 
   64 |     if let Some(ref profile_arg) = profile_arg {
   65 |         cargo_args.insert(0, profile_arg.clone());
   66 |     }
   67 | 
   68 |     // Retries handled by cargo nextest natively
   69 | 
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R018

- Repository: `louis-e/arnis` (Rust), change 1 of 10
- Commit: `daee5a7bff24`, judged against its first parent `d4852aa3d73d`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #1362 from louis-e/terrain-realism
>
> Vanilla-style ground decoration, coherent rock and snow, Mapterhorn-only elevation

### Files the change touched

```text
 src/args.rs                            |    2 +-
 src/bedrock_block_map.rs               |   81 +++++-
 src/biome.rs                           |   84 +++++-
 src/block_definitions.rs               |   64 +++++
 src/climate.rs                         |  112 ++++----
 src/element_processing/landuse.rs      |   33 ++-
 src/element_processing/leisure.rs      |   42 +--
 src/element_processing/natural.rs      |   66 +++--
 src/element_processing/tree.rs         |   38 ++-
 src/elevation/mod.rs                   |   14 +-
 src/elevation/provider.rs              |   14 -
 src/elevation/providers/aws_terrain.rs |   13 +-
 src/elevation/providers/fixed_tile.rs  | 1109 ------------------------------------------------------------------------
 src/elevation/providers/mapterhorn.rs  |   35 ++-
 src/elevation/providers/mod.rs         |    4 +-
 src/elevation/providers/planetary.rs   |    4 -
 src/elevation/providers/regional.rs    |  223 ---------------
 src/elevation/providers/tile_math.rs   |   63 +++++
 src/elevation/providers/usgs_3dep.rs   |  144 ----------
 src/elevation/selector.rs              |  102 ++-----
 src/ground.rs                          |   80 +++++-
 src/ground_decoration.rs               |  865 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/ground_generation.rs               |  523 +++++++++++++++++++---------------
 src/gui/js/preview3d.js                |    7 +-
 src/luanti_block_map.rs                |   22 ++
 src/main.rs                            |    2 +
 src/map_renderer.rs                    |   18 ++
 src/preview_3d.rs                      |    4 +-
 src/terrain_surface.rs                 |  369 ++++++++++++++++++++++++
 src/trees/schematic.rs                 |  111 +++++++-
 ...
 32 files changed, 2348 insertions(+), 1991 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `src/ground_decoration.rs:459` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":2,"escape":"allow"}`, nothing at the base matched

   ```text
   456 | 
   457 | /// Lays plant patches over `[min_x..=max_x] x [min_z..=max_z]`, which the ground
   458 | /// pass has just finished. Needs land cover; without it the ground stays as is.
   459 | #[allow(clippy::too_many_arguments)]
   460 | pub fn decorate_region(
   461 |     editor: &mut WorldEditor,
   462 |     ground: &Ground,
   463 |     args: &Args,
   ```

2. `src/elevation/providers/mapterhorn.rs:1010` worsened, `#[ignore]`, values `{"count":3,"escape":"skipped test"}`, base site `src/elevation/providers/mapterhorn.rs:1014` with `{"count":2,"escape":"skipped test"}`

   ```text
   1007 |     }
   1008 | 
   1009 |     #[test]
   1010 |     #[ignore]
   1011 |     fn test_live_tile_fetch_and_decode() {
   1012 |         let client = reqwest::blocking::Client::new();
   1013 |         // z12 tile containing Zermatt.
   1014 |         let key = TileKey {
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R019

- Repository: `louis-e/arnis` (Rust), change 1 of 10
- Commit: `daee5a7bff24`, judged against its first parent `d4852aa3d73d`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,674 functions at d4852aa, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,674 functions at d4852aa, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1362 from louis-e/terrain-realism
>
> Vanilla-style ground decoration, coherent rock and snow, Mapterhorn-only elevation

### Files the change touched

```text
 src/args.rs                            |    2 +-
 src/bedrock_block_map.rs               |   81 +++++-
 src/biome.rs                           |   84 +++++-
 src/block_definitions.rs               |   64 +++++
 src/climate.rs                         |  112 ++++----
 src/element_processing/landuse.rs      |   33 ++-
 src/element_processing/leisure.rs      |   42 +--
 src/element_processing/natural.rs      |   66 +++--
 src/element_processing/tree.rs         |   38 ++-
 src/elevation/mod.rs                   |   14 +-
 src/elevation/provider.rs              |   14 -
 src/elevation/providers/aws_terrain.rs |   13 +-
 src/elevation/providers/fixed_tile.rs  | 1109 ------------------------------------------------------------------------
 src/elevation/providers/mapterhorn.rs  |   35 ++-
 src/elevation/providers/mod.rs         |    4 +-
 src/elevation/providers/planetary.rs   |    4 -
 src/elevation/providers/regional.rs    |  223 ---------------
 src/elevation/providers/tile_math.rs   |   63 +++++
 src/elevation/providers/usgs_3dep.rs   |  144 ----------
 src/elevation/selector.rs              |  102 ++-----
 src/ground.rs                          |   80 +++++-
 src/ground_decoration.rs               |  865 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/ground_generation.rs               |  523 +++++++++++++++++++---------------
 src/gui/js/preview3d.js                |    7 +-
 src/luanti_block_map.rs                |   22 ++
 src/main.rs                            |    2 +
 src/map_renderer.rs                    |   18 ++
 src/preview_3d.rs                      |    4 +-
 src/terrain_surface.rs                 |  369 ++++++++++++++++++++++++
 src/trees/schematic.rs                 |  111 +++++++-
 ...
 32 files changed, 2348 insertions(+), 1991 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/ground_decoration.rs:157` new, `pub(crate) fn habitat(cover: u8, climate: Climate, abs_lat: f64, alpine: bool) -> Option<Habitat> {`, values `{"cc":22,"lines":36}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   154 |     }
   155 | }
   156 | 
   157 | pub(crate) fn habitat(cover: u8, climate: Climate, abs_lat: f64, alpine: bool) -> Option<Habitat> {
   158 |     let arid = matches!(climate, Climate::HotDesert | Climate::ColdDesert);
   159 |     let dry = matches!(
   160 |         climate,
   161 |         Climate::HotSteppe
   162 |             | Climate::ColdSteppe
   163 |             | Climate::TropicalSavanna
   164 |             | Climate::DryContinental
   165 |     );
   166 |     let polar = matches!(climate, Climate::Tundra | Climate::IceCap);
   167 |     Some(match cover {
   168 |         LC_TREE_COVER => {
   169 |             if matches!(climate, Climate::Boreal) || polar || abs_lat > 55.0 {
   170 |                 Habitat::Taiga
   171 |             } else if arid || dry {
   172 |                 Habitat::Shrub
   173 |             } else if abs_lat < 23.5 {
   174 |                 Habitat::Jungle
   175 |             } else {
   176 |                 Habitat::Forest
   177 |             }
   178 |         }
   179 |         LC_SHRUBLAND | LC_GRASSLAND if alpine && !arid => Habitat::Alpine,
   180 |         LC_SHRUBLAND if arid || dry => Habitat::Steppe,
   181 |         LC_SHRUBLAND if polar => Habitat::Tundra,
   182 |         LC_SHRUBLAND => Habitat::Shrub,
   183 |         LC_GRASSLAND if arid => Habitat::Desert,
   184 |         LC_GRASSLAND if dry => Habitat::Steppe,
   185 |         LC_GRASSLAND if polar => Habitat::Tundra,
   186 |         LC_GRASSLAND => Habitat::Meadow,
   187 |         LC_MOSS => Habitat::Tundra,
   188 |         LC_WETLAND | LC_MANGROVES => Habitat::Wetland,
   189 |         LC_BARE if arid || dry => Habitat::Desert,
   190 |         _ => return None,
   191 |     })
   192 | }
   193 | 
   ```

2. `src/ground_decoration.rs:332` new, `fn features(habitat: Habitat) -> &'static [(Feature, u32)] {`, values `{"cc":11,"lines":55}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   329 | }
   330 | 
   331 | /// Features an origin may start, per mille per origin; the remainder grows nothing.
   332 | fn features(habitat: Habitat) -> &'static [(Feature, u32)] {
   333 |     use Feature::*;
   334 |     match habitat {
   335 |         Habitat::Meadow => &[
   336 |             (Flowers(MEADOW_FLOWERS), 90),
   337 |             (TallGrass, 100),
   338 |             (TallFlowers(SUNFLOWERS), 6),
   339 |             (Pumpkins, 1),
   340 |             (SugarCane, 70),
   341 |         ],
   342 |         Habitat::Alpine => &[(Flowers(ALPINE_FLOWERS), 160), (TallGrass, 50)],
   343 |         Habitat::Forest => &[
   344 |             (Flowers(FOREST_FLOWERS), 50),
   345 |             (TallFlowers(FOREST_TALL_FLOWERS), 35),
   346 |             (Ferns, 80),
   347 |             (Mushrooms, 60),
   348 |             (SugarCane, 50),
   349 |             (Pumpkins, 1),
   350 |         ],
   351 |         Habitat::Taiga => &[
   352 |             (Ferns, 200),
   353 |             (SweetBerries, 60),
   354 |             (Mushrooms, 80),
   355 |             (Flowers(BOREAL_FLOWERS), 20),
   356 |             (Moss, 40),
   357 |         ],
   358 |         Habitat::Jungle => &[
   359 |             (Ferns, 180),
   360 |             (Flowers(TROPICAL_FLOWERS), 30),
   361 |             (SugarCane, 100),
   362 |             (TallGrass, 60),
   363 |         ],
   364 |         Habitat::Shrub => &[(Flowers(SHRUB_FLOWERS), 50), (TallGrass, 70)],
   365 |         Habitat::Steppe => &[
   366 |             (TallGrass, 140),
   367 |             (DeadBushes, 80),
   368 |             (Flowers(STEPPE_FLOWERS), 20),
   369 |             (SugarCane, 60),
   370 |             (Cactus, 30),
   371 |         ],
   372 |         Habitat::Desert => &[(DeadBushes, 160), (Cactus, 80), (SugarCane, 100)],
   ```

3. `src/ground_decoration.rs:460` new, `pub fn decorate_region(`, values `{"cc":12,"lines":68}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   457 | /// Lays plant patches over `[min_x..=max_x] x [min_z..=max_z]`, which the ground
   458 | /// pass has just finished. Needs land cover; without it the ground stays as is.
   459 | #[allow(clippy::too_many_arguments)]
   460 | pub fn decorate_region(
   461 |     editor: &mut WorldEditor,
   462 |     ground: &Ground,
   463 |     args: &Args,
   464 |     xzbbox: &XZBBox,
   465 |     min_x: i32,
   466 |     max_x: i32,
   467 |     min_z: i32,
   468 |     max_z: i32,
   469 | ) {
   470 |     if !ground.has_land_cover() || !ground.body().is_earth() || min_x > max_x || min_z > max_z {
   471 |         return;
   472 |     }
   473 |     let (lat, lon) = args
   474 |         .bbox
   475 |         .as_ref()
   476 |         .map(|b| {
   477 |             (
   478 |                 (b.min().lat() + b.max().lat()) * 0.5,
   479 |                 (b.min().lng() + b.max().lng()) * 0.5,
   480 |             )
   481 |         })
   482 |         .unwrap_or((45.0, 0.0));
   483 |     let snow_y = ground.snow_threshold_y();
   484 |     let alpine_from_y = match snow_y {
   485 |         i32::MAX | i32::MIN => snow_y,
   486 |         t => t - (ALPINE_BAND_METRES * ground.blocks_per_meter()).round() as i32,
   487 |     };
   488 |     let site = Site {
   489 |         ground,
   490 |         origin_x: xzbbox.min_x(),
   491 |         origin_z: xzbbox.min_z(),
   492 |         bounds: (min_x, max_x, min_z, max_z),
   493 |         terrain: ground.elevation_enabled,
   494 |         flat_y: args.ground_level,
   495 |         climate: ground.climate(),
   496 |         abs_lat: lat.abs(),
   497 |         cactus_country: (-170.0..=-30.0).contains(&lon),
   498 |         alpine_from_y,
   499 |     };
   500 | 
   ```

4. `src/ground_decoration.rs:554` new, `fn grow_patch(`, values `{"cc":31,"lines":109}`, ceiling cc 13, lines 51, nothing at the base matched
5. `src/ground_decoration.rs:792` new, `fn patches_come_out_the_same_whatever_the_tiling() {`, values `{"cc":7,"lines":66}`, ceiling cc 13, lines 51, nothing at the base matched
6. `src/world_editor/mod.rs:1971` new, `pub fn set_block_with_properties_absolute(`, values `{"cc":11,"lines":53}`, ceiling cc 13, lines 51, nothing at the base matched
7. `src/bedrock_block_map.rs:70` worsened, `pub fn to_bedrock_block(block: Block) -> BedrockBlock {`, values `{"cc":144,"lines":844}`, ceiling cc 13, lines 51, base site `src/bedrock_block_map.rs:70` with `{"cc":138,"lines":820}`
8. `src/bedrock_block_map.rs:920` worsened, `pub fn to_bedrock_block_with_properties(`, values `{"cc":25,"lines":117}`, ceiling cc 13, lines 51, base site `src/bedrock_block_map.rs:896` with `{"cc":25,"lines":114}`
9. `src/block_definitions.rs:111` worsened, `pub fn try_name(&self) -> Option<&str> {`, values `{"cc":453,"lines":460}`, ceiling cc 13, lines 51, base site `src/block_definitions.rs:111` with `{"cc":435,"lines":442}`
10. `src/block_definitions.rs:572` worsened, `pub fn properties(&self) -> Option<Value> {`, values `{"cc":60,"lines":361}`, ceiling cc 13, lines 51, base site `src/block_definitions.rs:554` with `{"cc":56,"lines":337}`
11. `src/climate.rs:69` worsened, `pub fn surface_palette(self, cover: u8, x: i32, z: i32) -> Option<(Block, Block)> {`, values `{"cc":21,"lines":85}`, ceiling cc 13, lines 51, base site `src/climate.rs:68` with `{"cc":37,"lines":80}`
12. `src/element_processing/landuse.rs:13` worsened, `pub fn generate_landuse(`, values `{"cc":118,"lines":451}`, ceiling cc 13, lines 51, base site `src/element_processing/landuse.rs:13` with `{"cc":120,"lines":442}`
13. `src/element_processing/leisure.rs:13` worsened, `pub fn generate_leisure(`, values `{"cc":36,"lines":168}`, ceiling cc 13, lines 51, base site `src/element_processing/leisure.rs:13` with `{"cc":38,"lines":164}`
14. `src/element_processing/natural.rs:13` worsened, `pub fn generate_natural(`, values `{"cc":171,"lines":774}`, ceiling cc 13, lines 51, base site `src/element_processing/natural.rs:13` with `{"cc":176,"lines":763}`
15. `src/element_processing/tree.rs:544` worsened, `fn build(`, values `{"cc":45,"lines":304}`, ceiling cc 13, lines 51, base site `src/element_processing/tree.rs:541` with `{"cc":43,"lines":289}`
16. `src/elevation/providers/aws_terrain.rs:334` worsened, `fn fetch_or_load_tile(`, values `{"cc":6,"lines":57}`, ceiling cc 13, lines 51, base site `src/elevation/providers/aws_terrain.rs:338` with `{"cc":6,"lines":54}`
17. `src/ground_generation.rs:236` worsened, `pub fn generate_ground_region(`, values `{"cc":189,"lines":1216}`, ceiling cc 13, lines 51, base site `src/ground_generation.rs:227` with `{"cc":185,"lines":1226}`
18. `src/luanti_block_map.rs:254` worsened, `fn to_mineclonia_node(block: Block, props: Option<&Value>) -> LuantiNode {`, values `{"cc":446,"lines":523}`, ceiling cc 13, lines 51, base site `src/luanti_block_map.rs:254` with `{"cc":424,"lines":501}`
19. `src/trees/schematic.rs:278` worsened, `pub fn place_schematic_tree(`, values `{"cc":28,"lines":156}`, ceiling cc 13, lines 51, base site `src/trees/schematic.rs:287` with `{"cc":18,"lines":110}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R020

- Repository: `louis-e/arnis` (Rust), change 2 of 10
- Commit: `d4852aa3d73d`, judged against its first parent `641c2100df8c`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,623 functions at 641c210, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,623 functions at 641c210, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1361 from louis-e/discord-feedback
>
> Discord feedback

### Files the change touched

```text
 src/args.rs                        |  72 +++++++++++++++
 src/biome.rs                       |   4 +-
 src/data_processing.rs             |  15 ++++
 src/element_processing/highways.rs | 369 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/element_processing/landuse.rs  | 207 ++++++++++++++++++++++++++++++++++++++++---
 src/element_processing/natural.rs  | 105 +++++++++++++++++++---
 src/element_processing/surfaces.rs |  97 +++++++++++++++++++-
 src/elevation/mod.rs               |   9 +-
 src/floodfill_cache.rs             |  83 +++++++++++++++++-
 src/ground.rs                      |  11 +++
 src/ground_generation.rs           |  31 ++++++-
 src/gui.rs                         |  28 +++---
 src/gui/css/styles.css             |   5 ++
 src/gui/index.html                 |  37 ++++++++
 src/gui/js/main.js                 | 179 +++++++++++++++++++------------------
 src/gui/js/settings-store.js       |   2 +
 src/gui/locales/ar.json            |   8 +-
 src/gui/locales/de.json            |   8 +-
 src/gui/locales/en-US.json         |   8 +-
 src/gui/locales/es.json            |   8 +-
 src/gui/locales/fi.json            |   8 +-
 src/gui/locales/fr-FR.json         |   8 +-
 src/gui/locales/hu.json            |   8 +-
 src/gui/locales/ja.json            |   8 +-
 src/gui/locales/ka-GE.json         |   8 +-
 src/gui/locales/ko.json            |   8 +-
 src/gui/locales/lt.json            |   8 +-
 src/gui/locales/lv.json            |   8 +-
 src/gui/locales/pl.json            |   8 +-
 src/gui/locales/pt-BR.json         |   8 +-
 ...
 44 files changed, 1868 insertions(+), 421 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/element_processing/highways.rs:2737` new, `pub fn drop_buildings_on_aircraft_pavement(`, values `{"cc":27,"lines":79}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   2734 | ///
   2735 | /// Costs a scan of the elements plus, for the few buildings inside the airport's bounds, one
   2736 | /// test per strip segment; nothing is allocated per block.
   2737 | pub fn drop_buildings_on_aircraft_pavement(
   2738 |     elements: &mut Vec<ProcessedElement>,
   2739 |     scale: f64,
   2740 | ) -> usize {
   2741 |     let mut runways: Vec<StripSegment> = Vec::new();
   2742 |     let mut taxiways: Vec<StripSegment> = Vec::new();
   2743 |     // Paved surfaces drawn as polygons: runway areas drop every building, the
   2744 |     // rest (aprons, taxiway areas) only traced ones.
   2745 |     let mut runway_areas: Vec<Vec<(i32, i32)>> = Vec::new();
   2746 |     let mut aprons: Vec<Vec<(i32, i32)>> = Vec::new();
   2747 |     for element in elements.iter() {
   2748 |         let ProcessedElement::Way(way) = element else {
   2749 |             continue;
   2750 |         };
   2751 |         let closed = way.nodes.len() >= 4
   2752 |             && way.nodes.first().map(|n| (n.x, n.z)) == way.nodes.last().map(|n| (n.x, n.z));
   2753 |         let ring = || way.nodes.iter().map(|n| (n.x, n.z)).collect::<Vec<_>>();
   2754 |         let area_kind = way.tags.get("area:aeroway").map(String::as_str);
   2755 |         match (way.tags.get("aeroway").map(String::as_str), area_kind) {
   2756 |             (_, Some("runway")) if closed => runway_areas.push(ring()),
   2757 |             (_, Some("taxiway")) if closed => aprons.push(ring()),
   2758 |             (Some("runway"), _)
   2759 |                 if closed && way.tags.get("area").map(String::as_str) == Some("yes") =>
   2760 |             {
   2761 |                 runway_areas.push(ring())
   2762 |             }
   2763 |             (Some("runway"), _) => push_strip_segments(&mut runways, way, scale),
   2764 |             (Some("taxiway"), _) => push_strip_segments(&mut taxiways, way, scale),
   2765 |             (Some("apron"), _) if closed => aprons.push(ring()),
   2766 |             _ => {}
   2767 |         }
   2768 |     }
   2769 |     if runways.is_empty() && taxiways.is_empty() && runway_areas.is_empty() && aprons.is_empty() {
   2770 |         return 0;
   2771 |     }
   2772 | 
   2773 |     // Everything that can drop a building lies inside these bounds.
   2774 |     let (mut min_x, mut min_z, mut max_x, mut max_z) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
   2775 |     for s in runways.iter().chain(&taxiways) {
   2776 |         min_x = min_x.min(s.ax.min(s.bx) - s.half);
   2777 |         max_x = max_x.max(s.ax.max(s.bx) + s.half);
   ```

2. `src/element_processing/landuse.rs:461` new, `fn military_ground(`, values `{"cc":13,"lines":56}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   458 | /// forest, a heath or a desert keeps looking like one instead of turning into a concrete slab.
   459 | /// Water, wetland, beach and ice give `None` and stay with the land cover. Positional only, so the
   460 | /// tiles agree at their seams.
   461 | fn military_ground(
   462 |     editor: &WorldEditor,
   463 |     climate: crate::climate::Climate,
   464 |     x: i32,
   465 |     z: i32,
   466 |     rough: bool,
   467 | ) -> Option<Block> {
   468 |     use crate::ground_generation::value_noise_01;
   469 |     use crate::land_cover::{
   470 |         coord_hash, LC_BARE, LC_BEACH, LC_BUILT_UP, LC_MANGROVES, LC_SNOW_ICE, LC_WATER, LC_WETLAND,
   471 |     };
   472 | 
   473 |     let cover = editor.cover_class(x, z);
   474 |     let h = coord_hash(x, z);
   475 |     match cover {
   476 |         LC_WATER | LC_WETLAND | LC_MANGROVES | LC_SNOW_ICE | LC_BEACH => None,
   477 |         LC_BUILT_UP => {
   478 |             // Concrete yards with gravel hardstands for the vehicles and strips of lawn.
   479 |             let n = value_noise_01(x + 211, z + 17, 7);
   480 |             Some(if n < 0.25 {
   481 |                 GRASS_BLOCK
   482 |             } else if n > 0.8 {
   483 |                 GRAVEL
   484 |             } else {
   485 |                 match h % 10 {
   486 |                     0..=6 => POLISHED_ANDESITE,
   487 |                     7..=8 => ANDESITE,
   488 |                     _ => STONE,
   489 |                 }
   490 |             })
   491 |         }
   492 |         _ => {
   493 |             // Arid and polar bases sit on the region's own ground.
   494 |             if let Some((surface, _)) = climate.surface_palette(cover, x, z) {
   495 |                 return Some(surface);
   496 |             }
   497 |             // Vehicle tracks and training ground as organic patches in the grass: about a
   498 |             // seventh of a lawn, a third of a training area, most of bare land.
   499 |             let worn_share = if cover == LC_BARE {
   500 |                 0.7
   501 |             } else if rough {
   ```

3. `src/floodfill_cache.rs:387` new, `pub fn precompute(elements: &[ProcessedElement], timeout: Option<&Duration>) -> Self {`, values `{"cc":10,"lines":60}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   384 |     /// Pre-computes flood fills for all elements that need them.
   385 |     ///
   386 |     /// This runs in parallel using Rayon, taking advantage of multiple CPU cores.
   387 |     pub fn precompute(elements: &[ProcessedElement], timeout: Option<&Duration>) -> Self {
   388 |         // Collect all ways that need flood fill
   389 |         let mut ways_needing_fill: Vec<&ProcessedWay> = elements
   390 |             .iter()
   391 |             .filter_map(|el| match el {
   392 |                 ProcessedElement::Way(way) => {
   393 |                     if Self::way_needs_flood_fill(way) {
   394 |                         Some(way)
   395 |                     } else {
   396 |                         None
   397 |                     }
   398 |                 }
   399 |                 _ => None,
   400 |             })
   401 |             .collect();
   402 | 
   403 |         // The outer rings of the relations filled member by member. Every tile a relation
   404 |         // overlaps used to fill them again, which for a large beach or dune field meant a
   405 |         // full-size fill per tile thread; once here they are shared like a way's.
   406 |         let mut seen: fnv::FnvHashSet<u64> = ways_needing_fill.iter().map(|w| w.id).collect();
   407 |         for element in elements {
   408 |             let ProcessedElement::Relation(rel) = element else {
   409 |                 continue;
   410 |             };
   411 |             if !Self::relation_fills_members(rel) {
   412 |                 continue;
   413 |             }
   414 |             for member in &rel.members {
   415 |                 if member.role == ProcessedMemberRole::Outer && seen.insert(member.way.id) {
   416 |                     ways_needing_fill.push(&member.way);
   417 |                 }
   418 |             }
   419 |         }
   420 | 
   421 |         // Compute all way flood fills in parallel
   422 |         let way_results: Vec<(u64, Vec<(i32, i32)>)> = ways_needing_fill
   423 |             .par_iter()
   424 |             .map(|way| {
   425 |                 let polygon_coords: Vec<(i32, i32)> =
   426 |                     way.nodes.iter().map(|n| (n.x, n.z)).collect();
   427 |                 let filled = flood_fill_area(&polygon_coords, timeout);
   ```

4. `src/land_cover/mod.rs:88` new, `pub fn mark_beaches(lc: &mut LandCoverData) {`, values `{"cc":22,"lines":53}`, ceiling cc 13, lines 51, nothing at the base matched
5. `src/osm_parser.rs:1854` new, `fn assemble_area_rings(`, values `{"cc":15,"lines":60}`, ceiling cc 13, lines 51, nothing at the base matched
6. `src/biome.rs:14` worsened, `pub fn biome_for_class(lc: u8, climate: Climate, lat_deg: f64, water_dist: u8) -> &'static str {`, values `{"cc":20,"lines":53}`, ceiling cc 13, lines 51, base site `src/biome.rs:14` with `{"cc":19,"lines":52}`
7. `src/biome.rs:69` worsened, `fn biome_temperate(lc: u8, lat_deg: f64, water_dist: u8) -> &'static str {`, values `{"cc":15,"lines":36}`, ceiling cc 13, lines 51, base site `src/biome.rs:68` with `{"cc":14,"lines":35}`
8. `src/data_processing.rs:546` worsened, `pub fn generate_world_with_options(`, values `{"cc":198,"lines":1351}`, ceiling cc 13, lines 51, base site `src/data_processing.rs:546` with `{"cc":194,"lines":1336}`
9. `src/element_processing/highways.rs:768` worsened, `pub fn generate_highway_tunnel_shell(`, values `{"cc":45,"lines":213}`, ceiling cc 13, lines 51, base site `src/element_processing/highways.rs:747` with `{"cc":45,"lines":210}`
10. `src/element_processing/highways.rs:1270` worsened, `fn generate_highways_internal(`, values `{"cc":144,"lines":1000}`, ceiling cc 13, lines 51, base site `src/element_processing/highways.rs:1246` with `{"cc":142,"lines":991}`
11. `src/element_processing/highways.rs:2900` worsened, `pub(crate) fn highway_block_range(`, values `{"cc":14,"lines":45}`, ceiling cc 13, lines 51, base site `src/element_processing/highways.rs:2725` with `{"cc":14,"lines":43}`
12. `src/element_processing/landuse.rs:13` worsened, `pub fn generate_landuse(`, values `{"cc":120,"lines":442}`, ceiling cc 13, lines 51, base site `src/element_processing/landuse.rs:13` with `{"cc":119,"lines":439}`
13. `src/element_processing/natural.rs:13` worsened, `pub fn generate_natural(`, values `{"cc":176,"lines":763}`, ceiling cc 13, lines 51, base site `src/element_processing/natural.rs:12` with `{"cc":175,"lines":760}`
14. `src/elevation/mod.rs:193` worsened, `pub fn fetch_elevation_data(`, values `{"cc":13,"lines":161}`, ceiling cc 13, lines 51, base site `src/elevation/mod.rs:192` with `{"cc":13,"lines":157}`
15. `src/ground.rs:381` worsened, `pub fn new_enabled(`, values `{"cc":18,"lines":159}`, ceiling cc 13, lines 51, base site `src/ground.rs:381` with `{"cc":18,"lines":157}`
16. `src/ground.rs:1078` worsened, `pub fn save_land_cover_debug_image(&self, filename: &str) {`, values `{"cc":19,"lines":38}`, ceiling cc 13, lines 51, base site `src/ground.rs:1069` with `{"cc":18,"lines":37}`
17. `src/ground_generation.rs:227` worsened, `pub fn generate_ground_region(`, values `{"cc":185,"lines":1226}`, ceiling cc 13, lines 51, base site `src/ground_generation.rs:227` with `{"cc":179,"lines":1197}`
18. `src/gui.rs:1374` worsened, `fn gui_start_generation(`, values `{"cc":92,"lines":709}`, ceiling cc 13, lines 51, base site `src/gui.rs:1389` with `{"cc":91,"lines":700}`
19. `src/gui/js/main.js:124` worsened, `async function applyLocalization(localization) {`, values `{"cc":4,"lines":57}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:124` with `{"cc":4,"lines":56}`
20. `src/gui/js/main.js:1137` worsened, `function initSettings() {`, values `{"cc":5,"lines":464}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1136` with `{"cc":5,"lines":442}`
21. `src/gui/js/main.js:3240` worsened, `async function startGeneration() {`, values `{"cc":51,"lines":196}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:3245` with `{"cc":49,"lines":192}`
22. `src/main.rs:178` worsened, `fn run_cli() {`, values `{"cc":66,"lines":622}`, ceiling cc 13, lines 51, base site `src/main.rs:178` with `{"cc":66,"lines":603}`
23. `src/one_world.rs:169` worsened, `fn validate(&self) -> Result<(), String> {`, values `{"cc":16,"lines":30}`, ceiling cc 13, lines 51, base site `src/one_world.rs:156` with `{"cc":15,"lines":29}`
24. `src/one_world.rs:369` worsened, `fn resolve(`, values `{"cc":25,"lines":192}`, ceiling cc 13, lines 51, base site `src/one_world.rs:355` with `{"cc":24,"lines":185}`
25. `src/osm_parser.rs:833` worsened, `pub fn parse_osm_data(`, values `{"cc":48,"lines":318}`, ceiling cc 13, lines 51, base site `src/osm_parser.rs:833` with `{"cc":41,"lines":293}`
26. `src/preview_3d.rs:75` worsened, `pub fn build_preview_payload(bbox_text: &str, aws_only: bool) -> Result<Vec<u8>, String> {`, values `{"cc":15,"lines":93}`, ceiling cc 13, lines 51, base site `src/preview_3d.rs:75` with `{"cc":15,"lines":92}`
27. `src/world_editor/java.rs:369` worsened, `fn write_region_to_disk(`, values `{"cc":27,"lines":153}`, ceiling cc 13, lines 51, base site `src/world_editor/java.rs:363` with `{"cc":22,"lines":142}`
28. `src/world_utils.rs:816` worsened, `pub fn apply_java_world_settings(`, values `{"cc":14,"lines":63}`, ceiling cc 13, lines 51, base site `src/world_utils.rs:891` with `{"cc":12,"lines":59}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R021

- Repository: `louis-e/arnis` (Rust), change 2 of 10
- Commit: `d4852aa3d73d`, judged against its first parent `641c2100df8c`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> Merge pull request #1361 from louis-e/discord-feedback
>
> Discord feedback

### Files the change touched

```text
 src/args.rs                        |  72 +++++++++++++++
 src/biome.rs                       |   4 +-
 src/data_processing.rs             |  15 ++++
 src/element_processing/highways.rs | 369 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/element_processing/landuse.rs  | 207 ++++++++++++++++++++++++++++++++++++++++---
 src/element_processing/natural.rs  | 105 +++++++++++++++++++---
 src/element_processing/surfaces.rs |  97 +++++++++++++++++++-
 src/elevation/mod.rs               |   9 +-
 src/floodfill_cache.rs             |  83 +++++++++++++++++-
 src/ground.rs                      |  11 +++
 src/ground_generation.rs           |  31 ++++++-
 src/gui.rs                         |  28 +++---
 src/gui/css/styles.css             |   5 ++
 src/gui/index.html                 |  37 ++++++++
 src/gui/js/main.js                 | 179 +++++++++++++++++++------------------
 src/gui/js/settings-store.js       |   2 +
 src/gui/locales/ar.json            |   8 +-
 src/gui/locales/de.json            |   8 +-
 src/gui/locales/en-US.json         |   8 +-
 src/gui/locales/es.json            |   8 +-
 src/gui/locales/fi.json            |   8 +-
 src/gui/locales/fr-FR.json         |   8 +-
 src/gui/locales/hu.json            |   8 +-
 src/gui/locales/ja.json            |   8 +-
 src/gui/locales/ka-GE.json         |   8 +-
 src/gui/locales/ko.json            |   8 +-
 src/gui/locales/lt.json            |   8 +-
 src/gui/locales/lv.json            |   8 +-
 src/gui/locales/pl.json            |   8 +-
 src/gui/locales/pt-BR.json         |   8 +-
 ...
 44 files changed, 1868 insertions(+), 421 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `src/one_world.rs:96` new, `fn real_height() -> f64 {`, values `{"dead":1}`, nothing at the base matched

   ```text
   093 |     pub areas: Vec<GeneratedArea>,
   094 | }
   095 | 
   096 | fn real_height() -> f64 {
   097 |     1.0
   098 | }
   099 | 
   100 | fn is_real_height(multiplier: &f64) -> bool {
   ```

2. `src/one_world.rs:100` new, `fn is_real_height(multiplier: &f64) -> bool {`, values `{"dead":1}`, nothing at the base matched

   ```text
   097 |     1.0
   098 | }
   099 | 
   100 | fn is_real_height(multiplier: &f64) -> bool {
   101 |     *multiplier == 1.0
   102 | }
   103 | 
   104 | impl Manifest {
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R022

- Repository: `louis-e/arnis` (Rust), change 3 of 10
- Commit: `641c2100df8c`, judged against its first parent `7e9a4344c53c`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,596 functions at 7e9a434, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,596 functions at 7e9a434, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1360 from louis-e/claude/zealous-clarke-iqhm9t
>
> Tighten telemetry: sync consent at startup, report generation failures

### Files the change touched

```text
 src/gui.rs                  |  87 +++++++++++++++++++++++++++++++++++++++++++-----------
 src/gui/js/main.js          |  11 +++++++
 src/progress.rs             |  12 ++------
 src/world_editor/bedrock.rs | 135 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++---
 src/world_editor/mod.rs     |  19 ++++--------
 5 files changed, 219 insertions(+), 45 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/gui.rs:66` worsened, `pub fn run_gui() -> Result<(), String> {`, values `{"cc":4,"lines":88}`, ceiling cc 13, lines 51, base site `src/gui.rs:66` with `{"cc":4,"lines":87}`

   ```text
   063 |     }
   064 | }
   065 | 
   066 | pub fn run_gui() -> Result<(), String> {
   067 |     // Configure thread pool with 90% CPU cap to keep system responsive
   068 |     crate::floodfill_cache::configure_rayon_thread_pool(0.9);
   069 | 
   070 |     // Clean up old cached elevation tiles on startup
   071 |     crate::elevation_data::cleanup_old_cached_tiles();
   072 | 
   073 |     // Launch the UI
   074 |     println!("Launching UI...");
   075 | 
   076 |     // Install panic hook for crash reporting
   077 |     telemetry::install_panic_hook();
   078 | 
   079 |     // Workaround WebKit2GTK issue with NVIDIA drivers and graphics issues
   080 |     // Source: https://github.com/tauri-apps/tauri/issues/10702
   081 |     #[cfg(target_os = "linux")]
   082 |     unsafe {
   083 |         // Disable problematic GPU features that cause map loading issues
   084 |         env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
   085 |         env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
   086 | 
   087 |         // Force software rendering for better compatibility.
   088 |         // Only set if not already configured by the user, allowing manual override
   089 |         // for systems where software rendering causes EGL_BAD_PARAMETER (see #1247).
   090 |         if env::var("LIBGL_ALWAYS_SOFTWARE").is_err() {
   091 |             env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
   092 |         }
   093 |         if env::var("GALLIUM_DRIVER").is_err() {
   094 |             env::set_var("GALLIUM_DRIVER", "softpipe");
   095 |         }
   096 | 
   097 |         // Note: Removed sandbox disabling for security reasons
   098 |         // Note: Removed Qt WebEngine flags as they don't apply to Tauri
   099 |     }
   100 | 
   101 |     tauri::Builder::default()
   102 |         .plugin(
   103 |             LogBuilder::default()
   104 |                 .level(LevelFilter::Info)
   105 |                 .targets([
   106 |                     Target::new(TargetKind::LogDir {
   ```

2. `src/gui.rs:1389` worsened, `fn gui_start_generation(`, values `{"cc":91,"lines":700}`, ceiling cc 13, lines 51, base site `src/gui.rs:1378` with `{"cc":91,"lines":698}`

   ```text
   1386 | #[tauri::command(async)]
   1387 | #[allow(clippy::too_many_arguments)]
   1388 | #[allow(unused_variables)]
   1389 | fn gui_start_generation(
   1390 |     bbox_text: String,
   1391 |     selected_world: String,
   1392 |     bedrock_save_path: String,
   1393 |     luanti_save_path: String,
   1394 |     world_scale: f64,
   1395 |     ground_level: i32,
   1396 |     terrain_enabled: bool,
   1397 |     skip_osm_objects: bool,
   1398 |     interior_enabled: bool,
   1399 |     fillground_enabled: bool,
   1400 |     caves_enabled: bool,
   1401 |     legacy_trees_enabled: bool,
   1402 |     max_tree_size: String,
   1403 |     canopy_height_enabled: bool,
   1404 |     overture_enabled: bool,
   1405 |     use_3d_enabled: bool,
   1406 |     disable_height_limit: bool,
   1407 |     aws_only_elevation: bool,
   1408 |     bake_lighting_enabled: bool,
   1409 |     voxy_lod_enabled: bool,
   1410 |     is_new_world: bool,
   1411 |     spawn_point: Option<(f64, f64)>,
   1412 |     telemetry_consent: bool,
   1413 |     world_format: String,
   1414 |     rotation_angle: f64,
   1415 |     gamemode: String,
   1416 |     world_time: i64,
   1417 |     map_item: bool,
   1418 |     signage: String,
   1419 |     mapillary_token: String,
   1420 |     facades_enabled: bool,
   1421 |     facade_mode: String,
   1422 |     building_facades_enabled: bool,
   1423 |     facade_detail: String,
   1424 |     celestial_body_name: String,
   1425 |     one_world: bool,
   1426 |     one_world_name: String,
   1427 | ) -> Result<(), String> {
   1428 |     use progress::emit_gui_error;
   1429 |     use LLBBox;
   ```

3. `src/gui/js/main.js:1136` worsened, `function initSettings() {`, values `{"cc":5,"lines":442}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1136` with `{"cc":5,"lines":441}`

   ```text
   1133 |   resolve('gui_get_default_luanti_save_path', 'luantiSavePath');
   1134 | }
   1135 | 
   1136 | function initSettings() {
   1137 |   // Settings
   1138 |   const settingsModal = document.getElementById("settings-modal");
   1139 |   const slider = document.getElementById("scale-value-slider");
   1140 |   const sliderValue = document.getElementById("slider-value");
   1141 | 
   1142 |   // Where focus goes back to once the page closes.
   1143 |   let focusBeforeSettings = null;
   1144 | 
   1145 |   // Open the settings page
   1146 |   function openSettings() {
   1147 |     focusBeforeSettings = document.activeElement;
   1148 |     settingsModal.style.display = "flex";
   1149 |     syncSettingsLayout();
   1150 |     // Focus moves onto the page, so Tab and the arrow keys act on it rather
   1151 |     // than on the map behind it.
   1152 |     settingsModal.focus({ preventScroll: true });
   1153 |     // The caches grow with every generation, so the number the panel shows
   1154 |     // has to be read when the panel opens; measuring it once at startup left
   1155 |     // it stale for the whole session.
   1156 |     refreshCacheSize();
   1157 |   }
   1158 | 
   1159 |   // Close the settings page
   1160 |   function closeSettings() {
   1161 |     settingsModal.style.display = "none";
   1162 |     // Webview teardown events are not guaranteed, so commit here.
   1163 |     flushSettingsStore();
   1164 |     cancelSettingsResetConfirm();
   1165 |     if (focusBeforeSettings && typeof focusBeforeSettings.focus === "function") {
   1166 |       focusBeforeSettings.focus({ preventScroll: true });
   1167 |     }
   1168 |     focusBeforeSettings = null;
   1169 |   }
   1170 | 
   1171 |   // Escape closes the topmost dialog only. License and version info open on
   1172 |   // top of the settings page, so closing them returns to it.
   1173 |   document.addEventListener("keydown", (event) => {
   1174 |     if (event.key !== "Escape") return;
   1175 | 
   1176 |     const licenseModal = document.getElementById("license-modal");
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R023

- Repository: `louis-e/arnis` (Rust), change 4 of 10
- Commit: `7e9a4344c53c`, judged against its first parent `69811f59d213`, exit 1
- Gate: doc-size, FAIL
- Decision group: document README.md
- Derived CODE_OF_CONDUCT.md: `750`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `800`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> Merge pull request #1356 from louis-e/one-world
>
> Add One World: one persistent Java world that every generation extends

### Files the change touched

```text
 Cargo.lock                              |   1 +
 Cargo.toml                              |   3 +
 README.md                               |   8 ++
 docs/one_world.md                       | 283 +++++++++++++++++++++++++++++++++++++++++
 src/args.rs                             |  75 ++++++++---
 src/biome.rs                            |   7 +-
 src/canopy/mod.rs                       |  13 ++
 src/climate.rs                          |   4 +
 src/coordinate_system/transformation.rs | 107 +++++-----------
 src/data_processing.rs                  | 121 +++++++++++++++---
 src/decals/registry.rs                  |  12 +-
 src/element_processing/amenities.rs     |   5 +-
 src/element_processing/signage.rs       |   3 +-
 src/elevation/mod.rs                    |  83 +++++++++++-
 src/elevation/postprocess.rs            | 497 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/grid_ops.rs                         | 264 ++++++++++++++++++++++++++++++++++++++
 src/ground.rs                           | 393 +++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/gui.rs                              | 328 +++++++++++++++++++++++++++++++++--------------
 src/gui/css/styles.css                  |  21 ++++
 src/gui/index.html                      |  40 ++++++
 src/gui/js/bbox.js                      | 155 ++++++++++++++++++++---
 src/gui/js/main.js                      | 472 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/gui/js/settings-store.js            |   1 +
 src/gui/locales/ar.json                 |  16 ++-
 src/gui/locales/de.json                 |  16 ++-
 src/gui/locales/en-US.json              |  16 ++-
 src/gui/locales/es.json                 |  16 ++-
 src/gui/locales/fi.json                 |  16 ++-
 src/gui/locales/fr-FR.json              |  16 ++-
 src/gui/locales/hu.json                 |  16 ++-
 ...
 57 files changed, 5311 insertions(+), 476 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `README.md` new, values `{"ceiling":800,"words":880}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R024

- Repository: `louis-e/arnis` (Rust), change 4 of 10
- Commit: `7e9a4344c53c`, judged against its first parent `69811f59d213`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #1356 from louis-e/one-world
>
> Add One World: one persistent Java world that every generation extends

### Files the change touched

```text
 Cargo.lock                              |   1 +
 Cargo.toml                              |   3 +
 README.md                               |   8 ++
 docs/one_world.md                       | 283 +++++++++++++++++++++++++++++++++++++++++
 src/args.rs                             |  75 ++++++++---
 src/biome.rs                            |   7 +-
 src/canopy/mod.rs                       |  13 ++
 src/climate.rs                          |   4 +
 src/coordinate_system/transformation.rs | 107 +++++-----------
 src/data_processing.rs                  | 121 +++++++++++++++---
 src/decals/registry.rs                  |  12 +-
 src/element_processing/amenities.rs     |   5 +-
 src/element_processing/signage.rs       |   3 +-
 src/elevation/mod.rs                    |  83 +++++++++++-
 src/elevation/postprocess.rs            | 497 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/grid_ops.rs                         | 264 ++++++++++++++++++++++++++++++++++++++
 src/ground.rs                           | 393 +++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/gui.rs                              | 328 +++++++++++++++++++++++++++++++++--------------
 src/gui/css/styles.css                  |  21 ++++
 src/gui/index.html                      |  40 ++++++
 src/gui/js/bbox.js                      | 155 ++++++++++++++++++++---
 src/gui/js/main.js                      | 472 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/gui/js/settings-store.js            |   1 +
 src/gui/locales/ar.json                 |  16 ++-
 src/gui/locales/de.json                 |  16 ++-
 src/gui/locales/en-US.json              |  16 ++-
 src/gui/locales/es.json                 |  16 ++-
 src/gui/locales/fi.json                 |  16 ++-
 src/gui/locales/fr-FR.json              |  16 ++-
 src/gui/locales/hu.json                 |  16 ++-
 ...
 57 files changed, 5311 insertions(+), 476 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `src/world_utils.rs:424` new, `let mut fl: libc::flock = unsafe { std::mem::zeroed() };`, values `{"count":2,"escape":"unsafe"}`, nothing at the base matched

   ```text
   421 | fn posix_lock(file: &fs::File) -> std::io::Result<()> {
   422 |     use std::os::unix::io::AsRawFd;
   423 |     // SAFETY: flock is plain data; zeroed is a valid whole-file request.
   424 |     let mut fl: libc::flock = unsafe { std::mem::zeroed() };
   425 |     fl.l_type = libc::F_WRLCK as _;
   426 |     fl.l_whence = libc::SEEK_SET as _;
   427 |     // SAFETY: the descriptor is open for the lifetime of `file`.
   428 |     if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &fl) } == 0 {
   ```

2. `src/world_utils.rs:428` new, `if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &fl) } == 0 {`, values `{"count":1,"escape":"unsafe"}`, nothing at the base matched

   ```text
   425 |     fl.l_type = libc::F_WRLCK as _;
   426 |     fl.l_whence = libc::SEEK_SET as _;
   427 |     // SAFETY: the descriptor is open for the lifetime of `file`.
   428 |     if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &fl) } == 0 {
   429 |         Ok(())
   430 |     } else {
   431 |         Err(std::io::Error::last_os_error())
   432 |     }
   ```

3. `src/world_utils.rs:442` new, `let r = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETLK, &mut fl) };`, values `{"count":1,"escape":"unsafe"}`, nothing at the base matched

   ```text
   439 |     let mut fl: libc::flock = unsafe { std::mem::zeroed() };
   440 |     fl.l_type = libc::F_WRLCK as _;
   441 |     fl.l_whence = libc::SEEK_SET as _;
   442 |     let r = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETLK, &mut fl) };
   443 |     // F_UNLCK is an i32 on Linux and an i16 on macOS; l_type is always c_short.
   444 |     let unlocked: libc::c_short = libc::F_UNLCK as _;
   445 |     r == 0 && fl.l_type != unlocked
   446 | }
   ```

4. `src/world_editor/java.rs:362` worsened, `#[allow(clippy::too_many_arguments)]`, values `{"count":2,"escape":"allow"}`, base site `src/world_editor/java.rs:218` with `{"count":1,"escape":"allow"}`

### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R025

- Repository: `louis-e/arnis` (Rust), change 4 of 10
- Commit: `7e9a4344c53c`, judged against its first parent `69811f59d213`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,388 functions at 69811f5, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,388 functions at 69811f5, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1356 from louis-e/one-world
>
> Add One World: one persistent Java world that every generation extends

### Files the change touched

```text
 Cargo.lock                              |   1 +
 Cargo.toml                              |   3 +
 README.md                               |   8 ++
 docs/one_world.md                       | 283 +++++++++++++++++++++++++++++++++++++++++
 src/args.rs                             |  75 ++++++++---
 src/biome.rs                            |   7 +-
 src/canopy/mod.rs                       |  13 ++
 src/climate.rs                          |   4 +
 src/coordinate_system/transformation.rs | 107 +++++-----------
 src/data_processing.rs                  | 121 +++++++++++++++---
 src/decals/registry.rs                  |  12 +-
 src/element_processing/amenities.rs     |   5 +-
 src/element_processing/signage.rs       |   3 +-
 src/elevation/mod.rs                    |  83 +++++++++++-
 src/elevation/postprocess.rs            | 497 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/grid_ops.rs                         | 264 ++++++++++++++++++++++++++++++++++++++
 src/ground.rs                           | 393 +++++++++++++++++++++++++++++++++++++++++++++++++++++----
 src/gui.rs                              | 328 +++++++++++++++++++++++++++++++++--------------
 src/gui/css/styles.css                  |  21 ++++
 src/gui/index.html                      |  40 ++++++
 src/gui/js/bbox.js                      | 155 ++++++++++++++++++++---
 src/gui/js/main.js                      | 472 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/gui/js/settings-store.js            |   1 +
 src/gui/locales/ar.json                 |  16 ++-
 src/gui/locales/de.json                 |  16 ++-
 src/gui/locales/en-US.json              |  16 ++-
 src/gui/locales/es.json                 |  16 ++-
 src/gui/locales/fi.json                 |  16 ++-
 src/gui/locales/fr-FR.json              |  16 ++-
 src/gui/locales/hu.json                 |  16 ++-
 ...
 57 files changed, 5311 insertions(+), 476 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/elevation/postprocess.rs:1917` new, `fn derive_affine(`, values `{"cc":13,"lines":100}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   1914 |     effective_max_y - TERRAIN_HEIGHT_BUFFER
   1915 | }
   1916 | 
   1917 | fn derive_affine(
   1918 |     blurred_heights: &[Vec<f64>],
   1919 |     scale: f64,
   1920 |     ground_level: i32,
   1921 |     min_ground_level: i32,
   1922 |     disable_height_limit: bool,
   1923 |     extended_max_y: i32,
   1924 | ) -> (ElevationAffine, FitRange) {
   1925 |     // Derive min/max
   1926 |     let (min_height, max_height) = blurred_heights
   1927 |         .par_iter()
   1928 |         .map(|row| {
   1929 |             let mut lo = f64::MAX;
   1930 |             let mut hi = f64::MIN;
   1931 |             for &h in row {
   1932 |                 if h.is_finite() {
   1933 |                     lo = lo.min(h);
   1934 |                     hi = hi.max(h);
   1935 |                 }
   1936 |             }
   1937 |             (lo, hi)
   1938 |         })
   1939 |         .reduce(
   1940 |             || (f64::MAX, f64::MIN),
   1941 |             |(lo1, hi1), (lo2, hi2)| (lo1.min(lo2), hi1.max(hi2)),
   1942 |         );
   1943 | 
   1944 |     let (min_height, height_range) =
   1945 |         if !min_height.is_finite() || !max_height.is_finite() || min_height >= max_height {
   1946 |             // Zero-relief/degenerate: keep the real min height (the snow line
   1947 |             // needs it) but flatten the range so every cell maps to ground_level.
   1948 |             // `min <= max` distinguishes true flat terrain from an all-NaN grid,
   1949 |             // whose reduce leaves min = f64::MAX (finite but bogus) -> use 0.
   1950 |             let real_min = if min_height.is_finite() && min_height <= max_height {
   1951 |                 min_height
   1952 |             } else {
   1953 |                 0.0
   1954 |             };
   1955 |             (real_min, 0.0_f64)
   1956 |         } else {
   1957 |             (min_height, max_height - min_height)
   ```

2. `src/elevation/postprocess.rs:2065` new, `pub fn scale_to_minecraft_with(`, values `{"cc":19,"lines":130}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   2062 |         .collect()
   2063 | }
   2064 | 
   2065 | pub fn scale_to_minecraft_with(
   2066 |     blurred_heights: &[Vec<f64>],
   2067 |     scale: f64,
   2068 |     ground_level: i32,
   2069 |     min_ground_level: i32,
   2070 |     disable_height_limit: bool,
   2071 |     extended_max_y: i32,
   2072 |     policy: AffinePolicy,
   2073 | ) -> (Vec<Vec<f64>>, ElevationAffine) {
   2074 |     let ceiling = terrain_ceiling(disable_height_limit, extended_max_y);
   2075 |     let upper_clamp = ceiling as f64;
   2076 | 
   2077 |     let mut fit = None;
   2078 |     let affine = match policy {
   2079 |         AffinePolicy::Fit => {
   2080 |             let (affine, range) = derive_affine(
   2081 |                 blurred_heights,
   2082 |                 scale,
   2083 |                 ground_level,
   2084 |                 min_ground_level,
   2085 |                 disable_height_limit,
   2086 |                 extended_max_y,
   2087 |             );
   2088 |             fit = Some(range);
   2089 |             affine
   2090 |         }
   2091 |         AffinePolicy::FitWithHeadroom => {
   2092 |             let (mut affine, range) = derive_affine(
   2093 |                 blurred_heights,
   2094 |                 scale,
   2095 |                 ground_level,
   2096 |                 min_ground_level,
   2097 |                 disable_height_limit,
   2098 |                 extended_max_y,
   2099 |             );
   2100 |             // A flat first area chose no slope; later areas need one.
   2101 |             if affine.blocks_per_meter <= 0.0 {
   2102 |                 affine.blocks_per_meter = scale;
   2103 |             }
   2104 |             let free = (ceiling - affine.ground_level) as f64 - range.scaled_range;
   2105 |             let margin_blocks = (free * 0.5).min(HEADROOM_MAX_BLOCKS).floor().max(0.0);
   ```

3. `src/ground.rs:258` new, `pub fn new_flat_with_land_cover(`, values `{"cc":8,"lines":52}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   255 |     }
   256 | 
   257 |     /// Flat ground (no elevation) that still carries land cover, so water bodies and land-cover surfaces render at the flat surface level.
   258 |     pub fn new_flat_with_land_cover(
   259 |         bbox: &LLBBox,
   260 |         scale: f64,
   261 |         ground_level: i32,
   262 |         canopy_height: bool,
   263 |         frame: &GroundFrame,
   264 |     ) -> Self {
   265 |         let plan = frame.fetch_plan(bbox, scale);
   266 |         let fetch_bbox = plan.bbox;
   267 |         let (grid_w, grid_h) = plan.grid_dims();
   268 |         // Canopy depends on neither, so it downloads alongside the land cover.
   269 |         let (mut land_cover, mut canopy) = std::thread::scope(|s| {
   270 |             let job = canopy_height
   271 |                 .then(|| s.spawn(move || canopy::fetch_canopy_data(&fetch_bbox, grid_w, grid_h)));
   272 |             let lc = land_cover::fetch_land_cover_data(&fetch_bbox, grid_w, grid_h);
   273 |             (lc, job.and_then(|h| h.join().ok()).flatten())
   274 |         });
   275 |         if land_cover.is_none() {
   276 |             eprintln!("Land cover fetch failed; generating flat ground without it.");
   277 |         }
   278 |         if frame.mercator.is_some() {
   279 |             if let Some(lc) = land_cover.as_mut() {
   280 |                 lc.remap_rows_to_mercator(fetch_bbox.max().lat(), fetch_bbox.min().lat());
   281 |             }
   282 |             if let Some(c) = canopy.as_mut() {
   283 |                 c.remap_rows_to_mercator(fetch_bbox.max().lat(), fetch_bbox.min().lat());
   284 |             }
   285 |         }
   286 |         let (world_w, world_h) = plan.final_dims;
   287 |         if plan.pad > 0 {
   288 |             if let Some(lc) = land_cover.as_mut() {
   289 |                 lc.crop(plan.pad, plan.pad, world_w, world_h);
   290 |             }
   291 |             if let Some(c) = canopy.as_mut() {
   292 |                 c.crop(plan.pad, plan.pad, world_w, world_h);
   293 |             }
   294 |         }
   295 |         Self {
   296 |             elevation_enabled: false,
   297 |             extended_ceiling: false,
   298 |             ground_level,
   ```

4. `src/gui/js/main.js:2682` new, `async function commitWorldNameEdit() {`, values `{"cc":11,"lines":55}`, ceiling cc 13, lines 51, nothing at the base matched
5. `src/gui/js/main.js:3035` new, `async function refreshOneWorldState() {`, values `{"cc":19,"lines":59}`, ceiling cc 13, lines 51, nothing at the base matched
6. `src/one_world.rs:156` new, `fn validate(&self) -> Result<(), String> {`, values `{"cc":15,"lines":29}`, ceiling cc 13, lines 51, nothing at the base matched
7. `src/one_world.rs:301` new, `pub fn prepare(world_dir: &Path, requested: &LLBBox, args: &mut Args) -> Result<Session, String> {`, values `{"cc":17,"lines":51}`, ceiling cc 13, lines 51, nothing at the base matched
8. `src/one_world.rs:355` new, `fn resolve(`, values `{"cc":24,"lines":185}`, ceiling cc 13, lines 51, nothing at the base matched
9. `src/world_editor/java.rs:2125` new, `fn a_merge_keeps_the_chunks_of_an_earlier_run_and_writes_no_filler() {`, values `{"cc":1,"lines":69}`, ceiling cc 13, lines 51, nothing at the base matched
10. `src/world_utils.rs:212` new, `pub fn write_world_skeleton(`, values `{"cc":23,"lines":104}`, ceiling cc 13, lines 51, nothing at the base matched
11. `src/args.rs:638` worsened, `pub fn validate_args(args: &Args) -> Result<(), String> {`, values `{"cc":80,"lines":267}`, ceiling cc 13, lines 51, base site `src/args.rs:623` with `{"cc":72,"lines":252}`
12. `src/data_processing.rs:546` worsened, `pub fn generate_world_with_options(`, values `{"cc":194,"lines":1336}`, ceiling cc 13, lines 51, base site `src/data_processing.rs:546` with `{"cc":166,"lines":1245}`
13. `src/element_processing/amenities.rs:18` worsened, `pub fn generate_amenities(`, values `{"cc":54,"lines":440}`, ceiling cc 13, lines 51, base site `src/element_processing/amenities.rs:18` with `{"cc":54,"lines":439}`
14. `src/elevation/mod.rs:192` worsened, `pub fn fetch_elevation_data(`, values `{"cc":13,"lines":157}`, ceiling cc 13, lines 51, base site `src/elevation/mod.rs:125` with `{"cc":13,"lines":147}`
15. `src/ground.rs:381` worsened, `pub fn new_enabled(`, values `{"cc":18,"lines":157}`, ceiling cc 13, lines 51, base site `src/ground.rs:217` with `{"cc":12,"lines":130}`
16. `src/gui.rs:66` worsened, `pub fn run_gui() -> Result<(), String> {`, values `{"cc":4,"lines":87}`, ceiling cc 13, lines 51, base site `src/gui.rs:105` with `{"cc":4,"lines":84}`
17. `src/gui.rs:1378` worsened, `fn gui_start_generation(`, values `{"cc":91,"lines":698}`, ceiling cc 13, lines 51, base site `src/gui.rs:1302` with `{"cc":71,"lines":630}`
18. `src/gui/js/bbox.js:464` worsened, `$(document).ready(function () {`, values `{"cc":14,"lines":1963}`, ceiling cc 13, lines 51, base site `src/gui/js/bbox.js:464` with `{"cc":14,"lines":1842}`
19. `src/gui/js/bbox.js:1407` worsened, `window.addEventListener('message', function(event) {`, values `{"cc":36,"lines":94}`, ceiling cc 13, lines 51, base site `src/gui/js/bbox.js:1330` with `{"cc":23,"lines":65}`
20. `src/gui/js/bbox.js:1630` worsened, `function describeTool(btn) {`, values `{"cc":14,"lines":62}`, ceiling cc 13, lines 51, base site `src/gui/js/bbox.js:1524` with `{"cc":13,"lines":61}`
21. `src/gui/js/main.js:124` worsened, `async function applyLocalization(localization) {`, values `{"cc":4,"lines":56}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:121` with `{"cc":4,"lines":55}`
22. `src/gui/js/main.js:1004` worsened, `function setupProgressListener() {`, values `{"cc":1,"lines":74}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:996` with `{"cc":1,"lines":63}`
23. `src/gui/js/main.js:1136` worsened, `function initSettings() {`, values `{"cc":5,"lines":441}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1117` with `{"cc":5,"lines":438}`
24. `src/gui/js/main.js:3234` worsened, `async function startGeneration() {`, values `{"cc":49,"lines":192}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:2804` with `{"cc":33,"lines":174}`
25. `src/main.rs:178` worsened, `fn run_cli() {`, values `{"cc":66,"lines":603}`, ceiling cc 13, lines 51, base site `src/main.rs:148` with `{"cc":62,"lines":566}`
26. `src/overture/mod.rs:934` worsened, `fn fetch_overture_buildings_inner(`, values `{"cc":13,"lines":72}`, ceiling cc 13, lines 51, base site `src/overture/mod.rs:934` with `{"cc":8,"lines":58}`
27. `src/preview_3d.rs:75` worsened, `pub fn build_preview_payload(bbox_text: &str, aws_only: bool) -> Result<Vec<u8>, String> {`, values `{"cc":15,"lines":92}`, ceiling cc 13, lines 51, base site `src/preview_3d.rs:75` with `{"cc":15,"lines":90}`
28. `src/world_editor/java.rs:363` worsened, `fn write_region_to_disk(`, values `{"cc":22,"lines":142}`, ceiling cc 13, lines 51, base site `src/world_editor/java.rs:219` with `{"cc":15,"lines":113}`
29. `src/world_utils.rs:957` worsened, `pub fn set_spawn_in_level_dat(`, values `{"cc":18,"lines":86}`, ceiling cc 13, lines 51, base site `src/world_utils.rs:742` with `{"cc":17,"lines":79}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R026

- Repository: `louis-e/arnis` (Rust), change 5 of 10
- Commit: `69811f59d213`, judged against its first parent `788d5e9ca9de`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,336 functions at 788d5e9, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,336 functions at 788d5e9, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1359 from louis-e/ui-redesign
>
> UI redesign

### Files the change touched

```text
 src/gui/css/bbox.css          |   74 ----
 src/gui/css/dialogs.css       |  531 +++++++++++++++++++++++++++++
 src/gui/css/map-controls.css  |  389 +++++++++++++++++++++
 src/gui/css/preview3d.css     |   93 +++--
 src/gui/css/settings.css      |  962 ++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/gui/css/styles.css        |  887 ++++++------------------------------------------
 src/gui/index.html            | 1502 ++++++++++++++++++++++++++++++++++++++++++++++++++++-----------------------------
 src/gui/js/bbox.js            |  251 ++++++++++++--
 src/gui/js/license.js         |  429 ++++++++++++-----------
 src/gui/js/main.js            |  378 +++++++++++++--------
 src/gui/js/preview3d.js       |   30 +-
 src/gui/js/search.js          |    8 +-
 src/gui/js/settings-layout.js |  159 +++++++++
 src/gui/js/settings-store.js  |   48 ++-
 src/gui/locales/ar.json       |  111 +++++-
 src/gui/locales/de.json       |  101 +++++-
 src/gui/locales/en-US.json    |   91 ++++-
 src/gui/locales/es.json       |  111 +++++-
 src/gui/locales/fi.json       |  111 +++++-
 src/gui/locales/fr-FR.json    |  109 +++++-
 src/gui/locales/hu.json       |  111 +++++-
 src/gui/locales/ja.json       |  111 +++++-
 src/gui/locales/ka-GE.json    |  111 +++++-
 src/gui/locales/ko.json       |  111 +++++-
 src/gui/locales/lt.json       |  111 +++++-
 src/gui/locales/lv.json       |  111 +++++-
 src/gui/locales/pl.json       |  111 +++++-
 src/gui/locales/pt-BR.json    |  111 +++++-
 src/gui/locales/ru.json       |  111 +++++-
 src/gui/locales/sl.json       |  111 +++++-
 ...
 34 files changed, 5782 insertions(+), 2035 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/gui/js/bbox.js:1524` new, `function describeTool(btn) {`, values `{"cc":13,"lines":61}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   1521 |     map.getContainer().appendChild(toolTip);
   1522 |     var toolTipFor = null;
   1523 | 
   1524 |     function describeTool(btn) {
   1525 |         var c = btn.classList;
   1526 |         if (c.contains('leaflet-draw-draw-rectangle')) {
   1527 |             return {
   1528 |                 name: mapText('map_tool_area', 'Select area'),
   1529 |                 desc: mapText('map_tool_area_desc', 'Drag over the map to choose what gets generated.')
   1530 |             };
   1531 |         }
   1532 |         if (c.contains('leaflet-draw-draw-polyline')) {
   1533 |             return {
   1534 |                 name: mapText('map_tool_rotation', 'Rotation angle'),
   1535 |                 desc: mapText('map_tool_rotation_desc', 'Draw a line along a street to line the world up with it.')
   1536 |             };
   1537 |         }
   1538 |         if (c.contains('leaflet-draw-draw-marker')) {
   1539 |             return {
   1540 |                 name: mapText('map_tool_spawn', 'Spawn point'),
   1541 |                 desc: mapText('map_tool_spawn_desc', 'Click where players start in the world.')
   1542 |             };
   1543 |         }
   1544 |         if (c.contains('leaflet-draw-edit-remove')) {
   1545 |             return {
   1546 |                 name: mapText('map_tool_clear', 'Clear selection'),
   1547 |                 desc: mapText('map_tool_clear_desc', 'Removes the selected area and the spawn point.'),
   1548 |                 note: drawnItems.getLayers().length ? '' : mapText('map_tool_clear_empty', 'Nothing is selected yet.')
   1549 |             };
   1550 |         }
   1551 |         if (btn.id === 'world-preview-btn') {
   1552 |             return {
   1553 |                 name: mapText('map_tool_world_preview', 'World preview'),
   1554 |                 desc: mapText('map_tool_world_preview_desc', 'Lays the generated world over the map.'),
   1555 |                 note: worldPreviewAvailable ? '' : mapText('map_tool_world_preview_unavailable', 'Available after generating a world.')
   1556 |             };
   1557 |         }
   1558 |         if (btn.id === 'terrain-preview-btn') {
   1559 |             var terrainNote = '';
   1560 |             if (c.contains('disabled')) {
   1561 |                 terrainNote = currentBody !== 'earth'
   1562 |                     ? mapText('map_tool_terrain_earth_only', 'Only available on Earth.')
   1563 |                     : mapText('map_tool_terrain_select', 'Select an area of up to 500 km² first.');
   1564 |             }
   ```

2. `src/gui/js/bbox.js:464` worsened, `$(document).ready(function () {`, values `{"cc":14,"lines":1842}`, ceiling cc 13, lines 51, base site `src/gui/js/bbox.js:453` with `{"cc":13,"lines":1651}`

   ```text
   461 | }
   462 | window.mapText = mapText;
   463 | 
   464 | $(document).ready(function () {
   465 |     /* 
   466 |     **
   467 |     **  make sure all textarea inputs
   468 |     **  are selected once they are clicked
   469 |     **  because some people might not 
   470 |     **  have flash enabled or installed
   471 |     **  and yes...
   472 |     **  there's a fucking Flash movie floating 
   473 |     **  on top of your DOM
   474 |     **
   475 |     */
   476 | 
   477 |     // init the projection input box as it is used to format the initial values
   478 |     $('input[type="textarea"]').on('click', function (evt) { this.select() });
   479 |     $("#projection").val(currentproj);
   480 | 
   481 |     // Initialize map
   482 |     map = L.map('map', { zoomControl: false }).setView([50.114768, 8.687322], 4);
   483 | 
   484 |     // Define available tile themes
   485 |     var tileThemes = {
   486 |         'osm': {
   487 |             url: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
   488 |             options: {
   489 |                 attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
   490 |                 maxZoom: 19
   491 |             }
   492 |         },
   493 |         'esri-imagery': {
   494 |             url: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
   495 |             options: {
   496 |                 attribution: 'Tiles &copy; Esri &mdash; Source: Esri, i-cubed, USDA, USGS, AEX, GeoEye, Getmapping, Aerogrid, IGN, IGP, UPR-EGP, and the GIS User Community',
   497 |                 maxZoom: 18
   498 |             }
   499 |         },
   500 |         'opentopomap': {
   501 |             url: 'https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png',
   502 |             options: {
   503 |                 attribution: 'Map data: &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, <a href="http://viewfinderpanoramas.org">SRTM</a> | Map style: &copy; <a href="https://opentopomap.org">OpenTopoMap</a> (<a href="https://creativecommons.org/licenses/by-sa/3.0/">CC-BY-SA</a>)',
   504 |                 maxZoom: 17
   ```

3. `src/gui/js/bbox.js:2093` worsened, `map.on('draw:created', function (e) {`, values `{"cc":13,"lines":79}`, ceiling cc 13, lines 51, base site `src/gui/js/bbox.js:1894` with `{"cc":13,"lines":77}`

   ```text
   2090 |         }, 6000);
   2091 |     }
   2092 | 
   2093 |     map.on('draw:created', function (e) {
   2094 |         // instanceof, not layerType: restore paths fire rectangles as "polygon"
   2095 |         var isRectangle = e.layer instanceof L.Rectangle;
   2096 | 
   2097 |         // The first area ever selected retires the hint for good. Recorded
   2098 |         // before the layer is added below, whose layeradd refreshes the outline.
   2099 |         if (isRectangle) {
   2100 |             markAreaToolLearned();
   2101 |             var hint = document.querySelector('.bbox-hint-overlay');
   2102 |             if (hint) hint.style.display = 'none';
   2103 |         }
   2104 | 
   2105 |         // If it's a marker, make sure we only have one
   2106 |         if (e.layerType === 'marker') {
   2107 |             // Remove any existing markers
   2108 |             drawnItems.eachLayer(function(layer) {
   2109 |                 if (layer instanceof L.Marker) {
   2110 |                     drawnItems.removeLayer(layer);
   2111 |                 }
   2112 |             });
   2113 |         }
   2114 | 
   2115 |         // If it's a rectangle, remove any existing rectangles first
   2116 |         if (isRectangle) {
   2117 |             drawnItems.eachLayer(function(layer) {
   2118 |                 if (layer instanceof L.Rectangle) {
   2119 |                     drawnItems.removeLayer(layer);
   2120 |                 }
   2121 |             });
   2122 |         }
   2123 | 
   2124 |         // Check if it's a rectangle and set proper styles before adding it to the layer
   2125 |         if (isRectangle) {
   2126 |             e.layer.setStyle({
   2127 |                 color: '#fecc44',
   2128 |                 opacity: 1.0,
   2129 |                 weight: 3,
   2130 |                 fill: '#fecc44',
   2131 |                 fillOpacity: 0.08,
   2132 |                 lineCap: 'round',
   2133 |                 lineJoin: 'round'
   ```

4. `src/gui/js/main.js:275` worsened, `function openUpdateModal(opts = {}) {`, values `{"cc":31,"lines":63}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:333` with `{"cc":24,"lines":51}`
5. `src/gui/js/main.js:1117` worsened, `function initSettings() {`, values `{"cc":5,"lines":438}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1062` with `{"cc":5,"lines":424}`
6. `src/gui/js/main.js:1442` worsened, `async function openLicense() {`, values `{"cc":9,"lines":104}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1379` with `{"cc":8,"lines":98}`
7. `src/gui/js/main.js:2014` worsened, `function initTooltips() {`, values `{"cc":2,"lines":121}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:1943` with `{"cc":2,"lines":118}`
8. `src/gui/js/preview3d.js:7` worsened, `(function () {`, values `{"cc":1,"lines":1018}`, ceiling cc 13, lines 51, base site `src/gui/js/preview3d.js:7` with `{"cc":1,"lines":996}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R027

- Repository: `louis-e/arnis` (Rust), change 6 of 10
- Commit: `788d5e9ca9de`, judged against its first parent `e7cdeb99e274`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,291 functions at e7cdeb9, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,291 functions at e7cdeb9, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1358 from louis-e/memory-time-improvements
>
> Memory and time improvements

### Files the change touched

```text
 .github/workflows/pr-benchmark.yml |   4 +-
 src/block_definitions.rs           | 115 ++++++++++++++--
 src/block_palette.rs               |  79 +++++++----
 src/canopy/mod.rs                  | 248 ++++++++++++++++++++++++++++-----
 src/caves/decoration.rs            |   4 +-
 src/caves/schems.rs                |   2 +-
 src/colors.rs                      |   1 +
 src/data_processing.rs             | 574 ++++++++++++++++++++++++++++++++++++++++++++---------------------------------
 src/element_processing/railways.rs | 129 ++++++++++++++----
 src/elevation/postprocess.rs       | 148 ++++++++++++++++++++
 src/ground_generation.rs           |  77 +++++++++--
 src/landmarks.rs                   |   3 +-
 src/osm_tiles.rs                   | 383 +++++++++++++++++++++++++++++++++++++++++++++++----
 src/structures/schematic.rs        |  21 ++-
 14 files changed, 1404 insertions(+), 384 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/osm_tiles.rs:386` new, `fn select_for_bbox(c: &Collected, bbox: &LLBBox) -> (HashSet<u64>, HashSet<u64>) {`, values `{"cc":15,"lines":63}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   383 | /// extent does (a lake enclosing the whole bbox included) along with all its members, and
   384 | /// the building parts lying inside any kept building. Those parts can reach past the bbox,
   385 | /// and the outline suppression weighs all of them against the outline.
   386 | fn select_for_bbox(c: &Collected, bbox: &LLBBox) -> (HashSet<u64>, HashSet<u64>) {
   387 |     let area = Extent::around(bbox);
   388 |     let way_extent: HashMap<u64, Extent> = c
   389 |         .ways
   390 |         .iter()
   391 |         .filter_map(|(&id, (_, _, pts))| Extent::of(pts).map(|e| (id, e)))
   392 |         .collect();
   393 |     let relation_extent = |members: &[(u64, String)]| {
   394 |         members
   395 |             .iter()
   396 |             .filter_map(|(m, _)| way_extent.get(m).copied())
   397 |             .reduce(Extent::union)
   398 |     };
   399 | 
   400 |     let mut ways: HashSet<u64> = way_extent
   401 |         .iter()
   402 |         .filter(|(_, e)| e.intersects(&area))
   403 |         .map(|(&id, _)| id)
   404 |         .collect();
   405 |     let mut relations: HashSet<u64> = HashSet::new();
   406 |     let mut buildings = area;
   407 |     for (&id, (tags, members)) in &c.relations {
   408 |         let Some(e) = relation_extent(members) else {
   409 |             continue;
   410 |         };
   411 |         if e.intersects(&area) {
   412 |             relations.insert(id);
   413 |             ways.extend(members.iter().map(|(m, _)| *m));
   414 |             if is_building(tags) {
   415 |                 buildings = buildings.union(e);
   416 |             }
   417 |         }
   418 |     }
   419 |     for id in &ways {
   420 |         if let (Some((_, tags, _)), Some(e)) = (c.ways.get(id), way_extent.get(id)) {
   421 |             if is_building(tags) {
   422 |                 buildings = buildings.union(*e);
   423 |             }
   424 |         }
   425 |     }
   426 | 
   ```

2. `src/osm_tiles.rs:454` new, `fn assemble(c: Collected, bbox: &LLBBox) -> OsmData {`, values `{"cc":13,"lines":118}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   451 | /// Everything is emitted in id order: the parser keeps the first of two nodes on one
   452 | /// coordinate and processes elements in list order, so a hash-map order here would make
   453 | /// two runs over the same data build different worlds.
   454 | fn assemble(c: Collected, bbox: &LLBBox) -> OsmData {
   455 |     let (keep_ways, keep_relations) = select_for_bbox(&c, bbox);
   456 |     let area = Extent::around(bbox);
   457 |     let Collected {
   458 |         nodes,
   459 |         ways,
   460 |         relations,
   461 |     } = c;
   462 | 
   463 |     let mut nodes: Vec<(u64, NodeBody)> = nodes.into_iter().collect();
   464 |     nodes.sort_unstable_by_key(|n| n.0);
   465 |     // One id per distinct coordinate, so junctions share a node and a ring closes on itself.
   466 |     let mut coord_ids: HashMap<(i32, i32), u64> = HashMap::new();
   467 |     for (id, (lat, lon, _)) in &nodes {
   468 |         coord_ids.entry((*lat, *lon)).or_insert(*id);
   469 |     }
   470 | 
   471 |     let mut ways: Vec<DecWay> = ways
   472 |         .into_iter()
   473 |         .filter(|(id, _)| keep_ways.contains(id))
   474 |         .map(|(id, (closed, tags, pts))| (id, closed, tags, pts))
   475 |         .collect();
   476 |     ways.sort_unstable_by_key(|w| w.0);
   477 | 
   478 |     let mut next_synthetic = SYNTHETIC_ID_BASE;
   479 |     let mut emitted: Vec<(u64, i32, i32)> = Vec::new();
   480 |     let mut vertex_nodes: HashSet<u64> = HashSet::new();
   481 |     let mut way_elements: Vec<OsmElement> = Vec::with_capacity(ways.len());
   482 |     for (id, closed, tags, points) in ways {
   483 |         let mut refs: Vec<u64> = Vec::with_capacity(points.len() + 1);
   484 |         for p in &points {
   485 |             let nid = *coord_ids.entry(*p).or_insert_with(|| {
   486 |                 let id = next_synthetic;
   487 |                 next_synthetic += 1;
   488 |                 emitted.push((id, p.0, p.1));
   489 |                 id
   490 |             });
   491 |             if nid < SYNTHETIC_ID_BASE {
   492 |                 vertex_nodes.insert(nid);
   493 |             }
   494 |             refs.push(nid);
   ```

3. `src/structures/schematic.rs:857` new, `pub fn place_structure(`, values `{"cc":7,"lines":52}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   854 | }
   855 | 
   856 | /// Stamp anchor at (base_x, base_z), lowest voxel at base_y, rotated by `rot`; `ground` fills under each column. Keep half-extent under TILE_EDITOR_HALO (64) or it clips at tile seams.
   857 | pub fn place_structure(
   858 |     editor: &mut WorldEditor,
   859 |     schem: &StructureSchematic,
   860 |     base_x: i32,
   861 |     base_z: i32,
   862 |     base_y: i32,
   863 |     rot: u8,
   864 |     ground: Option<Block>,
   865 | ) {
   866 |     let k = rot & 3;
   867 |     let (w, l) = (schem.width, schem.length);
   868 |     // Skip if any voxel sits farther from the anchor than the tile halo, else it clips at seams.
   869 |     if schem.max_extent > crate::tile::TILE_EDITOR_HALO {
   870 |         eprintln!(
   871 |             "structure extent {} exceeds tile halo {}; skipping to avoid seam clipping",
   872 |             schem.max_extent,
   873 |             crate::tile::TILE_EDITOR_HALO
   874 |         );
   875 |         return;
   876 |     }
   877 |     let (ax, az) = rotate_xz(schem.anchor_x, schem.anchor_z, w, l, k);
   878 |     // Voxels share their states, so each distinct one is rotated once per placement.
   879 |     let mut rotated: Vec<(*const Value, Arc<Value>)> = Vec::new();
   880 |     for (vx, vy, vz, bwp) in &schem.voxels {
   881 |         let (rx, rz) = rotate_xz(*vx, *vz, w, l, k);
   882 |         let wx = base_x + rx - ax;
   883 |         let wz = base_z + rz - az;
   884 |         let placed = if k == 0 {
   885 |             bwp.clone()
   886 |         } else {
   887 |             let props = bwp.properties.as_ref().map(|p| {
   888 |                 match rotated
   889 |                     .iter()
   890 |                     .find(|(src, _)| std::ptr::eq(*src, Arc::as_ptr(p)))
   891 |                 {
   892 |                     Some((_, r)) => Arc::clone(r),
   893 |                     None => {
   894 |                         let r = intern_props(rotate_props(p, k));
   895 |                         rotated.push((Arc::as_ptr(p), Arc::clone(&r)));
   896 |                         r
   897 |                     }
   ```

4. `src/canopy/mod.rs:390` worsened, `fn fill_from_tile(`, values `{"cc":23,"lines":109}`, ceiling cc 13, lines 51, base site `src/canopy/mod.rs:381` with `{"cc":19,"lines":96}`
5. `src/canopy/mod.rs:636` worsened, `pub fn fetch_canopy_data(`, values `{"cc":10,"lines":62}`, ceiling cc 13, lines 51, base site `src/canopy/mod.rs:480` with `{"cc":10,"lines":61}`
6. `src/data_processing.rs:546` worsened, `pub fn generate_world_with_options(`, values `{"cc":166,"lines":1245}`, ceiling cc 13, lines 51, base site `src/data_processing.rs:519` with `{"cc":158,"lines":1190}`
7. `src/elevation/postprocess.rs:39` worsened, `pub fn repair_terrain_anomalies(heights: &mut [Vec<f64>], m_per_cell: f64) {`, values `{"cc":21,"lines":116}`, ceiling cc 13, lines 51, base site `src/elevation/postprocess.rs:39` with `{"cc":19,"lines":107}`
8. `src/elevation/postprocess.rs:1507` worsened, `pub(crate) fn gaussian_blur_mask_to_f32_reported(`, values `{"cc":21,"lines":122}`, ceiling cc 13, lines 51, base site `src/elevation/postprocess.rs:1498` with `{"cc":17,"lines":106}`
9. `src/ground_generation.rs:227` worsened, `pub fn generate_ground_region(`, values `{"cc":179,"lines":1197}`, ceiling cc 13, lines 51, base site `src/ground_generation.rs:174` with `{"cc":179,"lines":1191}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R028

- Repository: `louis-e/arnis` (Rust), change 7 of 10
- Commit: `e7cdeb99e274`, judged against its first parent `fc19146b3f72`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #1357 from louis-e/caves
>
> Add opt-in cave generation

### Files the change touched

```text
 src/args.rs                  |   73 +++++-
 src/bedrock_block_map.rs     |  134 ++++++++++
 src/block_definitions.rs     |  145 +++++++++++
 src/caves/carver.rs          |  496 ++++++++++++++++++++++++++++++++++++
 src/caves/decoration.rs      | 1123 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/deepslate.rs       |  113 +++++++++
 src/caves/density.rs         |  419 +++++++++++++++++++++++++++++++
 src/caves/mod.rs             |  951 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/noise.rs           |  273 ++++++++++++++++++++
 src/caves/ores.rs            |  424 +++++++++++++++++++++++++++++++
 src/caves/rng.rs             |  266 ++++++++++++++++++++
 src/caves/schems.rs          |  913 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/shape.rs           |  209 ++++++++++++++++
 src/caves/water.rs           |  435 ++++++++++++++++++++++++++++++++
 src/caves/zone_map.rs        |  144 +++++++++++
 src/data_processing.rs       |   42 +++-
 src/gui.rs                   |   12 +
 src/gui/index.html           |   11 +
 src/gui/js/main.js           |   26 ++
 src/gui/js/settings-store.js |    1 +
 src/gui/locales/ar.json      |    1 +
 src/gui/locales/de.json      |    1 +
 src/gui/locales/en-US.json   |    1 +
 src/gui/locales/es.json      |    1 +
 src/gui/locales/fi.json      |    1 +
 src/gui/locales/fr-FR.json   |    1 +
 src/gui/locales/hu.json      |    1 +
 src/gui/locales/ja.json      |    1 +
 src/gui/locales/ka-GE.json   |    1 +
 src/gui/locales/ko.json      |    1 +
 ...
 43 files changed, 6467 insertions(+), 8 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `src/caves/carver.rs:275` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":1,"escape":"allow"}`, nothing at the base matched

   ```text
   272 |     f.min(3.0)
   273 | }
   274 | 
   275 | #[allow(clippy::too_many_arguments)]
   276 | fn create_tunnel(
   277 |     tseed: i64,
   278 |     mut x: f64,
   279 |     mut y: f64,
   ```

2. `src/caves/decoration.rs:276` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":1,"escape":"allow"}`, nothing at the base matched

   ```text
   273 | /// `water_cells` is the PURE pool/river water set — coral reef decoration converts pool floors that
   274 | /// land inside a coral blotch. Nothing is written outside `region`: a block placed in a tile's halo
   275 | /// would land in the neighbouring tile's caves when the tiles merge.
   276 | #[allow(clippy::too_many_arguments)]
   277 | pub(super) fn decorate(
   278 |     ed: &mut WorldEditor,
   279 |     d: &Decor,
   280 |     air: &HashSet<i64>,
   ```

3. `src/caves/noise.rs:64` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":2,"escape":"allow"}`, nothing at the base matched

   ```text
   61 | fn lerp2(d1: f64, d2: f64, s1: f64, e1: f64, s2: f64, e2: f64) -> f64 {
   62 |     lerp(d2, lerp(d1, s1, e1), lerp(d1, s2, e2))
   63 | }
   64 | #[allow(clippy::too_many_arguments)]
   65 | #[inline]
   66 | fn lerp3(
   67 |     d1: f64,
   68 |     d2: f64,
   ```

4. `src/caves/rng.rs:129` new, `#[allow(dead_code)] // vanilla-parity API kept complete`, values `{"count":1,"escape":"allow"}`, nothing at the base matched
5. `src/caves/rng.rs:172` new, `#[allow(clippy::wrong_self_convention)] // mirrors vanilla's forkPositional().fromHashOf`, values `{"count":1,"escape":"allow"}`, nothing at the base matched
6. `src/caves/schems.rs:33` new, `#[allow(dead_code)] // kept for debugging dumps`, values `{"count":1,"escape":"allow"}`, nothing at the base matched
7. `src/caves/schems.rs:451` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":2,"escape":"allow"}`, nothing at the base matched
8. `src/caves/schems.rs:534` new, `let probe_y = floors.first().or(ceils.first()).copied().unwrap();`, values `{"count":1,"escape":"unwrap"}`, nothing at the base matched
9. `src/caves/water.rs:280` new, `let y_lo = cells.iter().map(|&(_, y, _)| y).min().unwrap();`, values `{"count":1,"escape":"unwrap"}`, nothing at the base matched
10. `src/caves/water.rs:281` new, `let y_hi = cells.iter().map(|&(_, y, _)| y).max().unwrap();`, values `{"count":1,"escape":"unwrap"}`, nothing at the base matched
11. `src/caves/water.rs:313` new, `#[allow(clippy::too_many_arguments)]`, values `{"count":1,"escape":"allow"}`, nothing at the base matched
12. `src/caves/zone_map.rs:49` new, `.expect("render() is only called when --cave-zone-map is set");`, values `{"count":1,"escape":"expect"}`, nothing at the base matched

### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R029

- Repository: `louis-e/arnis` (Rust), change 7 of 10
- Commit: `e7cdeb99e274`, judged against its first parent `fc19146b3f72`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,117 functions at fc19146, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,117 functions at fc19146, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1357 from louis-e/caves
>
> Add opt-in cave generation

### Files the change touched

```text
 src/args.rs                  |   73 +++++-
 src/bedrock_block_map.rs     |  134 ++++++++++
 src/block_definitions.rs     |  145 +++++++++++
 src/caves/carver.rs          |  496 ++++++++++++++++++++++++++++++++++++
 src/caves/decoration.rs      | 1123 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/deepslate.rs       |  113 +++++++++
 src/caves/density.rs         |  419 +++++++++++++++++++++++++++++++
 src/caves/mod.rs             |  951 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/noise.rs           |  273 ++++++++++++++++++++
 src/caves/ores.rs            |  424 +++++++++++++++++++++++++++++++
 src/caves/rng.rs             |  266 ++++++++++++++++++++
 src/caves/schems.rs          |  913 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/caves/shape.rs           |  209 ++++++++++++++++
 src/caves/water.rs           |  435 ++++++++++++++++++++++++++++++++
 src/caves/zone_map.rs        |  144 +++++++++++
 src/data_processing.rs       |   42 +++-
 src/gui.rs                   |   12 +
 src/gui/index.html           |   11 +
 src/gui/js/main.js           |   26 ++
 src/gui/js/settings-store.js |    1 +
 src/gui/locales/ar.json      |    1 +
 src/gui/locales/de.json      |    1 +
 src/gui/locales/en-US.json   |    1 +
 src/gui/locales/es.json      |    1 +
 src/gui/locales/fi.json      |    1 +
 src/gui/locales/fr-FR.json   |    1 +
 src/gui/locales/hu.json      |    1 +
 src/gui/locales/ja.json      |    1 +
 src/gui/locales/ka-GE.json   |    1 +
 src/gui/locales/ko.json      |    1 +
 ...
 43 files changed, 6467 insertions(+), 8 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/bedrock_block_map.rs:1014` new, `fn convert_cave_block(`, values `{"cc":26,"lines":127}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   1011 | /// Blocks the cave passes place whose Bedrock form differs from Java's. Like the rest of this
   1012 | /// file, the older Bedrock names (`coral_block` with a colour state) are used; the game
   1013 | /// upgrades them on load.
   1014 | fn convert_cave_block(
   1015 |     java_name: &str,
   1016 |     props: Option<&std::collections::HashMap<String, fastnbt::Value>>,
   1017 | ) -> Option<BedrockBlock> {
   1018 |     use BedrockBlockStateValue::{Bool, Int, String as Str};
   1019 |     let prop = |key: &str| match props.and_then(|p| p.get(key)) {
   1020 |         Some(fastnbt::Value::String(v)) => Some(v.as_str()),
   1021 |         _ => None,
   1022 |     };
   1023 |     let is_true = |key: &str| prop(key) == Some("true");
   1024 |     // Bedrock's facing_direction order: down, up, north, south, west, east.
   1025 |     let facing_direction = |facing: Option<&str>| match facing {
   1026 |         Some("down") => 0,
   1027 |         Some("north") => 2,
   1028 |         Some("south") => 3,
   1029 |         Some("west") => 4,
   1030 |         Some("east") => 5,
   1031 |         _ => 1,
   1032 |     };
   1033 |     let coral_color = |species: &str| match species {
   1034 |         "tube" => "blue",
   1035 |         "brain" => "pink",
   1036 |         "bubble" => "purple",
   1037 |         "fire" => "red",
   1038 |         _ => "yellow",
   1039 |     };
   1040 | 
   1041 |     Some(match java_name {
   1042 |         "pointed_dripstone" => BedrockBlock::with_states(
   1043 |             "pointed_dripstone",
   1044 |             vec![
   1045 |                 (
   1046 |                     "dripstone_thickness",
   1047 |                     Str(match prop("thickness").unwrap_or("tip") {
   1048 |                         "tip_merge" => "merge".to_string(),
   1049 |                         other => other.to_string(),
   1050 |                     }),
   1051 |                 ),
   1052 |                 ("hanging", Bool(prop("vertical_direction") == Some("down"))),
   1053 |             ],
   1054 |         ),
   ```

2. `src/caves/carver.rs:202` new, `fn cave_chunk(seed: i64, cfg: &Cfg, cx: i32, cz: i32, out: &mut Vec<Ellipsoid>) {`, values `{"cc":5,"lines":61}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   199 | }
   200 | 
   201 | // ---- cave + cave_extra (round tunnels) ----
   202 | fn cave_chunk(seed: i64, cfg: &Cfg, cx: i32, cz: i32, out: &mut Vec<Ellipsoid>) {
   203 |     let mut r = chunk_rng(seed, cx, cz, cfg.salt);
   204 |     if r.next_float() > cfg.probability {
   205 |         return;
   206 |     }
   207 |     // triple-nested nextInt → vanilla's skewed origin count (usually 0-2, rarely a cluster).
   208 |     // Evaluated inner→outer (Rust can't double-borrow in one expression; same order as vanilla).
   209 |     let a = r.next_int(15) + 1;
   210 |     let b = r.next_int(a) + 1;
   211 |     let n = r.next_int(b);
   212 |     for _ in 0..n {
   213 |         let ox = cx * 16 + r.next_int(16);
   214 |         let oy = cfg.y_min + r.next_int(cfg.y_max - cfg.y_min + 1);
   215 |         let oz = cz * 16 + r.next_int(16);
   216 |         let h_mult = 0.7 + r.next_float() as f64 * 0.7; // 0.7..1.4
   217 |         let v_mult = 0.8 + r.next_float() as f64 * 0.5; // 0.8..1.3
   218 |         let floor_level = -1.0 + r.next_float() as f64 * 0.6; // -1.0..-0.4
   219 | 
   220 |         let mut tunnels = 1;
   221 |         if r.next_int(4) == 0 {
   222 |             // a room: one fat blob
   223 |             let y_scale = 0.1 + r.next_float() as f64 * 0.8;
   224 |             let f = 1.0 + r.next_float() as f64 * 2.0; // room radius 1..3 (vanilla rolls 1..7; capped to keep rooms moderate)
   225 |             let d = 1.5 + f;
   226 |             out.push(Ellipsoid {
   227 |                 x: ox as f64 + 1.0,
   228 |                 y: oy as f64,
   229 |                 z: oz as f64,
   230 |                 horiz_radius: d,
   231 |                 vert_radius: d * y_scale,
   232 |                 floor_level,
   233 |             });
   234 |             tunnels += r.next_int(4);
   235 |         }
   236 |         for _ in 0..tunnels {
   237 |             let yaw = r.next_float() as f64 * 2.0 * PI;
   238 |             let pitch = (r.next_float() as f64 - 0.5) / 4.0;
   239 |             let thickness = get_thickness(&mut r);
   240 |             let branch_count = BRANCH_BUDGET - r.next_int(BRANCH_BUDGET / 4);
   241 |             let tseed = r.next_long();
   242 |             create_tunnel(
   ```

3. `src/caves/carver.rs:276` new, `fn create_tunnel(`, values `{"cc":7,"lines":98}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   273 | }
   274 | 
   275 | #[allow(clippy::too_many_arguments)]
   276 | fn create_tunnel(
   277 |     tseed: i64,
   278 |     mut x: f64,
   279 |     mut y: f64,
   280 |     mut z: f64,
   281 |     h_mult: f64,
   282 |     v_mult: f64,
   283 |     thickness: f64,
   284 |     mut yaw: f64,
   285 |     mut pitch: f64,
   286 |     branch_index: i32,
   287 |     branch_count: i32,
   288 |     horiz_vert_ratio: f64,
   289 |     floor_level: f64,
   290 |     cx: i32,
   291 |     cz: i32,
   292 |     out: &mut Vec<Ellipsoid>,
   293 | ) {
   294 |     let mut r = XoroRandom::from_seed(tseed);
   295 |     let branch_point = r.next_int(branch_count / 2) + branch_count / 4;
   296 |     let steep = r.next_int(6) == 0;
   297 |     let mut yaw_delta = 0.0f64;
   298 |     let mut pitch_delta = 0.0f64;
   299 | 
   300 |     for step in branch_index..branch_count {
   301 |         let d = 1.5 + (PI * step as f64 / branch_count as f64).sin() * thickness;
   302 |         let d1 = d * horiz_vert_ratio;
   303 |         let cos_pitch = pitch.cos();
   304 |         x += yaw.cos() * cos_pitch;
   305 |         y += pitch.sin();
   306 |         z += yaw.sin() * cos_pitch;
   307 |         pitch *= if steep { 0.92 } else { 0.7 };
   308 |         pitch += pitch_delta * 0.1;
   309 |         yaw += yaw_delta * 0.1;
   310 |         pitch_delta *= 0.9;
   311 |         yaw_delta *= 0.75;
   312 |         pitch_delta +=
   313 |             (r.next_float() as f64 - r.next_float() as f64) * r.next_float() as f64 * 2.0;
   314 |         yaw_delta += (r.next_float() as f64 - r.next_float() as f64) * r.next_float() as f64 * 4.0;
   315 | 
   316 |         if step == branch_point && thickness > 1.0 {
   ```

4. `src/caves/carver.rs:394` new, `fn canyon_chunk(seed: i64, cx: i32, cz: i32, out: &mut Vec<Ellipsoid>) {`, values `{"cc":8,"lines":75}`, ceiling cc 13, lines 51, nothing at the base matched
5. `src/caves/decoration.rs:132` new, `pub fn parse(spec: &str) -> Result<Self, String> {`, values `{"cc":14,"lines":31}`, ceiling cc 13, lines 51, nothing at the base matched
6. `src/caves/decoration.rs:220` new, `pub(super) fn zone(&self, x: i32, y: i32, z: i32, surf_y: i32) -> Zone {`, values `{"cc":15,"lines":41}`, ceiling cc 13, lines 51, nothing at the base matched
7. `src/caves/decoration.rs:277` new, `pub(super) fn decorate(`, values `{"cc":125,"lines":418}`, ceiling cc 13, lines 51, nothing at the base matched
8. `src/caves/decoration.rs:761` new, `fn build_giant_mushroom(`, values `{"cc":19,"lines":57}`, ceiling cc 13, lines 51, nothing at the base matched
9. `src/caves/decoration.rs:902` new, `pub(super) fn plan_geodes(`, values `{"cc":31,"lines":139}`, ceiling cc 13, lines 51, nothing at the base matched
10. `src/caves/mod.rs:149` new, `pub fn carve_region(`, values `{"cc":58,"lines":284}`, ceiling cc 13, lines 51, nothing at the base matched
11. `src/caves/mod.rs:441` new, `fn noise_cells(gen: &CaveGen, region: Rect, surf: &[i32], floor: i32) -> Vec<(i32, i32, i32)> {`, values `{"cc":8,"lines":90}`, ceiling cc 13, lines 51, nothing at the base matched
12. `src/caves/mod.rs:786` new, `fn features_do_not_depend_on_the_tile_split() {`, values `{"cc":9,"lines":92}`, ceiling cc 13, lines 51, nothing at the base matched
13. `src/caves/ores.rs:54` new, `fn ore_table() -> Vec<Ore> {`, values `{"cc":1,"lines":214}`, ceiling cc 13, lines 51, nothing at the base matched
14. `src/caves/ores.rs:293` new, `pub fn place_ores(`, values `{"cc":11,"lines":54}`, ceiling cc 13, lines 51, nothing at the base matched
15. `src/caves/ores.rs:363` new, `fn place_blob(editor: &mut WorldEditor, cx: i32, cy: i32, cz: i32, ore: &Ore, r: &mut XoroRandom) {`, values `{"cc":13,"lines":62}`, ceiling cc 13, lines 51, nothing at the base matched
16. `src/caves/rng.rs:183` new, `fn md5_16(msg: &[u8]) -> [u8; 16] {`, values `{"cc":8,"lines":67}`, ceiling cc 13, lines 51, nothing at the base matched
17. `src/caves/schems.rs:131` new, `fn load_pack(dir: &Path) -> Result<CavePack, String> {`, values `{"cc":16,"lines":37}`, ceiling cc 13, lines 51, nothing at the base matched
18. `src/caves/schems.rs:217` new, `fn cave_map(base: &str, props: &[(String, String)]) -> Option<(Block, bool)> {`, values `{"cc":27,"lines":37}`, ceiling cc 13, lines 51, nothing at the base matched
19. `src/caves/schems.rs:255` new, `fn load_cave_schem(gz_bytes: &[u8]) -> Result<CaveSchem, String> {`, values `{"cc":33,"lines":152}`, ceiling cc 13, lines 51, nothing at the base matched
20. `src/caves/schems.rs:452` new, `pub(super) fn stamp_region(`, values `{"cc":39,"lines":169}`, ceiling cc 13, lines 51, nothing at the base matched
21. `src/caves/schems.rs:637` new, `fn place_schem(`, values `{"cc":47,"lines":195}`, ceiling cc 13, lines 51, nothing at the base matched
22. `src/caves/water.rs:57` new, `pub(super) fn plan(shape: &CaveShape, decor: &Decor, seed: i64, region: Rect) -> WaterPlan {`, values `{"cc":10,"lines":61}`, ceiling cc 13, lines 51, nothing at the base matched
23. `src/caves/water.rs:160` new, `fn plan_pool(`, values `{"cc":19,"lines":142}`, ceiling cc 13, lines 51, nothing at the base matched
24. `src/caves/water.rs:314` new, `fn walk_river(`, values `{"cc":28,"lines":122}`, ceiling cc 13, lines 51, nothing at the base matched
25. `src/caves/zone_map.rs:45` new, `pub fn render(args: &Args) -> Result<(), String> {`, values `{"cc":16,"lines":81}`, ceiling cc 13, lines 51, nothing at the base matched
26. `src/args.rs:623` worsened, `pub fn validate_args(args: &Args) -> Result<(), String> {`, values `{"cc":72,"lines":252}`, ceiling cc 13, lines 51, base site `src/args.rs:586` with `{"cc":59,"lines":220}`
27. `src/bedrock_block_map.rs:896` worsened, `pub fn to_bedrock_block_with_properties(`, values `{"cc":25,"lines":114}`, ceiling cc 13, lines 51, base site `src/bedrock_block_map.rs:896` with `{"cc":24,"lines":111}`
28. `src/block_definitions.rs:111` worsened, `pub fn try_name(&self) -> Option<&str> {`, values `{"cc":435,"lines":442}`, ceiling cc 13, lines 51, base site `src/block_definitions.rs:111` with `{"cc":369,"lines":376}`
29. `src/block_definitions.rs:554` worsened, `pub fn properties(&self) -> Option<Value> {`, values `{"cc":56,"lines":337}`, ceiling cc 13, lines 51, base site `src/block_definitions.rs:488` with `{"cc":54,"lines":326}`
30. `src/data_processing.rs:519` worsened, `pub fn generate_world_with_options(`, values `{"cc":158,"lines":1190}`, ceiling cc 13, lines 51, base site `src/data_processing.rs:519` with `{"cc":152,"lines":1152}`
31. `src/gui.rs:1302` worsened, `fn gui_start_generation(`, values `{"cc":71,"lines":630}`, ceiling cc 13, lines 51, base site `src/gui.rs:1302` with `{"cc":70,"lines":618}`
32. `src/gui/js/main.js:117` worsened, `async function applyLocalization(localization) {`, values `{"cc":4,"lines":121}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:116` with `{"cc":4,"lines":120}`
33. `src/gui/js/main.js:2724` worsened, `async function startGeneration() {`, values `{"cc":33,"lines":174}`, ceiling cc 13, lines 51, base site `src/gui/js/main.js:2700` with `{"cc":33,"lines":172}`
34. `src/luanti_block_map.rs:254` worsened, `fn to_mineclonia_node(block: Block, props: Option<&Value>) -> LuantiNode {`, values `{"cc":424,"lines":501}`, ceiling cc 13, lines 51, base site `src/luanti_block_map.rs:165` with `{"cc":358,"lines":413}`
35. `src/main.rs:148` worsened, `fn run_cli() {`, values `{"cc":62,"lines":566}`, ceiling cc 13, lines 51, base site `src/main.rs:147` with `{"cc":59,"lines":552}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R030

- Repository: `louis-e/arnis` (Rust), change 8 of 10
- Commit: `fc19146b3f72`, judged against its first parent `80553b383f85`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,071 functions at 80553b3, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,071 functions at 80553b3, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1354 from louis-e/osm-tile-archive
>
> Osm tile archive

### Files the change touched

```text
 Cargo.lock              |  34 ++++
 Cargo.toml              |   2 +-
 src/args.rs             |   8 +
 src/gui.rs              |  16 +-
 src/main.rs             |   5 +-
 src/osm_parser.rs       |  24 ++-
 src/osm_tiles.rs        | 789 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/overture/mod.rs     |   2 +-
 src/overture/pmtiles.rs |  45 +++--
 src/retrieve_data.rs    | 200 +++++++++++++++++++----
 10 files changed, 1072 insertions(+), 53 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/osm_tiles.rs:185` new, `pub fn fetch_data_from_tiles(bbox: LLBBox, base_url: &str) -> Result<OsmData> {`, values `{"cc":21,"lines":105}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   182 |     relations: HashMap<u64, RelationBody>,
   183 | }
   184 | 
   185 | pub fn fetch_data_from_tiles(bbox: LLBBox, base_url: &str) -> Result<OsmData> {
   186 |     println!("{} Fetching data from the tile archive...", "[1/7]".bold());
   187 |     emit_gui_progress_update(1.0, "Downloading data...");
   188 | 
   189 |     let client = client()?;
   190 |     let manifest = manifest(&client, base_url)?;
   191 |     if manifest.zoom != ZOOM {
   192 |         return Err(format!(
   193 |             "archive is zoom {} but this build reads zoom {ZOOM}",
   194 |             manifest.zoom
   195 |         ));
   196 |     }
   197 | 
   198 |     let (min_x, min_y) = pmtiles::lonlat_to_tile(bbox.min().lng(), bbox.max().lat(), ZOOM);
   199 |     let (max_x, max_y) = pmtiles::lonlat_to_tile(bbox.max().lng(), bbox.min().lat(), ZOOM);
   200 |     let (xs, xe) = (min_x.min(max_x), min_x.max(max_x));
   201 |     let (ys, ye) = (min_y.min(max_y), min_y.max(max_y));
   202 |     // Counted before collecting: a planet-sized bbox is 67M tiles, and the vector would be
   203 |     // hundreds of megabytes before the cap ever ran.
   204 |     let needed = ((xe - xs) as usize + 1).saturating_mul((ye - ys) as usize + 1);
   205 |     if needed > MAX_TILES {
   206 |         return Err(format!(
   207 |             "that area needs {needed} tiles, past the {MAX_TILES} cap"
   208 |         ));
   209 |     }
   210 |     let wanted: Vec<(u32, u32)> = (xs..=xe)
   211 |         .flat_map(|x| (ys..=ye).map(move |y| (x, y)))
   212 |         .collect();
   213 | 
   214 |     let mut collected = Collected::default();
   215 |     let mut tiles_read = 0usize;
   216 |     let mut bytes = 0u64;
   217 | 
   218 |     if manifest.cell_zoom > ZOOM {
   219 |         return Err(format!(
   220 |             "archive index has cell zoom {} above zoom {ZOOM}",
   221 |             manifest.cell_zoom
   222 |         ));
   223 |     }
   224 |     let shift = ZOOM - manifest.cell_zoom;
   225 |     let side = 1u32 << manifest.cell_zoom;
   ```

2. `src/osm_tiles.rs:306` new, `fn assemble(c: Collected) -> OsmData {`, values `{"cc":9,"lines":95}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   303 | }
   304 | 
   305 | /// Builds the element list the Overpass path produces, so every later stage is unchanged.
   306 | fn assemble(c: Collected) -> OsmData {
   307 |     let mut elements: Vec<OsmElement> = Vec::new();
   308 |     // One id per distinct coordinate, so junctions share a node and a ring closes on itself.
   309 |     let mut coord_ids: HashMap<(i32, i32), u64> = HashMap::new();
   310 |     let mut next_synthetic = SYNTHETIC_ID_BASE;
   311 |     let mut emitted: Vec<(u64, i32, i32)> = Vec::new();
   312 | 
   313 |     for (id, (lat, lon, tags)) in c.nodes {
   314 |         coord_ids.entry((lat, lon)).or_insert(id);
   315 |         elements.push(OsmElement {
   316 |             r#type: "node".into(),
   317 |             id,
   318 |             lat: Some(f64::from(lat) / COORD_SCALE),
   319 |             lon: Some(f64::from(lon) / COORD_SCALE),
   320 |             nodes: None,
   321 |             tags: Some(tags.into_iter().collect()),
   322 |             members: Vec::new(),
   323 |         });
   324 |     }
   325 | 
   326 |     let mut ways: Vec<DecWay> = c
   327 |         .ways
   328 |         .into_iter()
   329 |         .map(|(id, (closed, tags, pts))| (id, closed, tags, pts))
   330 |         .collect();
   331 |     ways.sort_by_key(|w| w.0);
   332 | 
   333 |     for (id, closed, tags, points) in ways {
   334 |         let mut refs: Vec<u64> = Vec::with_capacity(points.len() + 1);
   335 |         for p in &points {
   336 |             let nid = *coord_ids.entry(*p).or_insert_with(|| {
   337 |                 let id = next_synthetic;
   338 |                 next_synthetic += 1;
   339 |                 emitted.push((id, p.0, p.1));
   340 |                 id
   341 |             });
   342 |             refs.push(nid);
   343 |         }
   344 |         if closed {
   345 |             if let (Some(first), Some(last)) = (refs.first().copied(), refs.last().copied()) {
   346 |                 if first != last {
   ```

3. `src/osm_tiles.rs:477` new, `fn decode(buf: &[u8]) -> Result<DecodedTile> {`, values `{"cc":54,"lines":99}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   474 |     }
   475 | }
   476 | 
   477 | fn decode(buf: &[u8]) -> Result<DecodedTile> {
   478 |     if buf.len() < 4 || &buf[..4] != b"AOT1" {
   479 |         return Err("not an Arnis tile payload".into());
   480 |     }
   481 |     let mut r = Reader { buf, pos: 4 };
   482 | 
   483 |     let n_strings = r.uvarint()?;
   484 |     if n_strings > 1 << 24 {
   485 |         return Err("implausible string table".into());
   486 |     }
   487 |     let mut strings: Vec<String> = Vec::with_capacity(n_strings.min(4096) as usize);
   488 |     for _ in 0..n_strings {
   489 |         let len = r.uvarint()? as usize;
   490 |         let end = r.pos.checked_add(len).ok_or("string overflows tile")?;
   491 |         let raw = buf.get(r.pos..end).ok_or("truncated string")?;
   492 |         strings.push(String::from_utf8(raw.to_vec()).map_err(|e| e.to_string())?);
   493 |         r.pos = end;
   494 |     }
   495 |     let at = |i: u64| -> Result<String> {
   496 |         strings
   497 |             .get(i as usize)
   498 |             .cloned()
   499 |             .ok_or_else(|| "string index out of range".to_string())
   500 |     };
   501 |     let read_tags = |r: &mut Reader| -> Result<Tags> {
   502 |         let n = r.uvarint()?;
   503 |         if n > 1 << 16 {
   504 |             return Err("implausible tag count".into());
   505 |         }
   506 |         let mut out = Vec::with_capacity(n as usize);
   507 |         for _ in 0..n {
   508 |             let (k, v) = (r.uvarint()?, r.uvarint()?);
   509 |             out.push((at(k)?, at(v)?));
   510 |         }
   511 |         Ok(out)
   512 |     };
   513 | 
   514 |     let mut tile = DecodedTile::default();
   515 | 
   516 |     let n_nodes = r.uvarint()?;
   517 |     if n_nodes > MAX_RECORDS {
   ```

4. `src/overture/pmtiles.rs:388` new, `pub fn open_allowing(`, values `{"cc":12,"lines":56}`, ceiling cc 13, lines 51, nothing at the base matched
5. `src/overture/pmtiles.rs:587` new, `fn fetch_range(client: &Client, url: &str, offset: u64, length: u64) -> Result<Vec<u8>> {`, values `{"cc":13,"lines":53}`, ceiling cc 13, lines 51, nothing at the base matched
6. `src/retrieve_data.rs:41` new, `fn download_with_reqwest(`, values `{"cc":12,"lines":52}`, ceiling cc 13, lines 51, nothing at the base matched
7. `src/gui.rs:1302` worsened, `fn gui_start_generation(`, values `{"cc":70,"lines":618}`, ceiling cc 13, lines 51, base site `src/gui.rs:1298` with `{"cc":70,"lines":610}`
8. `src/main.rs:147` worsened, `fn run_cli() {`, values `{"cc":59,"lines":552}`, ceiling cc 13, lines 51, base site `src/main.rs:146` with `{"cc":59,"lines":550}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R031

- Repository: `louis-e/arnis` (Rust), change 8 of 10
- Commit: `fc19146b3f72`, judged against its first parent `80553b383f85`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> Merge pull request #1354 from louis-e/osm-tile-archive
>
> Osm tile archive

### Files the change touched

```text
 Cargo.lock              |  34 ++++
 Cargo.toml              |   2 +-
 src/args.rs             |   8 +
 src/gui.rs              |  16 +-
 src/main.rs             |   5 +-
 src/osm_parser.rs       |  24 ++-
 src/osm_tiles.rs        | 789 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/overture/mod.rs     |   2 +-
 src/overture/pmtiles.rs |  45 +++--
 src/retrieve_data.rs    | 200 +++++++++++++++++++----
 10 files changed, 1072 insertions(+), 53 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `src/osm_tiles.rs:59` new, `fn default_cell_zoom() -> u8 {`, values `{"dead":1}`, nothing at the base matched

   ```text
   56 |     archives: Vec<ArchiveEntry>,
   57 | }
   58 | 
   59 | fn default_cell_zoom() -> u8 {
   60 |     CELL_ZOOM
   61 | }
   62 | 
   63 | #[derive(Debug, Deserialize, Clone)]
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R032

- Repository: `louis-e/arnis` (Rust), change 9 of 10
- Commit: `80553b383f85`, judged against its first parent `a967f6570cde`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `13`, 95th percentile of 13,019 functions at a967f65, floor 5; recorded scope: whole repository
- Derived lines: `51`, 95th percentile of 13,019 functions at a967f65, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #1343 from louis-e/building-detail-improvement
>
> Improve building details

### Files the change touched

```text
 src/element_processing/building_facade.rs |    9 +-
 src/element_processing/buildings.rs       | 2110 ++++++++++++++++++++++++++++++++++++++++++++++++++++++---------------
 src/overture/mod.rs                       |   35 ++
 src/overture/tiles.rs                     |    5 +
 4 files changed, 1698 insertions(+), 461 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 13 or body > 51 lines).

1. `src/element_processing/buildings.rs:11075` new, `fn footprint_radial_fractions(`, values `{"cc":20,"lines":121}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   11072 | /// against the far side of the wing, so the roof there rides too high and
   11073 | /// never comes down to the wing's own wall. Those footprints keep the
   11074 | /// bounding-box profile instead.
   11075 | fn footprint_radial_fractions(
   11076 |     nodes: &[ProcessedNode],
   11077 |     cells: &[(i32, i32)],
   11078 | ) -> Option<HashMap<(i32, i32), f64>> {
   11079 |     let mut pts: Vec<(f64, f64)> = nodes.iter().map(|n| (n.x as f64, n.z as f64)).collect();
   11080 |     if pts.len() >= 2 && pts.first() == pts.last() {
   11081 |         pts.pop();
   11082 |     }
   11083 |     pts.dedup();
   11084 |     if pts.len() < 3 {
   11085 |         return None;
   11086 |     }
   11087 |     let n = pts.len();
   11088 | 
   11089 |     // Area centroid: a run of dense nodes on one wall must not pull it over.
   11090 |     let (mut area2, mut cx, mut cz) = (0.0f64, 0.0f64, 0.0f64);
   11091 |     for i in 0..n {
   11092 |         let (x0, z0) = pts[i];
   11093 |         let (x1, z1) = pts[(i + 1) % n];
   11094 |         let cross = x0 * z1 - x1 * z0;
   11095 |         area2 += cross;
   11096 |         cx += (x0 + x1) * cross;
   11097 |         cz += (z0 + z1) * cross;
   11098 |     }
   11099 |     if area2.abs() < 1e-9 {
   11100 |         return None;
   11101 |     }
   11102 |     let (cx, cz) = (cx / (3.0 * area2), cz / (3.0 * area2));
   11103 |     if !polygon_contains(&pts, cx, cz) {
   11104 |         return None;
   11105 |     }
   11106 | 
   11107 |     // Edges bucketed by the bearings they cover from the centroid, so a cell
   11108 |     // tests the few edges its ray can cross rather than the whole outline.
   11109 |     const BUCKETS: usize = 512;
   11110 |     let bucket_of = |ang: f64| -> usize {
   11111 |         (((ang + std::f64::consts::PI) / std::f64::consts::TAU) * BUCKETS as f64) as usize
   11112 |     };
   11113 |     let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); BUCKETS];
   11114 |     for i in 0..n {
   11115 |         let (ax, az) = pts[i];
   ```

2. `src/overture/mod.rs:934` new, `fn fetch_overture_buildings_inner(`, values `{"cc":8,"lines":58}`, ceiling cc 13, lines 51, nothing at the base matched

   ```text
   931 |     })
   932 | }
   933 | 
   934 | fn fetch_overture_buildings_inner(
   935 |     bbox: &LLBBox,
   936 |     scale: f64,
   937 |     source: OvertureSource,
   938 |     debug: bool,
   939 | ) -> Result<OvertureData, Box<dyn std::error::Error>> {
   940 |     let client = overture_client()?;
   941 | 
   942 |     emit_gui_progress_update(6.0, "Downloading data...");
   943 | 
   944 |     let budget = overture_building_budget(bbox);
   945 |     let OvertureCollection {
   946 |         buildings: all_buildings,
   947 |         hints,
   948 |     } = collect_overture_buildings(&client, bbox, source, false, budget, true, debug)?;
   949 | 
   950 |     if debug {
   951 |         println!(
   952 |             "Overture: {} non-OSM buildings found, {} attribute hints for OSM buildings",
   953 |             all_buildings.len(),
   954 |             hints.len()
   955 |         );
   956 |     }
   957 | 
   958 |     // Convert to ProcessedElements and clip to xzbbox (matching OSM clipping)
   959 |     let (coord_transformer, xzbbox) = CoordTransformer::llbbox_to_xzbbox(bbox, scale)?;
   960 | 
   961 |     let elements: Vec<ProcessedElement> = all_buildings
   962 |         .into_iter()
   963 |         .take(budget)
   964 |         .filter_map(|building| {
   965 |             if debug {
   966 |                 // One line per footprint, so a stray one can be found and listed above.
   967 |                 let n = building.exterior_ring.len().max(1) as f64;
   968 |                 let (lng, lat) = building
   969 |                     .exterior_ring
   970 |                     .iter()
   971 |                     .fold((0.0, 0.0), |(x, y), &(lng, lat)| (x + lng / n, y + lat / n));
   972 |                 println!(
   973 |                     "Overture building {} {}/{} h={:?} at {lat:.6},{lng:.6}",
   974 |                     building.id,
   ```

3. `src/element_processing/building_facade.rs:166` worsened, `pub fn compute_facade_plan(`, values `{"cc":35,"lines":164}`, ceiling cc 13, lines 51, base site `src/element_processing/building_facade.rs:166` with `{"cc":34,"lines":163}`

   ```text
   163 | 
   164 | /// Classifies every wall segment. `own_cells` must include the building's
   165 | /// own cells plus its part group mates.
   166 | pub fn compute_facade_plan(
   167 |     element: &ProcessedWay,
   168 |     ctx: &BuildingContext<'_>,
   169 |     scale: f64,
   170 |     own_cells: &FnvHashSet<(i32, i32)>,
   171 | ) -> FacadePlan {
   172 |     if element.nodes.len() < 3 {
   173 |         return FacadePlan::empty();
   174 |     }
   175 |     let outward = outward_side(&element.nodes);
   176 |     let setback_max = street_setback_max(scale);
   177 | 
   178 |     let mut segments: Vec<Option<SegmentFacade>> = Vec::new();
   179 |     let mut party_columns: FnvHashSet<(i32, i32)> = FnvHashSet::default();
   180 |     let mut segment_columns: Vec<Vec<(i32, i32)>> = Vec::new();
   181 | 
   182 |     let mut previous_node: Option<(i32, i32)> = None;
   183 |     for node in &element.nodes {
   184 |         let (x2, z2) = (node.x, node.z);
   185 |         if let Some((x1, z1)) = previous_node {
   186 |             let (nx, nz) = compute_outward_normal(x1, z1, x2, z2, outward);
   187 |             if nx == 0 && nz == 0 {
   188 |                 segments.push(None);
   189 |                 segment_columns.push(Vec::new());
   190 |                 previous_node = Some((x2, z2));
   191 |                 continue;
   192 |             }
   193 |             let tangent = ((x2 - x1).signum(), (z2 - z1).signum());
   194 |             let len = (x2 - x1).abs().max((z2 - z1).abs());
   195 |             let points: Vec<(i32, i32)> = bresenham_line(x1, 0, z1, x2, 0, z2)
   196 |                 .into_iter()
   197 |                 .map(|(x, _, z)| (x, z))
   198 |                 .collect();
   199 | 
   200 |             // Party detection: outward cells at depth 1-2 belonging to a
   201 |             // foreign footprint.
   202 |             let mut party_cols = 0usize;
   203 |             for &(bx, bz) in &points {
   204 |                 let is_party = (1..=2).any(|d| {
   205 |                     let c = (bx + nx * d, bz + nz * d);
   206 |                     ctx.building_footprints.contains(c.0, c.1) && !own_cells.contains(&c)
   ```

4. `src/element_processing/buildings.rs:2654` worsened, `fn calculate_building_height(`, values `{"cc":24,"lines":139}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:2604` with `{"cc":22,"lines":136}`
5. `src/element_processing/buildings.rs:2917` worsened, `fn generate_roof_only_structure(`, values `{"cc":26,"lines":252}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:2864` with `{"cc":25,"lines":232}`
6. `src/element_processing/buildings.rs:3413` worsened, `fn plan_mapped_entrances(`, values `{"cc":22,"lines":92}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:3318` with `{"cc":21,"lines":79}`
7. `src/element_processing/buildings.rs:3553` worsened, `fn render_entrance(`, values `{"cc":14,"lines":130}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:3445` with `{"cc":14,"lines":120}`
8. `src/element_processing/buildings.rs:5133` worsened, `fn generate_residential_window_decorations(`, values `{"cc":63,"lines":392}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:4884` with `{"cc":60,"lines":375}`
9. `src/element_processing/buildings.rs:5553` worsened, `fn generate_corner_quoins(`, values `{"cc":15,"lines":96}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:5287` with `{"cc":16,"lines":90}`
10. `src/element_processing/buildings.rs:6130` worsened, `fn generate_facade_cornices(`, values `{"cc":28,"lines":100}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:5862` with `{"cc":29,"lines":96}`
11. `src/element_processing/buildings.rs:6233` worsened, `fn generate_archetype_window_headers(`, values `{"cc":22,"lines":75}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:5961` with `{"cc":20,"lines":70}`
12. `src/element_processing/buildings.rs:6372` worsened, `fn generate_corner_downpipes(`, values `{"cc":16,"lines":88}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6099` with `{"cc":19,"lines":79}`
13. `src/element_processing/buildings.rs:6503` worsened, `fn place_modern_pillars(`, values `{"cc":8,"lines":63}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6221` with `{"cc":7,"lines":61}`
14. `src/element_processing/buildings.rs:6570` worsened, `fn place_institutional_bands(`, values `{"cc":7,"lines":61}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6286` with `{"cc":6,"lines":59}`
15. `src/element_processing/buildings.rs:6665` worsened, `fn place_historic_ornate(`, values `{"cc":10,"lines":86}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6379` with `{"cc":9,"lines":84}`
16. `src/element_processing/buildings.rs:6825` worsened, `fn place_skyscraper_fins(`, values `{"cc":6,"lines":57}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6537` with `{"cc":5,"lines":55}`
17. `src/element_processing/buildings.rs:7255` worsened, `pub fn generate_buildings(`, values `{"cc":126,"lines":849}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:6964` with `{"cc":120,"lines":835}`
18. `src/element_processing/buildings.rs:8264` worsened, `fn generate_building_roof(`, values `{"cc":36,"lines":171}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:7959` with `{"cc":33,"lines":160}`
19. `src/element_processing/buildings.rs:8812` worsened, `fn generate_chimney(`, values `{"cc":15,"lines":129}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:8496` with `{"cc":15,"lines":117}`
20. `src/element_processing/buildings.rs:9807` worsened, `fn place_roof_blocks_with_stairs(`, values `{"cc":7,"lines":57}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:9470` with `{"cc":6,"lines":57}`
21. `src/element_processing/buildings.rs:9906` worsened, `fn generate_gabled_roof(`, values `{"cc":56,"lines":423}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:9564` with `{"cc":56,"lines":416}`
22. `src/element_processing/buildings.rs:10369` worsened, `fn place_eave_overhang_inner(`, values `{"cc":29,"lines":153}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:10017` with `{"cc":28,"lines":148}`
23. `src/element_processing/buildings.rs:10547` worsened, `fn generate_hipped_roof_inner(`, values `{"cc":39,"lines":316}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:10190` with `{"cc":38,"lines":304}`
24. `src/element_processing/buildings.rs:11198` worsened, `fn generate_pyramidal_roof(`, values `{"cc":18,"lines":148}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:10663` with `{"cc":18,"lines":138}`
25. `src/element_processing/buildings.rs:11458` worsened, `fn generate_onion_roof(editor: &mut WorldEditor, floor_area: &[(i32, i32)], config: &RoofConfig) {`, values `{"cc":11,"lines":64}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:10865` with `{"cc":11,"lines":58}`
26. `src/element_processing/buildings.rs:11552` worsened, `fn generate_roof(`, values `{"cc":20,"lines":150}`, ceiling cc 13, lines 51, base site `src/element_processing/buildings.rs:10953` with `{"cc":17,"lines":162}`
27. `src/overture/mod.rs:794` worsened, `fn collect_from_parquet(`, values `{"cc":26,"lines":139}`, ceiling cc 13, lines 51, base site `src/overture/mod.rs:777` with `{"cc":25,"lines":136}`
28. `src/overture/tiles.rs:342` worsened, `pub fn collect_from_tiles(`, values `{"cc":38,"lines":194}`, ceiling cc 13, lines 51, base site `src/overture/tiles.rs:342` with `{"cc":37,"lines":189}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R033

- Repository: `denisidoro/navi` (Rust), change 4 of 10
- Commit: `a171c2938d5e`, judged against its first parent `9e6e8da6f5da`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 180 functions at 9e6e8da, floor 5; recorded scope: whole repository
- Derived lines: `45`, 95th percentile of 180 functions at 9e6e8da, floor 25; recorded scope: whole repository

### Commit message

> style: fix cargo fmt
>
> Signed-off-by: OPOLKA Alexis OF/DSI <alexis.opolka@orange.com>

### Files the change touched

```text
 src/finder/mod.rs | 6 +++++-
 1 file changed, 5 insertions(+), 1 deletion(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 45 lines).

1. `src/finder/mod.rs:68` worsened, `pub fn call<F, R>(&self, finder_opts: Opts, stdin_fn: F) -> Result<(String, R)>`, values `{"cc":34,"lines":165}`, ceiling cc 9, lines 45, base site `src/finder/mod.rs:68` with `{"cc":34,"lines":161}`

   ```text
   065 |         }
   066 |     }
   067 | 
   068 |     pub fn call<F, R>(&self, finder_opts: Opts, stdin_fn: F) -> Result<(String, R)>
   069 |     where
   070 |         F: Fn(&mut dyn Write) -> Result<R>,
   071 |     {
   072 |         let finder_str = match self {
   073 |             Self::Fzf => "fzf",
   074 |             Self::Skim => "sk",
   075 |         };
   076 | 
   077 |         if let Self::Fzf = self {
   078 |             if let Some((major, minor, patch)) = Self::check_fzf_version() {
   079 |                 if (major, minor, patch)
   080 |                     < (
   081 |                         MIN_FZF_VERSION_MAJOR,
   082 |                         MIN_FZF_VERSION_MINOR,
   083 |                         MIN_FZF_VERSION_PATCH,
   084 |                     )
   085 |                 {
   086 |                     eprintln!(
   087 |                         "Warning: Fzf version {major}.{minor} does not support the preview window layout used by navi.",
   088 |                     );
   089 |                     eprintln!(
   090 |                         "Consider updating Fzf to a version >= {MIN_FZF_VERSION_MAJOR}.{MIN_FZF_VERSION_MINOR}.{MIN_FZF_VERSION_PATCH} or use a compatible layout.",
   091 |                     );
   092 |                     process::exit(1);
   093 |                 }
   094 |             }
   095 |         }
   096 | 
   097 |         let mut command = Command::new(finder_str);
   098 |         let opts = finder_opts.clone();
   099 | 
   100 |         let preview_height = match self {
   101 |             FinderChoice::Skim => 3,
   102 |             _ => 2,
   103 |         };
   104 | 
   105 |         let bindings = if opts.suggestion_type == SuggestionType::MultipleSelections {
   106 |             ",ctrl-r:toggle-all"
   107 |         } else {
   108 |             ""
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R034

- Repository: `denisidoro/navi` (Rust), change 5 of 10
- Commit: `9e6e8da6f5da`, judged against its first parent `51060f94f95b`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #1027 from GauravS11112003/shell-plugin-tests
>
> Fix CI test reliability and add tmux shell plugin tests (#1014)

### Files the change touched

```text
 .github/workflows/ci.yml          |  11 ++++-
 .github/workflows/shell-tests.yml |  53 +++++++++++++++++++++++
 tests/config.yaml                 |   4 ++
 tests/run                         |  12 ++++-
 tests/shell/cheats/plugin.cheat   |  23 ++++++++++
 tests/shell/lib.bash              | 185 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/shell/rc/bashrc             |  14 ++++++
 tests/shell/rc/fish_init.fish     |  13 ++++++
 tests/shell/rc/zsh-dotdir/.zshrc  |   9 ++++
 tests/shell/run                   | 178 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 10 files changed, 498 insertions(+), 4 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `tests/shell/lib.bash:94` new, `tmux capture-pane -t "$session" -p -S - 2>/dev/null || true`, values `{"count":1,"escape":"errors ignored"}`, nothing at the base matched

   ```text
   91 | # Capture the full visible pane contents, including scrollback.
   92 | shell::pane() {
   93 |    local -r session="$1"
   94 |    tmux capture-pane -t "$session" -p -S - 2>/dev/null || true
   95 | }
   96 | 
   97 | # Poll the pane until `pattern` (extended regex) appears, or the
   98 | # timeout (in seconds) elapses. Returns 0 on match, 1 on timeout.
   ```

2. `tests/shell/lib.bash:127` new, `tmux kill-session -t "$session" 2>/dev/null || true`, values `{"count":3,"escape":"errors ignored"}`, nothing at the base matched

   ```text
   124 |    local -r session="$(shell::_session_name "$shell" "$case_id")"
   125 |    local -r launch_cmd="$(shell::_launch_cmd "$shell")"
   126 | 
   127 |    tmux kill-session -t "$session" 2>/dev/null || true
   128 |    tmux new-session -d -s "$session" -x 200 -y 50 "$launch_cmd"
   129 | 
   130 |    if ! shell::wait_for_prompt "$session"; then
   131 |       log::error "Shell '$shell' failed to reach a prompt in session '$session'"
   ```

3. `tests/shell/lib.bash:184` new, `pkill -f "tmux.*navi_shell_test_.*_$$" 2>/dev/null || true`, values `{"count":1,"escape":"errors ignored"}`, nothing at the base matched

   ```text
   181 | # Convenience: kill any stray tmux sessions left behind by aborted
   182 | # runs. Mirrors `_kill_tmux` in tests/run.
   183 | shell::kill_all() {
   184 |    pkill -f "tmux.*navi_shell_test_.*_$$" 2>/dev/null || true
   185 | }
   186 | 
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R035

- Repository: `denisidoro/navi` (Rust), change 8 of 10
- Commit: `cc4072361735`, judged against its first parent `75eb06092762`, exit 1
- Gate: stubs, FAIL
- Decision group: stubs

### Commit message

> Merge pull request #1029 from shaked-shlomo/fix/comment-doc-typos
>
> Fix typos in doc comments and widgets docs

### Files the change touched

```text
 docs/widgets/README.md | 2 +-
 src/filesystem.rs      | 4 ++--
 2 files changed, 3 insertions(+), 3 deletions(-)
```

### Findings

Condition: where the code stands in for work nobody did.

1. `src/filesystem.rs:16` new, `/// FIXME: it's actually incorrect to assume a path doesn't contain this separator`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched

   ```text
   13 | use walkdir::WalkDir;
   14 | 
   15 | /// Multiple paths are joined by a platform-specific separator.
   16 | /// FIXME: it's actually incorrect to assume a path doesn't contain this separator
   17 | #[cfg(target_family = "windows")]
   18 | pub const JOIN_SEPARATOR: &str = ";";
   19 | #[cfg(not(target_family = "windows"))]
   20 | pub const JOIN_SEPARATOR: &str = ":";
   ```


### Remedy klin printed

> Do what the marker stands in for. A placeholder an agent left behind is not work, and accepting one is a decision for a person, in the config, in a reviewed commit.

## R036

- Repository: `denisidoro/navi` (Rust), change 9 of 10
- Commit: `75eb06092762`, judged against its first parent `9973dc88573b`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #1026 from emma31-dev/test_changes
>
> Minor code formatting

### Files the change touched

```text
 .gitignore                     |  1 +
 rust-toolchain.toml            |  1 +
 src/commands/core/actor.rs     | 17 ++++++++---------
 src/commands/mod.rs            |  3 +--
 src/common/fs.rs               |  1 +
 src/config/env.rs              | 16 +++++++---------
 src/config/mod.rs              | 46 ++++++++++++++++++++--------------------------
 src/libs/dns_common/tracing.rs |  1 +
 8 files changed, 40 insertions(+), 46 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `src/common/fs.rs:29` new, `#[allow(unused)]`, values `{"count":1,"escape":"allow"}`, nothing at the base matched

   ```text
   26 | #[error("Invalid path `{0}`")]
   27 | pub struct InvalidPath(pub PathBuf);
   28 | 
   29 | #[allow(unused)]
   30 | #[derive(Error, Debug)]
   31 | #[error("Unable to read directory `{dir}`")]
   32 | pub struct UnreadableDir {
   33 |     dir: PathBuf,
   ```

2. `src/libs/dns_common/tracing.rs:3` new, `#[allow(unused)]`, values `{"count":1,"escape":"allow"}`, nothing at the base matched

   ```text
   1 | use crate::prelude::*;
   2 | 
   3 | #[allow(unused)]
   4 | #[derive(Deserialize, Serialize, Debug, Clone)]
   5 | #[serde(deny_unknown_fields)]
   6 | pub struct TracingConfig {
   7 |     pub time: bool,
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R037

- Repository: `refactoringhq/tolaria` (TypeScript), change 1 of 10
- Commit: `04030c3e0a23`, judged against its first parent `d9e02a167907`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at the floor, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `5`, the floor of 5, over 34,768 function(s) at d9e02a1; recorded scope: whole repository
- Derived lines: `34`, 95th percentile of 34,768 functions at d9e02a1, floor 25; recorded scope: whole repository

### Commit message

> fix: bypass looping editor during recovery

### Files the change touched

```text
 docs/ABSTRACTIONS.md                                       |  2 ++
 src/components/editor-content/EditorContentLayout.test.tsx | 35 ++++++++++++++++++++++++++++++++++-
 src/components/editor-content/useEditorContentModel.ts     | 16 ++++++++++++++--
 3 files changed, 50 insertions(+), 3 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 5 or body > 34 lines).

1. `src/components/editor-content/useEditorContentModel.ts:70` worsened, `export function useEditorContentModel(props: EditorContentProps) {`, values `{"cc":7,"lines":60}`, ceiling cc 5, lines 34, base site `src/components/editor-content/useEditorContentModel.ts:68` with `{"cc":7,"lines":50}`

   ```text
   067 |   locale?: AppLocale
   068 | }
   069 | 
   070 | export function useEditorContentModel(props: EditorContentProps) {
   071 |   const {
   072 |     activeTab,
   073 |     activeTabPath,
   074 |     entries,
   075 |     rawMode,
   076 |     diffMode,
   077 |     onToggleRaw: toggleRaw,
   078 |   } = props
   079 | 
   080 |   const { cssVars } = useEditorTheme()
   081 |   const {
   082 |     isArchived,
   083 |     isDeletedPreview,
   084 |     isHtmlPreview,
   085 |     isSheet,
   086 |     isNonMarkdownText,
   087 |     effectiveRawMode,
   088 |     showEditor: showContentEditor,
   089 |     path,
   090 |     wordCount,
   091 |   } = useMemo(() => deriveEditorContentState({
   092 |     activeTab,
   093 |     entries,
   094 |     rawMode,
   095 |     activeStatus: props.activeStatus,
   096 |   }), [activeTab, entries, props.activeStatus, rawMode])
   097 |   const showEditor = !diffMode && showContentEditor
   098 |   const loadingEntry = !activeTab && activeTabPath
   099 |     ? entries.find((entry) => entry.path === activeTabPath) ?? null
   100 |     : null
   101 |   const loadingTab = loadingEntry ? { entry: loadingEntry, content: '' } : null
   102 |   const onToggleRaw = useCallback((recoveryReason?: BlockNoteRenderRecoveryReason) => {
   103 |     if (recoveryReason === 'react_update_depth_exceeded') {
   104 |       updateVaultConfigField('editor_mode', 'raw')
   105 |       return
   106 |     }
   107 | 
   108 |     toggleRaw()
   109 |   }, [toggleRaw])
   110 | 
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R038

- Repository: `refactoringhq/tolaria` (TypeScript), change 3 of 10
- Commit: `44740e4ae18d`, judged against its first parent `7435ef89016d`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at the floor, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `5`, the floor of 5, over 34,714 function(s) at 7435ef8; recorded scope: whole repository
- Derived lines: `34`, 95th percentile of 34,714 functions at 7435ef8, floor 25; recorded scope: whole repository

### Commit message

> fix: render PDF previews with PDF.js

### Files the change touched

```text
 docs/ABSTRACTIONS.md                                    |   2 +-
 docs/ARCHITECTURE.md                                    |   2 +-
 docs/adr/0182-app-owned-cross-platform-pdf-rendering.md |  39 ++++++++++++
 docs/adr/README.md                                      |   1 +
 package.json                                            |   1 +
 pnpm-lock.yaml                                          | 135 ++++++++++++++++++++++++++++++++++++++++
 src-tauri/tauri.conf.json                               |   9 +--
 src/components/Editor.test.tsx                          |   6 +-
 src/components/FilePreview.test.tsx                     |  29 ++++++---
 src/components/FilePreview.tsx                          | 159 +++++++++++++++++++++++------------------------
 src/components/PdfFilePreview.test.tsx                  | 108 ++++++++++++++++++++++++++++++++
 src/components/PdfFilePreview.tsx                       | 189 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/utils/pdfPreviewRuntime.ts                          |  29 +++++++++
 src/utils/tauriCsp.test.ts                              |  38 ++++++++----
 14 files changed, 635 insertions(+), 112 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 5 or body > 34 lines).

1. `src/components/FilePreview.tsx:480` new, `function FilePreviewLayout(options: {`, values `{"cc":4,"lines":36}`, ceiling cc 5, lines 34, nothing at the base matched

   ```text
   477 |   return previewKind
   478 | }
   479 | 
   480 | function FilePreviewLayout(options: {
   481 |   actions: ReturnType<typeof useFilePreviewActions>
   482 |   assetSrc: string | null
   483 |   canUseFileActions: boolean
   484 |   entry: VaultEntry
   485 |   externalMediaPreview: boolean
   486 |   failures: ReturnType<typeof useFilePreviewFailureState>
   487 |   fileTypeLabel: string
   488 |   locale: AppLocale
   489 |   onCopyDeepLink?: (entry: VaultEntry) => void
   490 |   onCopyFilePath?: (path: string) => void
   491 |   onRevealFile?: (path: string) => void
   492 |   previewKind: FilePreviewKind | null
   493 |   previewRef: RefObject<HTMLElement | null>
   494 | }) {
   495 |   const { actions, assetSrc, canUseFileActions, entry, externalMediaPreview, failures, fileTypeLabel, locale,
   496 |     onCopyDeepLink, onCopyFilePath, onRevealFile, previewKind, previewRef } = options
   497 |   return (
   498 |     <section ref={previewRef} className="flex min-h-0 min-w-0 flex-1 flex-col bg-background text-foreground"
   499 |       data-testid="file-preview" aria-label={`Preview ${entry.title}`}>
   500 |       <FilePreviewHeader entry={entry} previewKind={previewKind} canUseFileActions={canUseFileActions}
   501 |         fileTypeLabel={fileTypeLabel} locale={locale} onOpenExternal={actions.handleOpenExternal}
   502 |         onRevealFile={onRevealFile ? actions.handleRevealFile : undefined}
   503 |         onCopyFilePath={onCopyFilePath ? actions.handleCopyFilePath : undefined}
   504 |         onCopyDeepLink={onCopyDeepLink ? actions.handleCopyDeepLink : undefined} />
   505 |       <div className="min-h-0 flex-1 overflow-auto bg-background">
   506 |         <FilePreviewBody entry={entry}
   507 |           previewKind={previewKindForBody(previewKind, failures.mediaFailed, externalMediaPreview)}
   508 |           assetSrc={assetSrc} imageFailed={failures.imageFailed} canOpenExternal={canUseFileActions}
   509 |           onImageError={failures.handleImageError} onAudioError={failures.handleAudioError}
   510 |           onVideoError={failures.handleVideoError} onPdfError={failures.handlePdfError}
   511 |           onOpenExternal={actions.handleOpenExternal} />
   512 |       </div>
   513 |     </section>
   514 |   )
   515 | }
   516 | 
   ```

2. `src/components/PdfFilePreview.tsx:133` new, `export function PdfFilePreview({ fallback, onError, source, title }: PdfFilePreviewProps) {`, values `{"cc":3,"lines":57}`, ceiling cc 5, lines 34, nothing at the base matched

   ```text
   130 |   )
   131 | }
   132 | 
   133 | export function PdfFilePreview({ fallback, onError, source, title }: PdfFilePreviewProps) {
   134 |   const previewRef = useRef<HTMLDivElement>(null)
   135 |   const [document, setDocument] = useState<PdfDocument | null>(null)
   136 |   const [failed, setFailed] = useState(false)
   137 |   const availableWidth = usePreviewWidth(previewRef)
   138 |   const pageNumbers = useMemo(
   139 |     () => Array.from({ length: document?.numPages ?? 0 }, (_, index) => index + 1),
   140 |     [document?.numPages],
   141 |   )
   142 | 
   143 |   useEffect(() => {
   144 |     let active = true
   145 |     let loadedDocument: PdfDocument | null = null
   146 |     void loadPdfDocument(source)
   147 |       .then((nextDocument) => {
   148 |         loadedDocument = nextDocument
   149 |         if (active) setDocument(nextDocument)
   150 |         else void nextDocument.destroy()
   151 |       })
   152 |       .catch(() => {
   153 |         if (!active) return
   154 |         setFailed(true)
   155 |         onError()
   156 |       })
   157 | 
   158 |     return () => {
   159 |       active = false
   160 |       if (loadedDocument) void loadedDocument.destroy()
   161 |     }
   162 |   }, [onError, source])
   163 | 
   164 |   if (failed) return fallback
   165 | 
   166 |   return (
   167 |     <section
   168 |       ref={previewRef}
   169 |       className="h-full min-h-[320px] overflow-auto bg-muted/20 py-6"
   170 |       data-testid="pdf-file-preview"
   171 |       data-pdf-source={source}
   172 |       aria-busy={document === null}
   173 |       aria-label={title}
   ```

3. `src/components/FilePreview.tsx:316` worsened, `function FilePreviewBody(options: {`, values `{"cc":8,"lines":68}`, ceiling cc 5, lines 34, base site `src/components/FilePreview.tsx:351` with `{"cc":8,"lines":50}`

   ```text
   313 |   return isImage && imageSrc !== null && !imageFailed
   314 | }
   315 | 
   316 | function FilePreviewBody(options: {
   317 |   entry: VaultEntry
   318 |   previewKind: FilePreviewKind | null
   319 |   assetSrc: string | null
   320 |   imageFailed: boolean
   321 |   canOpenExternal: boolean
   322 |   onImageError: () => void
   323 |   onAudioError: () => void
   324 |   onVideoError: () => void
   325 |   onPdfError: () => void
   326 |   onOpenExternal: () => void
   327 | }) {
   328 |   const {
   329 |     entry,
   330 |     previewKind,
   331 |     assetSrc,
   332 |     imageFailed,
   333 |     canOpenExternal,
   334 |     onImageError,
   335 |     onAudioError,
   336 |     onVideoError,
   337 |     onPdfError,
   338 |     onOpenExternal,
   339 |   } = options
   340 |   if (shouldRenderImagePreview(previewKind === 'image', assetSrc, imageFailed)) {
   341 |     return <FilePreviewImage entry={entry} imageSrc={assetSrc} onImageError={onImageError} />
   342 |   }
   343 | 
   344 |   if (previewKind === 'pdf' && assetSrc !== null) {
   345 |     const fallback = fallbackContentForPreviewKind('pdf')
   346 |     return (
   347 |       <PdfFilePreview
   348 |         key={assetSrc}
   349 |         source={assetSrc}
   350 |         title={entry.title}
   351 |         onError={onPdfError}
   352 |         fallback={
   353 |           <FilePreviewFallback
   354 |             icon={fallback.icon}
   355 |             title={fallback.title}
   356 |             description={fallback.description}
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R039

- Repository: `refactoringhq/tolaria` (TypeScript), change 3 of 10
- Commit: `44740e4ae18d`, judged against its first parent `7435ef89016d`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> fix: render PDF previews with PDF.js

### Files the change touched

```text
 docs/ABSTRACTIONS.md                                    |   2 +-
 docs/ARCHITECTURE.md                                    |   2 +-
 docs/adr/0182-app-owned-cross-platform-pdf-rendering.md |  39 ++++++++++++
 docs/adr/README.md                                      |   1 +
 package.json                                            |   1 +
 pnpm-lock.yaml                                          | 135 ++++++++++++++++++++++++++++++++++++++++
 src-tauri/tauri.conf.json                               |   9 +--
 src/components/Editor.test.tsx                          |   6 +-
 src/components/FilePreview.test.tsx                     |  29 ++++++---
 src/components/FilePreview.tsx                          | 159 +++++++++++++++++++++++------------------------
 src/components/PdfFilePreview.test.tsx                  | 108 ++++++++++++++++++++++++++++++++
 src/components/PdfFilePreview.tsx                       | 189 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/utils/pdfPreviewRuntime.ts                          |  29 +++++++++
 src/utils/tauriCsp.test.ts                              |  38 ++++++++----
 14 files changed, 635 insertions(+), 112 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `src/components/PdfFilePreview.test.tsx:5` new, `const { loadPdfDocumentMock, pdfCanvasContextMock } = vi.hoisted(() => ({`, values `{"dead":1}`, nothing at the base matched

   ```text
   2 | import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
   3 | import { PdfFilePreview } from './PdfFilePreview'
   4 | 
   5 | const { loadPdfDocumentMock, pdfCanvasContextMock } = vi.hoisted(() => ({
   6 |   loadPdfDocumentMock: vi.fn(),
   7 |   pdfCanvasContextMock: vi.fn(() => ({} as CanvasRenderingContext2D)),
   8 | }))
   9 | 
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R040

- Repository: `whyour/qinglong` (TypeScript), change 1 of 10
- Commit: `44129ca0883e`, judged against its first parent `def4917447b1`, exit 1
- Gate: complexity, FAIL
- Decision group: lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `7`, 95th percentile of 5,056 functions at def4917, floor 5; recorded scope: whole repository
- Derived lines: `57`, 95th percentile of 5,056 functions at def4917, floor 25; recorded scope: whole repository

### Commit message

> fix: align CLI shell completion and cancellation semantics

### Files the change touched

```text
 cli/src/internal/execution/shellSession.ts | 20 ++++++++++++++++++--
 cli/test/integration/differential.test.cjs | 14 +++++++-------
 cli/test/internal/local.test.cjs           |  8 ++++----
 cli/test/internal/taskLifecycle.test.cjs   |  5 +++--
 4 files changed, 32 insertions(+), 15 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 7 or body > 57 lines).

1. `cli/test/integration/differential.test.cjs:10` new, `test('Shell and CLI agree on isolated script state and account modes', async (t) => {`, values `{"cc":5,"lines":245}`, ceiling cc 7, lines 57, nothing at the base matched

   ```text
   07 | const { createContext, sourceEnvironment } = require('../../dist/internal/runtime/context');
   08 | const { executeTask } = require('../../dist/internal/execution/taskRunner');
   09 | 
   10 | test('Shell and CLI agree on isolated script state and account modes', async (t) => {
   11 |   const root = await fs.realpath(
   12 |     await fs.mkdtemp(path.join(os.tmpdir(), 'ql-differential-')),
   13 |   );
   14 |   t.after(() => fs.rm(root, { recursive: true, force: true }));
   15 |   const bin = path.join(root, 'bin');
   16 |   await fs.mkdir(bin);
   17 |   await fs.symlink(process.execPath, path.join(bin, 'node'));
   18 |   await fs.writeFile(path.join(bin, 'pnpm'), '#!/bin/sh\nexit 0\n', {
   19 |     mode: 0o755,
   20 |   });
   21 |   const context = createContext(
   22 |     { root },
   23 |     { PATH: `${bin}:/usr/local/bin:/usr/bin:/bin`, no_tee: 'true' },
   24 |   );
   25 |   for (const directory of [
   26 |     context.paths.dir_shell,
   27 |     context.paths.dir_preload,
   28 |     context.paths.dir_config,
   29 |     context.paths.dir_scripts,
   30 |     context.paths.dir_log,
   31 |     path.join(root, 'static/build'),
   32 |   ])
   33 |     await fs.mkdir(directory, { recursive: true });
   34 |   for (const name of ['task.sh', 'otask.sh', 'share.sh', 'api.sh', 'env.sh'])
   35 |     await fs.copyFile(
   36 |       path.resolve(__dirname, '../../../shell', name),
   37 |       path.join(context.paths.dir_shell, name),
   38 |     );
   39 |   await fs.mkdir(path.join(context.paths.dir_shell, 'lang'));
   40 |   for (const name of ['zh.sh', 'en.sh'])
   41 |     await fs.copyFile(
   42 |       path.resolve(__dirname, '../../../shell/lang', name),
   43 |       path.join(context.paths.dir_shell, 'lang', name),
   44 |     );
   45 |   await fs.writeFile(
   46 |     path.join(root, 'static/build/token.js'),
   47 |     'process.stdout.write("fixture-local-token")',
   48 |   );
   49 |   await fs.writeFile(
   50 |     context.paths.file_config_user,
   ```

2. `cli/src/internal/execution/shellSession.ts:5` worsened, `export function shellSessionArguments(`, values `{"cc":3,"lines":110}`, ceiling cc 7, lines 57, base site `cli/src/internal/execution/shellSession.ts:5` with `{"cc":3,"lines":94}`

   ```text
   02 | 
   03 | // Bash is the interpreter for user-owned files. The CLI supplies paths/argv as
   04 | // separate arguments; no user argument is interpolated into this fixed bridge.
   05 | export function shellSessionArguments(
   06 |   context: LocalContext,
   07 |   argv: string[],
   08 |   scriptArgs: string[],
   09 |   command = false,
   10 |   timeoutMarker = '',
   11 | ): string[] {
   12 |   const scriptIndex = command
   13 |     ? argv.findIndex((arg) => /\.(?:js|mjs|py|pyc|sh|ts)$/.test(arg))
   14 |     : 0;
   15 |   const bridge = `
   16 | __ql_env=$1; __ql_before=$2; __ql_after=$3; __ql_command=$4; __ql_index=$5; __ql_count=$6; __ql_timeout=$7; shift 7
   17 | __ql_hook_args=( "\${@:1:__ql_count}" ); shift "$__ql_count"
   18 | __ql_script_args=( "$@" )
   19 | # Reserve an internal descriptor before sourcing user code. In particular, fd 3
   20 | # and fd 9 remain available for user redirections and locks. Bash 3 also supports
   21 | # this numeric descriptor, unlike Bash 4's dynamic descriptor syntax.
   22 | if [ -n "$__ql_timeout" ]; then exec 19>&3 3>&-; fi
   23 | __ql_after_started=false
   24 | __ql_run_after() {
   25 |   __ql_after_started=true
   26 |   if [ -n "$__ql_timeout" ]; then printf A >&19; fi
   27 |   if [ "$__ql_nounset" = true ]; then set -u; fi
   28 |   export NODE_PATH="\${PREV_NODE_PATH:-}"
   29 |   unset QL_NODE_GLOBAL_PATH
   30 |   if [ -f "$__ql_after" ]; then . "$__ql_after" "\${__ql_hook_args[@]}"; fi
   31 |   if [ -n "\${task_after:-}" ]; then eval "\${task_after%;}"; fi
   32 | }
   33 | __ql_timeout_exit() {
   34 |   if [ -f "$__ql_timeout" ] && [ "$__ql_after_started" = false ]; then
   35 |     _task_exit_code=124
   36 |     __ql_run_after
   37 |     exit 124
   38 |   fi
   39 | }
   40 | if [ -f "$__ql_env" ]; then . "$__ql_env"; fi
   41 | if [ -f "$__ql_before" ]; then . "$__ql_before" "\${__ql_hook_args[@]}"; fi
   42 | if [ -n "\${task_before:-}" ]; then eval "\${task_before%;}"; fi
   43 | __ql_nounset=false
   44 | case $- in *u*) __ql_nounset=true; set +u;; esac
   45 | if [ -n "\${__ql_selected_name:-}" ]; then
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R041

- Repository: `whyour/qinglong` (TypeScript), change 3 of 10
- Commit: `bc0f35e3f748`, judged against its first parent `f168efcdaeba`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> fix: reduce task startup overhead and lazy-load notifications (#3080)

### Files the change touched

```text
 docs/development/task-startup.md       |  54 ++++++++++++++++++++++++
 shell/node_path_cache.sh               |  12 +++++-
 shell/preload/sitecustomize.js         |   8 +++-
 shell/preload/sitecustomize.py         |   6 ++-
 test/back/node-path-cache.test.cjs     |  29 +++++++++++++
 test/back/node-path-lock.test.cjs      |   8 +++-
 test/back/preload-lazy-notify.test.cjs | 168 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 7 files changed, 277 insertions(+), 8 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `shell/node_path_cache.sh:101` new, `-name "pnpm-root-${EUID}-*.cache" -mmin +60 -delete 2>/dev/null || true`, values `{"count":1,"escape":"errors ignored"}`, nothing at the base matched

   ```text
   098 |   # Warm lookups never scan the directory. Retire old environment records on
   099 |   # refresh, while preserving the shared lock and unrelated cache files.
   100 |   find "$dir_tmp" -maxdepth 1 -type f -user "$EUID" \
   101 |     -name "pnpm-root-${EUID}-*.cache" -mmin +60 -delete 2>/dev/null || true
   102 |   now=${EPOCHSECONDS:-$(date +%s)}
   103 |   result=$(pnpm root -g 9>&- 2>/dev/null) || return $?
   104 |   # Never cache failed, empty, multiline or non-absolute answers.
   105 |   if [[ "$result" == /* && "$result" != *$'\n'* && "$result" != *$'\r'* ]]; then
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R042

- Repository: `whyour/qinglong` (TypeScript), change 3 of 10
- Commit: `bc0f35e3f748`, judged against its first parent `f168efcdaeba`, exit 1
- Gate: complexity, FAIL
- Decision group: lines at a derived percentile, the base site already over
- Derived cc: `7`, 95th percentile of 5,025 functions at f168efc, floor 5; recorded scope: whole repository
- Derived lines: `57`, 95th percentile of 5,025 functions at f168efc, floor 25; recorded scope: whole repository

### Commit message

> fix: reduce task startup overhead and lazy-load notifications (#3080)

### Files the change touched

```text
 docs/development/task-startup.md       |  54 ++++++++++++++++++++++++
 shell/node_path_cache.sh               |  12 +++++-
 shell/preload/sitecustomize.js         |   8 +++-
 shell/preload/sitecustomize.py         |   6 ++-
 test/back/node-path-cache.test.cjs     |  29 +++++++++++++
 test/back/node-path-lock.test.cjs      |   8 +++-
 test/back/preload-lazy-notify.test.cjs | 168 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 7 files changed, 277 insertions(+), 8 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 7 or body > 57 lines).

1. `test/back/node-path-lock.test.cjs:9` worsened, `function fixture(t) {`, values `{"cc":1,"lines":62}`, ceiling cc 7, lines 57, base site `test/back/node-path-lock.test.cjs:9` with `{"cc":1,"lines":58}`

   ```text
   06 | const { spawn, spawnSync } = require('node:child_process');
   07 | const helper = path.resolve('shell/node_path_cache.sh');
   08 | const hasFlock = spawnSync('/bin/bash', ['-c', 'type -P flock']).status === 0;
   09 | function fixture(t) {
   10 |   const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ql-path-lock-'));
   11 |   t.after(() => fs.rmSync(root, { recursive: true, force: true }));
   12 |   const bin = path.join(root, 'bin');
   13 |   fs.mkdirSync(bin);
   14 |   fs.symlinkSync(process.execPath, path.join(bin, 'node'));
   15 |   fs.writeFileSync(
   16 |     path.join(bin, 'pnpm'),
   17 |     '#!/bin/bash\nprintf "call\\n" >> "$CALLS"\n[[ -n "${UMASK_FILE:-}" ]] && umask > "$UMASK_FILE"\nsleep "${LOOKUP_DELAY:-0.2}"\n[[ "${FAIL:-0}" == 1 ]] && exit 17\nprintf "%s\\n" "${npm_config_global_dir:-/test/global/node_modules}"\n',
   18 |     { mode: 0o755 },
   19 |   );
   20 |   const env = {
   21 |     ...process.env,
   22 |     HOME: root,
   23 |     PATH: bin + ':' + process.env.PATH,
   24 |     dir_tmp: path.join(root, 'cache'),
   25 |     CALLS: path.join(root, 'calls'),
   26 |     HELPER: helper,
   27 |   };
   28 |   const command = '. "$HELPER"; ql_get_node_global_path';
   29 |   const run = (extra = {}, code = command) =>
   30 |     spawnSync('/bin/bash', ['-euc', code], {
   31 |       cwd: root,
   32 |       env: { ...env, ...extra },
   33 |       encoding: 'utf8',
   34 |     });
   35 |   const asyncRun = (extra = {}) =>
   36 |     new Promise((resolve, reject) => {
   37 |       const child = spawn('/bin/bash', ['-euc', command], {
   38 |         cwd: root,
   39 |         env: { ...env, ...extra },
   40 |       });
   41 |       let out = '',
   42 |         err = '';
   43 |       child.stdout.on('data', (c) => (out += c));
   44 |       child.stderr.on('data', (c) => (err += c));
   45 |       child.on('error', reject);
   46 |       child.on('close', (status) =>
   47 |         resolve({ status, stdout: out, stderr: err }),
   48 |       );
   49 |     });
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R043

- Repository: `whyour/qinglong` (TypeScript), change 4 of 10
- Commit: `f168efcdaeba`, judged against its first parent `bf63425805be`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> refactor: migrate cron scheduling to node-cron (#3079)
>
> * refactor: migrate cron scheduling to node-cron
>
> * feat: support annually midnight and minutely cron macros
>
> * fix: harden cron recovery and system scheduler compatibility

### Files the change touched

```text
 back/api/subscription.ts                       |   13 +-
 back/schedule/addCron.ts                       |  121 +++---
 back/schedule/client.ts                        |    8 +-
 back/schedule/data.ts                          |    4 +-
 back/schedule/delCron.ts                       |    2 +-
 back/services/cron.ts                          |   19 +-
 back/services/schedule.ts                      |   47 ++-
 back/shared/cronSchedule.ts                    |   97 ++++-
 back/shared/cronScheduler.ts                   |   89 +++++
 docs/development/node-cron-migration.md        |   49 +++
 package.json                                   |    4 +-
 pnpm-lock.yaml                                 |   52 +--
 test/back/cron-macros.test.cjs                 |   51 +++
 test/back/cron-schedule-compatibility.test.cjs |   98 +++--
 test/back/cron-scheduler-routing.test.cjs      |   10 +-
 test/back/legacy-cron-recovery.test.cjs        |   10 +-
 test/back/node-cron-migration.test.cjs         |   65 +++
 test/back/scheduler-bulk-recovery.test.cjs     |   40 ++
 test/back/scheduler-reconciliation.test.cjs    |    6 +-
 test/fixtures/legacy-cron.json                 | 1371 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 20 files changed, 1955 insertions(+), 201 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `back/schedule/addCron.ts:84` new, `const err: any = new Error(`, values `{"count":1,"escape":"any"}`, nothing at the base matched

   ```text
   81 |     }
   82 |   } catch (error) {
   83 |     for (const jobs of prepared.values()) jobs.forEach((job) => job.cancel());
   84 |     const err: any = new Error(
   85 |       error instanceof Error ? error.message : String(error),
   86 |     );
   87 |     err.code = status.INVALID_ARGUMENT;
   88 |     err.details = err.message;
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R044

- Repository: `whyour/qinglong` (TypeScript), change 4 of 10
- Commit: `f168efcdaeba`, judged against its first parent `bf63425805be`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `7`, 95th percentile of 4,946 functions at bf63425, floor 5; recorded scope: whole repository
- Derived lines: `57`, 95th percentile of 4,946 functions at bf63425, floor 25; recorded scope: whole repository

### Commit message

> refactor: migrate cron scheduling to node-cron (#3079)
>
> * refactor: migrate cron scheduling to node-cron
>
> * feat: support annually midnight and minutely cron macros
>
> * fix: harden cron recovery and system scheduler compatibility

### Files the change touched

```text
 back/api/subscription.ts                       |   13 +-
 back/schedule/addCron.ts                       |  121 +++---
 back/schedule/client.ts                        |    8 +-
 back/schedule/data.ts                          |    4 +-
 back/schedule/delCron.ts                       |    2 +-
 back/services/cron.ts                          |   19 +-
 back/services/schedule.ts                      |   47 ++-
 back/shared/cronSchedule.ts                    |   97 ++++-
 back/shared/cronScheduler.ts                   |   89 +++++
 docs/development/node-cron-migration.md        |   49 +++
 package.json                                   |    4 +-
 pnpm-lock.yaml                                 |   52 +--
 test/back/cron-macros.test.cjs                 |   51 +++
 test/back/cron-schedule-compatibility.test.cjs |   98 +++--
 test/back/cron-scheduler-routing.test.cjs      |   10 +-
 test/back/legacy-cron-recovery.test.cjs        |   10 +-
 test/back/node-cron-migration.test.cjs         |   65 +++
 test/back/scheduler-bulk-recovery.test.cjs     |   40 ++
 test/back/scheduler-reconciliation.test.cjs    |    6 +-
 test/fixtures/legacy-cron.json                 | 1371 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 20 files changed, 1955 insertions(+), 201 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 7 or body > 57 lines).

1. `back/services/cron.ts:47` new, `private isNodeCron(cron: Crontab) {`, values `{"cc":10,"lines":18}`, ceiling cc 7, lines 57, nothing at the base matched

   ```text
   44 | export default class CronService {
   45 |   constructor(@Inject('logger') private logger: winston.Logger) { }
   46 | 
   47 |   private isNodeCron(cron: Crontab) {
   48 |     const { schedule, extra_schedules } = cron;
   49 |     const fields = schedule?.trim().split(/\s+/) || [];
   50 |     // System crontab only receives portable numeric five-field expressions.
   51 |     // Extended syntax, macros and legacy shorthand use the Node scheduler.
   52 |     return (
   53 |       fields.length !== 5 ||
   54 |       /[^\d\s*,/\-]/.test(schedule || '') ||
   55 |       Boolean(extra_schedules?.length) ||
   56 |       // BusyBox treats N/step as a single value and doesn't support Sunday=7.
   57 |       fields.some((field) => field.split(',').some((part) => /^\d+\//.test(part))) ||
   58 |       fields[4].includes('7') ||
   59 |       // BusyBox steps DOM from zero; its full-range day/week wildcard
   60 |       // handling also differs from the legacy Node parser's OR semantics.
   61 |       fields[2].includes('/') ||
   62 |       (fields[2] !== '*' && fields[4] !== '*')
   63 |     );
   64 |   }
   65 | 
   ```

2. `back/shared/cronSchedule.ts:28` new, `export function parseCronSchedule(schedule: unknown): CronSchedule {`, values `{"cc":14,"lines":58}`, ceiling cc 7, lines 57, nothing at the base matched

   ```text
   25 | 
   26 | // Canonicalize legacy shorthand, aliases, names and numeric-start steps before
   27 | // node-cron sees them. Extend legacy aliases explicitly; reject H and bare /N.
   28 | export function parseCronSchedule(schedule: unknown): CronSchedule {
   29 |   if (typeof schedule !== 'string' || !schedule.trim()) {
   30 |     throw new Error('Invalid cron schedule');
   31 |   }
   32 |   let source = schedule.trim();
   33 |   const cacheKey = source;
   34 |   const cached = scheduleCache.get(cacheKey);
   35 |   if (cached) return cached;
   36 |   if (source.startsWith('@')) {
   37 |     if (!Object.hasOwn(aliases, source)) throw new Error('Invalid cron alias');
   38 |     source = aliases[source];
   39 |   }
   40 |   let parts = source.split(/\s+/);
   41 |   if (
   42 |     parts.length > 6 ||
   43 |     parts.some((part) => /(^|,)\//.test(part) || /H(?:\(|\/|$)/i.test(part))
   44 |   ) {
   45 |     throw new Error('Unsupported cron syntax');
   46 |   }
   47 |   parts = [
   48 |     ...['0', '*', '*', '*', '*', '*'].slice(0, 6 - parts.length),
   49 |     ...parts,
   50 |   ];
   51 |   if (
   52 |     parts.some(
   53 |       (part, index) => index !== 3 && index !== 5 && part.includes('?'),
   54 |     )
   55 |   ) {
   56 |     throw new Error('Question mark is only valid in day fields');
   57 |   }
   58 |   const expression = CronExpressionParser.parse(parts.join(' '));
   59 |   if (!expression.hasNext())
   60 |     throw new Error('Cron schedule has no next execution');
   61 |   const normalized = expression.stringify(true).replace(/\?/g, '*').split(' ');
   62 |   const dayCount = expression.fields.dayOfMonth.values.length;
   63 |   const weekCount = expression.fields.dayOfWeek.values.length;
   64 |   const patterns =
   65 |     dayCount < 31 && weekCount < 8
   66 |       ? [
   67 |           [...normalized.slice(0, 5), '*'].join(' '),
   68 |           [...normalized.slice(0, 3), '*', ...normalized.slice(4)].join(' '),
   ```

3. `back/shared/cronScheduler.ts:16` new, `export function createCronJob(`, values `{"cc":4,"lines":74}`, ceiling cc 7, lines 57, nothing at the base matched

   ```text
   13 | 
   14 | // node-cron owns timers and calendar calculation; Qinglong owns execution
   15 | // concurrency and process lifetime. Missed in-process slots join that same queue.
   16 | export function createCronJob(
   17 |   schedule: string,
   18 |   callback: (date: Date) => unknown | Promise<unknown>,
   19 |   options: { name: string; logger: SchedulerLogger; start?: boolean },
   20 | ): CronJob {
   21 |   const parsed = parseCronSchedule(schedule);
   22 |   const tasks: ScheduledTask[] = [];
   23 |   let cancelled = false;
   24 |   let started = false;
   25 |   const accepts = (index: number, date: Date) => {
   26 |     if (tasks.length === 1) return true;
   27 |     // Preserve cron-parser 4's day/weekday OR semantics, including its
   28 |     // month-dependent full-day range and the global nth-week constraint.
   29 |     if (parsed.nthDay && Math.ceil(date.getDate() / 7) !== parsed.nthDay)
   30 |       return false;
   31 |     if (index === 1) return !tasks[0].match(date);
   32 |     const monthDays = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
   33 |     return parsed.dayCount < monthDays[date.getMonth()] || tasks[1].match(date);
   34 |   };
   35 |   const dispatch = async (index: number, date: Date, missed: boolean) => {
   36 |     if (cancelled || !started || !accepts(index, date)) return;
   37 |     if (missed) {
   38 |       options.logger.warn(
   39 |         '[schedule][补执行迟到任务] 任务: %s, 计划时间: %s',
   40 |         options.name,
   41 |         date.toISOString(),
   42 |       );
   43 |     }
   44 |     try {
   45 |       await callback(date);
   46 |     } catch (error) {
   47 |       options.logger.error(
   48 |         '[schedule][定时回调失败] 任务: %s, 错误: %s',
   49 |         options.name,
   50 |         error instanceof Error ? error.message : String(error),
   51 |       );
   52 |     }
   53 |   };
   54 |   try {
   55 |     for (const [index, pattern] of parsed.patterns.entries()) {
   56 |       const task = createTask(
   ```

4. `back/schedule/addCron.ts:15` worsened, `const addCron = (`, values `{"cc":15,"lines":96}`, ceiling cc 7, lines 57, base site `back/schedule/addCron.ts:15` with `{"cc":14,"lines":121}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R045

- Repository: `whyour/qinglong` (TypeScript), change 8 of 10
- Commit: `6c487018c2e6`, judged against its first parent `801a71d7402f`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `7`, 95th percentile of 4,861 functions at 801a71d, floor 5; recorded scope: whole repository
- Derived lines: `58`, 95th percentile of 4,861 functions at 801a71d, floor 25; recorded scope: whole repository

### Commit message

> fix: drain completed task logs and prepare 2.22.0 release (#3076)

### Files the change touched

```text
 docs/releases/2.22.0-validation.md      |  49 +++++++++++++++++
 docs/releases/2.22.0.md                 |  54 ++++++++++++++++++
 package.json                            |   2 +-
 src/pages/crontab/logModal.tsx          |  19 +++++--
 test/front/cron-log-pagination.test.cjs | 214 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 version.yaml                            |  53 +++++++-----------
 6 files changed, 352 insertions(+), 39 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 7 or body > 58 lines).

1. `src/pages/crontab/logModal.tsx:59` new, `.then(({ code, data, logStatus, nextOffset, total }) => {`, values `{"cc":18,"lines":39}`, ceiling cc 7, lines 58, nothing at the base matched

   ```text
   56 |     }`;
   57 |     request
   58 |       .get(`${baseUrl}${pagination}`)
   59 |       .then(({ code, data, logStatus, nextOffset, total }) => {
   60 |         if (code !== 200 || localStorage.getItem("logCron") !== uniqPath) {
   61 |           return;
   62 |         }
   63 | 
   64 |         const isRunning = logStatus === "running";
   65 |         // A completed process can still have multiple unread chunks. Drain
   66 |         // them before stopping, without presenting it as a running task.
   67 |         const hasUnread =
   68 |           typeof nextOffset === "number" &&
   69 |           typeof total === "number" &&
   70 |           nextOffset > (offset ?? 0) &&
   71 |           nextOffset < total;
   72 |         const hasNext = isRunning || hasUnread;
   73 |         const chunk = (data as string) || "";
   74 |         let log = isFirst ? chunk : `${valueRef.current}${chunk}`;
   75 |         if (!log && !hasNext) {
   76 |           log = intl.get("暂无日志");
   77 |         }
   78 |         if (log.length > MAX_LOG_VIEW_CHARS) {
   79 |           log = log.slice(-MAX_LOG_VIEW_CHARS);
   80 |         }
   81 |         valueRef.current = log;
   82 |         setValue(log);
   83 |         if (typeof nextOffset === "number") {
   84 |           logOffsetRef.current = nextOffset;
   85 |         }
   86 |         setExecuting(isRunning);
   87 | 
   88 |         if (chunk || !hasNext) {
   89 |           autoScroll();
   90 |         }
   91 |         if (hasNext) {
   92 |           pollTimerRef.current = setTimeout(
   93 |             () => getCronLog(),
   94 |             hasUnread ? 0 : 2000,
   95 |           );
   96 |         }
   97 |       })
   98 |       .finally(() => {
   ```

2. `test/front/cron-log-pagination.test.cjs:33` new, `function fixture(get) {`, values `{"cc":2,"lines":79}`, ceiling cc 7, lines 58, nothing at the base matched

   ```text
   30 | }
   31 | visit(ast);
   32 | 
   33 | function fixture(get) {
   34 |   const timers = new Map();
   35 |   const storage = new Map([['logCron', '1']]);
   36 |   const values = { content: '', executing: true };
   37 |   const pending = [];
   38 |   let timerId = 0;
   39 |   const context = {
   40 |     logUrl: undefined,
   41 |     config: { apiPrefix: '/api/' },
   42 |     cron: { id: 1 },
   43 |     uniqPath: '1',
   44 |     LOG_CHUNK_BYTES: 256 * 1024,
   45 |     MAX_LOG_VIEW_CHARS: 1024 * 1024,
   46 |     logOffsetRef: { current: undefined },
   47 |     valueRef: { current: '' },
   48 |     pollTimerRef: {},
   49 |     request: {
   50 |       get(url) {
   51 |         const result = get(url);
   52 |         pending.push(result);
   53 |         return result;
   54 |       },
   55 |     },
   56 |     localStorage: {
   57 |       getItem: (key) => storage.get(key),
   58 |       removeItem: (key) => storage.delete(key),
   59 |     },
   60 |     intl: { get: (value) => value },
   61 |     setLoading() {},
   62 |     autoScroll() {},
   63 |     handleCancel() {},
   64 |     setValue(value) {
   65 |       values.content = value;
   66 |     },
   67 |     setExecuting(value) {
   68 |       values.executing = value;
   69 |     },
   70 |     setTimeout(fn, delay) {
   71 |       timers.set(++timerId, { fn, delay });
   72 |       return timerId;
   73 |     },
   ```

3. `src/pages/crontab/logModal.tsx:26` worsened, `const CronLogModal = ({`, values `{"cc":4,"lines":181}`, ceiling cc 7, lines 58, base site `src/pages/crontab/logModal.tsx:26` with `{"cc":4,"lines":170}`

   ```text
   23 | const LOG_CHUNK_BYTES = 256 * 1024;
   24 | const MAX_LOG_VIEW_CHARS = 1024 * 1024;
   25 | 
   26 | const CronLogModal = ({
   27 |   cron,
   28 |   handleCancel,
   29 |   data,
   30 |   logUrl,
   31 | }: {
   32 |   cron?: any;
   33 |   handleCancel: () => void;
   34 |   data?: string;
   35 |   logUrl?: string;
   36 | }) => {
   37 |   const [value, setValue] = useState<string>(intl.get("启动中..."));
   38 |   const [loading, setLoading] = useState<any>(true);
   39 |   const [executing, setExecuting] = useState<any>(true);
   40 |   const [isPhone, setIsPhone] = useState(false);
   41 |   const scrollInfoRef = useRef({ value: 0, down: true });
   42 |   const logOffsetRef = useRef<number>();
   43 |   const valueRef = useRef(value);
   44 |   const pollTimerRef = useRef<ReturnType<typeof setTimeout>>();
   45 |   const uniqPath = logUrl ? logUrl : String(cron?.id);
   46 | 
   47 |   const getCronLog = (isFirst?: boolean) => {
   48 |     if (isFirst) {
   49 |       setLoading(true);
   50 |     }
   51 |     const baseUrl = logUrl ? logUrl : `${config.apiPrefix}crons/${cron.id}/log`;
   52 |     const separator = baseUrl.includes("?") ? "&" : "?";
   53 |     const offset = isFirst ? undefined : logOffsetRef.current;
   54 |     const pagination = `${separator}limit=${LOG_CHUNK_BYTES}${
   55 |       isFirst ? "&tail=true" : offset !== undefined ? `&offset=${offset}` : ""
   56 |     }`;
   57 |     request
   58 |       .get(`${baseUrl}${pagination}`)
   59 |       .then(({ code, data, logStatus, nextOffset, total }) => {
   60 |         if (code !== 200 || localStorage.getItem("logCron") !== uniqPath) {
   61 |           return;
   62 |         }
   63 | 
   64 |         const isRunning = logStatus === "running";
   65 |         // A completed process can still have multiple unread chunks. Drain
   66 |         // them before stopping, without presenting it as a running task.
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R046

- Repository: `whyour/qinglong` (TypeScript), change 9 of 10
- Commit: `801a71d7402f`, judged against its first parent `f051135fc4ab`, exit 1
- Gate: doc-size, FAIL
- Decision group: document README-en.md; document README.md
- Derived AGENTS.md: `400`, the word count at the derivation commit, rounded up to the next 50
- Derived README-en.md: `550`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `250`, the word count at the derivation commit, rounded up to the next 50
- Derived SECURITY.md: `50`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> feat(cli): cover OpenAPI with a remote npm CLI and internal panel tools (#3074)
>
> * feat(cli): add unified Commander CLI for QingLong 2.x
>
> * fix(cli): publish via npm and address security review feedback
>
> * ci(cli): package npm artifacts and remove evaluation collateral
>
> * test(cli): use a fixed shell fixture for log retention
>
> * refactor(cli): separate remote npm client from panel tools
>
> * feat(cli): cover active panel OpenAPI resources
>
> * docs(cli): unify authentication and skill guidance
>
> * refactor(cli): isolate internal commands and generate Commander help
>
> * refactor(cli): organize remote and internal modules by responsibility
>
> * ci(cli): publish verified npm archives from master
>
> * fix(cli): publish under the whyour npm scope
>
> * ci: use npm trusted publishing for both packages
>
> * docs: introduce the published CLI on the project homepage
>
> * fix(cli): preserve server log truncation and correct login hints
>
> (13 more lines)

### Files the change touched

```text
 .github/workflows/build-docker-image.yml            |  18 +-
 .github/workflows/cli-package.yml                   | 115 +++++++++++++
 README-en.md                                        |  24 +++
 README.md                                           |  24 +++
 back/loaders/deps.ts                                |  46 ++++-
 cli/.gitignore                                      |   2 +
 cli/LICENSE                                         | 201 ++++++++++++++++++++++
 cli/LOCAL.en.md                                     |  45 +++++
 cli/LOCAL.md                                        |  47 ++++++
 cli/README.en.md                                    | 156 +++++++++++++++++
 cli/README.md                                       | 156 +++++++++++++++++
 cli/package-lock.json                               | 550 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 cli/package.json                                    |  40 +++++
 cli/scripts/build.cjs                               |  77 +++++++++
 cli/scripts/entrypoints.cjs                         |  55 ++++++
 cli/scripts/test.cjs                                |  22 +++
 cli/scripts/verify-package.cjs                      | 124 ++++++++++++++
 cli/skills/qinglong-cli/SKILL.md                    |  21 +++
 cli/skills/qinglong-cli/references/openapi.md       | 213 +++++++++++++++++++++++
 cli/skills/qinglong-cli/references/panel.md         |  57 +++++++
 cli/skills/qinglong-local/SKILL.md                  |  19 +++
 cli/skills/qinglong-local/references/execution.md   |  35 ++++
 cli/skills/qinglong-local/references/maintenance.md |  28 ++++
 cli/src/compatibility/legacy.ts                     | 126 ++++++++++++++
 cli/src/compatibility/main.ts                       |  11 ++
 cli/src/compatibility/startup.ts                    |  50 ++++++
 cli/src/entrypoints/admin.ts                        |  21 +++
 cli/src/entrypoints/container.ts                    |  30 ++++
 cli/src/entrypoints/index.ts                        |   6 +
 cli/src/entrypoints/ql.ts                           | 224 +++++++++++++++++++++++++
 ...
 185 files changed, 22412 insertions(+), 6 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `README-en.md` new, values `{"ceiling":550,"words":726}`, nothing at the base matched
2. `README.md` new, values `{"ceiling":250,"words":307}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R047

- Repository: `whyour/qinglong` (TypeScript), change 9 of 10
- Commit: `801a71d7402f`, judged against its first parent `f051135fc4ab`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> feat(cli): cover OpenAPI with a remote npm CLI and internal panel tools (#3074)
>
> * feat(cli): add unified Commander CLI for QingLong 2.x
>
> * fix(cli): publish via npm and address security review feedback
>
> * ci(cli): package npm artifacts and remove evaluation collateral
>
> * test(cli): use a fixed shell fixture for log retention
>
> * refactor(cli): separate remote npm client from panel tools
>
> * feat(cli): cover active panel OpenAPI resources
>
> * docs(cli): unify authentication and skill guidance
>
> * refactor(cli): isolate internal commands and generate Commander help
>
> * refactor(cli): organize remote and internal modules by responsibility
>
> * ci(cli): publish verified npm archives from master
>
> * fix(cli): publish under the whyour npm scope
>
> * ci: use npm trusted publishing for both packages
>
> * docs: introduce the published CLI on the project homepage
>
> * fix(cli): preserve server log truncation and correct login hints
>
> (13 more lines)

### Files the change touched

```text
 .github/workflows/build-docker-image.yml            |  18 +-
 .github/workflows/cli-package.yml                   | 115 +++++++++++++
 README-en.md                                        |  24 +++
 README.md                                           |  24 +++
 back/loaders/deps.ts                                |  46 ++++-
 cli/.gitignore                                      |   2 +
 cli/LICENSE                                         | 201 ++++++++++++++++++++++
 cli/LOCAL.en.md                                     |  45 +++++
 cli/LOCAL.md                                        |  47 ++++++
 cli/README.en.md                                    | 156 +++++++++++++++++
 cli/README.md                                       | 156 +++++++++++++++++
 cli/package-lock.json                               | 550 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 cli/package.json                                    |  40 +++++
 cli/scripts/build.cjs                               |  77 +++++++++
 cli/scripts/entrypoints.cjs                         |  55 ++++++
 cli/scripts/test.cjs                                |  22 +++
 cli/scripts/verify-package.cjs                      | 124 ++++++++++++++
 cli/skills/qinglong-cli/SKILL.md                    |  21 +++
 cli/skills/qinglong-cli/references/openapi.md       | 213 +++++++++++++++++++++++
 cli/skills/qinglong-cli/references/panel.md         |  57 +++++++
 cli/skills/qinglong-local/SKILL.md                  |  19 +++
 cli/skills/qinglong-local/references/execution.md   |  35 ++++
 cli/skills/qinglong-local/references/maintenance.md |  28 ++++
 cli/src/compatibility/legacy.ts                     | 126 ++++++++++++++
 cli/src/compatibility/main.ts                       |  11 ++
 cli/src/compatibility/startup.ts                    |  50 ++++++
 cli/src/entrypoints/admin.ts                        |  21 +++
 cli/src/entrypoints/container.ts                    |  30 ++++
 cli/src/entrypoints/index.ts                        |   6 +
 cli/src/entrypoints/ql.ts                           | 224 +++++++++++++++++++++++++
 ...
 185 files changed, 22412 insertions(+), 6 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `cli/src/internal/runtime/containerEnvironment.ts:66` new, `.map((line) => line.split('#', 1)[0]!.trim());`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   63 |     }
   64 |     const lines = content
   65 |       .split(/\r?\n/)
   66 |       .map((line) => line.split('#', 1)[0]!.trim());
   67 |     const missing = entries.filter((entry) => !lines.some(entry.matches));
   68 |     if (!missing.length) return;
   69 |     // Append in place: Docker may mount these files individually, so rename
   70 |     // based replacement would fail with EBUSY. Keep existing administrator data.
   ```

2. `cli/src/internal/runtime/process.ts:182` new, `child.stdout!.on('data', (chunk: Buffer) => {`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   179 |         } else chunks.push(chunk);
   180 |       } else emit(chunk);
   181 |     };
   182 |     child.stdout!.on('data', (chunk: Buffer) => {
   183 |       if (options.captureFd === 3) emit(chunk);
   184 |       else capture(chunk);
   185 |     });
   186 |     if (options.captureFd === 3) child.stdio[3]!.on('data', capture);
   ```

3. `cli/src/internal/runtime/process.ts:186` new, `if (options.captureFd === 3) child.stdio[3]!.on('data', capture);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   183 |       if (options.captureFd === 3) emit(chunk);
   184 |       else capture(chunk);
   185 |     });
   186 |     if (options.captureFd === 3) child.stdio[3]!.on('data', capture);
   187 |     if (options.timeoutControl) child.stdio[3]!.on('data', (chunk: Buffer) => {
   188 |       for (const phase of chunk.toString()) {
   189 |         if (phase === 'T') startTimeout();
   190 |         if (phase === 'A') {
   ```

4. `cli/src/internal/runtime/process.ts:187` new, `if (options.timeoutControl) child.stdio[3]!.on('data', (chunk: Buffer) => {`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
5. `cli/src/internal/runtime/process.ts:201` new, `child.stderr!.on('data', (chunk: Buffer) =>`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
6. `cli/src/remote/commands/open.ts:92` new, `? command.positionals[1]!.replace(/^\/open\//, '').replace(/^\//, '')`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
7. `cli/src/remote/commands/open.ts:100` new, `? route(command.positionals[0]!.toUpperCase(), endpoint)`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
8. `cli/src/shared/cli/arguments.ts:91` new, `groups.get(parentName!)!.addCommand(command);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R048

- Repository: `whyour/qinglong` (TypeScript), change 9 of 10
- Commit: `801a71d7402f`, judged against its first parent `f051135fc4ab`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; lines at a derived percentile
- Derived cc: `7`, 95th percentile of 3,635 functions at f051135, floor 5; recorded scope: whole repository
- Derived lines: `48`, 95th percentile of 3,635 functions at f051135, floor 25; recorded scope: whole repository

### Commit message

> feat(cli): cover OpenAPI with a remote npm CLI and internal panel tools (#3074)
>
> * feat(cli): add unified Commander CLI for QingLong 2.x
>
> * fix(cli): publish via npm and address security review feedback
>
> * ci(cli): package npm artifacts and remove evaluation collateral
>
> * test(cli): use a fixed shell fixture for log retention
>
> * refactor(cli): separate remote npm client from panel tools
>
> * feat(cli): cover active panel OpenAPI resources
>
> * docs(cli): unify authentication and skill guidance
>
> * refactor(cli): isolate internal commands and generate Commander help
>
> * refactor(cli): organize remote and internal modules by responsibility
>
> * ci(cli): publish verified npm archives from master
>
> * fix(cli): publish under the whyour npm scope
>
> * ci: use npm trusted publishing for both packages
>
> * docs: introduce the published CLI on the project homepage
>
> * fix(cli): preserve server log truncation and correct login hints
>
> (13 more lines)

### Files the change touched

```text
 .github/workflows/build-docker-image.yml            |  18 +-
 .github/workflows/cli-package.yml                   | 115 +++++++++++++
 README-en.md                                        |  24 +++
 README.md                                           |  24 +++
 back/loaders/deps.ts                                |  46 ++++-
 cli/.gitignore                                      |   2 +
 cli/LICENSE                                         | 201 ++++++++++++++++++++++
 cli/LOCAL.en.md                                     |  45 +++++
 cli/LOCAL.md                                        |  47 ++++++
 cli/README.en.md                                    | 156 +++++++++++++++++
 cli/README.md                                       | 156 +++++++++++++++++
 cli/package-lock.json                               | 550 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 cli/package.json                                    |  40 +++++
 cli/scripts/build.cjs                               |  77 +++++++++
 cli/scripts/entrypoints.cjs                         |  55 ++++++
 cli/scripts/test.cjs                                |  22 +++
 cli/scripts/verify-package.cjs                      | 124 ++++++++++++++
 cli/skills/qinglong-cli/SKILL.md                    |  21 +++
 cli/skills/qinglong-cli/references/openapi.md       | 213 +++++++++++++++++++++++
 cli/skills/qinglong-cli/references/panel.md         |  57 +++++++
 cli/skills/qinglong-local/SKILL.md                  |  19 +++
 cli/skills/qinglong-local/references/execution.md   |  35 ++++
 cli/skills/qinglong-local/references/maintenance.md |  28 ++++
 cli/src/compatibility/legacy.ts                     | 126 ++++++++++++++
 cli/src/compatibility/main.ts                       |  11 ++
 cli/src/compatibility/startup.ts                    |  50 ++++++
 cli/src/entrypoints/admin.ts                        |  21 +++
 cli/src/entrypoints/container.ts                    |  30 ++++
 cli/src/entrypoints/index.ts                        |   6 +
 cli/src/entrypoints/ql.ts                           | 224 +++++++++++++++++++++++++
 ...
 185 files changed, 22412 insertions(+), 6 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 7 or body > 48 lines).

1. `back/loaders/deps.ts:19` new, `async function linkCommandToDir(commandDir: string) {`, values `{"cc":11,"lines":73}`, ceiling cc 7, lines 48, nothing at the base matched

   ```text
   16 |   }
   17 | }
   18 | 
   19 | async function linkCommandToDir(commandDir: string) {
   20 |   const cliRoot = process.env.QL_CLI_ROOT;
   21 |   if (cliRoot) {
   22 |     if (!path.isAbsolute(cliRoot)) {
   23 |       throw new Error('QL_CLI_ROOT must be an absolute CLI installation path');
   24 |     }
   25 |     const { installCliEntrypoints } = require(path.join(
   26 |       cliRoot,
   27 |       'dist/local/entrypoints.js',
   28 |     ));
   29 |     await installCliEntrypoints(commandDir, cliRoot);
   30 |     const { installCronEntrypoint } = require(path.join(
   31 |       cliRoot,
   32 |       'dist/local/cronEntrypoint.js',
   33 |     ));
   34 |     await installCronEntrypoint(commandDir, cliRoot, config.crontabFile, {
   35 |       ...process.env,
   36 |       QL_DIR: config.rootPath,
   37 |       QL_DATA_DIR: config.dataPath,
   38 |     });
   39 |     // Container-wide legacy links may precede ~/bin. Ensure panel children use
   40 |     // the explicitly selected entries without changing system-wide links.
   41 |     process.env.PATH = [
   42 |       commandDir,
   43 |       ...(process.env.PATH?.split(path.delimiter) ?? []).filter(
   44 |         (entry) => entry !== commandDir,
   45 |       ),
   46 |     ].join(path.delimiter);
   47 |     return;
   48 |   }
   49 | 
   50 |   const cronBridge = path.join(commandDir, 'crontab');
   51 |   try {
   52 |     if (
   53 |       (await fs.lstat(cronBridge)).isFile() &&
   54 |       (await fs.readFile(cronBridge, 'utf8')).includes(
   55 |         '// QingLong CLI crontab bridge',
   56 |       )
   57 |     ) {
   58 |       await fs.unlink(cronBridge);
   59 |     }
   ```

2. `cli/src/compatibility/legacy.ts:15` new, `export function compatibilityRoute(input: string[]): CompatibilityRoute {`, values `{"cc":16,"lines":48}`, ceiling cc 7, lines 48, nothing at the base matched

   ```text
   12 |   args: string[];
   13 | }
   14 | 
   15 | export function compatibilityRoute(input: string[]): CompatibilityRoute {
   16 |   const args = input[0] === '-l' ? input.slice(1) : [...input];
   17 |   const [action, ...values] = args;
   18 |   if (action === 'repo' || action === 'raw')
   19 |     return { surface: 'subscription', args };
   20 |   if (action === 'update') {
   21 |     if (
   22 |       values.length > 1 ||
   23 |       (values.length && !['true', 'false'].includes(values[0]!))
   24 |     )
   25 |       fail(translate(process.env, '用法：ql-compat update [true|false]'), 2);
   26 |     return {
   27 |       surface: 'local',
   28 |       args: ['update', ...(values[0] === 'false' ? ['--download-only'] : [])],
   29 |     };
   30 |   }
   31 |   if (action === 'reload') {
   32 |     if (
   33 |       values.length > 1 ||
   34 |       (values.length && !['services', 'system', 'data'].includes(values[0]!))
   35 |     )
   36 |       fail(
   37 |         translate(process.env, '用法：ql-compat reload [services|system|data]'),
   38 |         2,
   39 |       );
   40 |     return {
   41 |       surface: 'local',
   42 |       args: ['reload', '--target', values[0] || 'services'],
   43 |     };
   44 |   }
   45 |   if (
   46 |     action &&
   47 |     [
   48 |       'rmlog',
   49 |       'extra',
   50 |       'bot',
   51 |       'check',
   52 |       'resetlet',
   53 |       'resettfa',
   54 |       'resetpwd',
   55 |       'resetname',
   ```

3. `cli/src/compatibility/legacy.ts:64` new, `export async function compatibilityMain(`, values `{"cc":1,"lines":58}`, ceiling cc 7, lines 48, nothing at the base matched

   ```text
   061 |   fail(translate(process.env, '未知旧命令，请运行 ql-compat --help。'), 2);
   062 | }
   063 | 
   064 | export async function compatibilityMain(
   065 |   args = process.argv.slice(2),
   066 | ): Promise<number> {
   067 |   return withCommandCancellation(async (signal) => {
   068 |     try {
   069 |       if (
   070 |         !args.length ||
   071 |         (args.length === 1 && ['--help', '-h'].includes(args[0]!))
   072 |       ) {
   073 |         process.stdout.write(standaloneHelp('compat'));
   074 |         return 0;
   075 |       }
   076 |       const route = compatibilityRoute(args);
   077 |       if (route.surface === 'subscription') {
   078 |         const { subscriptionWorker } = await import(
   079 |           '../internal/subscription/worker'
   080 |         );
   081 |         return subscriptionWorker(route.args);
   082 |       }
   083 |       const { parse } = await import('../shared/cli/arguments');
   084 |       const { localContext } = await import('../internal/runtime/context');
   085 |       const { maintenance } = await import(
   086 |         '../internal/maintenance/maintenance'
   087 |       );
   088 |       const { loggedOperation } = await import(
   089 |         '../internal/runtime/commandLog'
   090 |       );
   091 |       const { registry } = await import('../internal/commands/registry');
   092 |       const command = parse(route.args, registry);
   093 |       const context = await localContext(command, { signal });
   094 |       const { result, logPath } = await loggedOperation(
   095 |         context,
   096 |         route.args[0]!,
   097 |         () => maintenance(command, context),
   098 |         signal,
   099 |       );
   100 |       process.stdout.write(
   101 |         JSON.stringify({ code: 200, data: result, logPath }) + '\n',
   102 |       );
   103 |       return 0;
   104 |     } catch (error) {
   ```

4. `cli/src/compatibility/legacy.ts:67` new, `return withCommandCancellation(async (signal) => {`, values `{"cc":10,"lines":54}`, ceiling cc 7, lines 48, nothing at the base matched
5. `cli/src/entrypoints/ql.ts:47` new, `export function operatorArguments(args: string[]): string[] {`, values `{"cc":8,"lines":11}`, ceiling cc 7, lines 48, nothing at the base matched
6. `cli/src/entrypoints/ql.ts:72` new, `function groupHelp(group: 'root' | 'task' | 'local'): string {`, values `{"cc":17,"lines":43}`, ceiling cc 7, lines 48, nothing at the base matched
7. `cli/src/entrypoints/ql.ts:155` new, `export async function qlMain(args = process.argv.slice(2)): Promise<number> {`, values `{"cc":5,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
8. `cli/src/internal/execution/runner.ts:12` new, `export function parseExecution(args: string[]): {`, values `{"cc":12,"lines":37}`, ceiling cc 7, lines 48, nothing at the base matched
9. `cli/src/internal/execution/runner.ts:50` new, `export async function runnerMain(`, values `{"cc":21,"lines":87}`, ceiling cc 7, lines 48, nothing at the base matched
10. `cli/src/internal/execution/shellSession.ts:5` new, `export function shellSessionArguments(`, values `{"cc":3,"lines":94}`, ceiling cc 7, lines 48, nothing at the base matched
11. `cli/src/internal/execution/taskRunner.ts:40` new, `export function selectedAccounts(`, values `{"cc":9,"lines":26}`, ceiling cc 7, lines 48, nothing at the base matched
12. `cli/src/internal/execution/taskRunner.ts:116` new, `async function prepareProgram(`, values `{"cc":18,"lines":83}`, ceiling cc 7, lines 48, nothing at the base matched
13. `cli/src/internal/execution/taskRunner.ts:200` new, `export async function executeTask(`, values `{"cc":60,"lines":343}`, ceiling cc 7, lines 48, nothing at the base matched
14. `cli/src/internal/execution/taskRunner.ts:249` new, `const report = async (final: boolean, code = 0, duration = 0) => {`, values `{"cc":10,"lines":36}`, ceiling cc 7, lines 48, nothing at the base matched
15. `cli/src/internal/execution/taskRunner.ts:396` new, `const invoke = async (selection?: number[], sink = output) => {`, values `{"cc":16,"lines":52}`, ceiling cc 7, lines 48, nothing at the base matched
16. `cli/src/internal/integration/cronEntrypoint.ts:20` new, `export function runCrontab(`, values `{"cc":16,"lines":85}`, ceiling cc 7, lines 48, nothing at the base matched
17. `cli/src/internal/integration/cronEntrypoint.ts:106` new, `export async function installCronEntrypoint(`, values `{"cc":14,"lines":70}`, ceiling cc 7, lines 48, nothing at the base matched
18. `cli/src/internal/maintenance/bootstrap.ts:12` new, `export function bootstrapPackages(`, values `{"cc":5,"lines":58}`, ceiling cc 7, lines 48, nothing at the base matched
19. `cli/src/internal/maintenance/bootstrap.ts:160` new, `export async function bootstrapPanel(`, values `{"cc":20,"lines":100}`, ceiling cc 7, lines 48, nothing at the base matched
20. `cli/src/internal/maintenance/bot.ts:58` new, `async function validateBotTree(`, values `{"cc":8,"lines":25}`, ceiling cc 7, lines 48, nothing at the base matched
21. `cli/src/internal/maintenance/bot.ts:84` new, `export async function prepareBot(context: LocalContext): Promise<{`, values `{"cc":13,"lines":85}`, ceiling cc 7, lines 48, nothing at the base matched
22. `cli/src/internal/maintenance/bot.ts:170` new, `export async function stopBot(context: LocalContext): Promise<void> {`, values `{"cc":13,"lines":31}`, ceiling cc 7, lines 48, nothing at the base matched
23. `cli/src/internal/maintenance/check.ts:21` new, `async function probe(`, values `{"cc":1,"lines":53}`, ceiling cc 7, lines 48, nothing at the base matched
24. `cli/src/internal/maintenance/check.ts:91` new, `probe(port, '${basePath}/api/health?t=${Math.floor(Date.now() / 1000)}', (body) => {`, values `{"cc":10,"lines":16}`, ceiling cc 7, lines 48, nothing at the base matched
25. `cli/src/internal/maintenance/check.ts:111` new, `export async function diagnosticLog(file: string): Promise<{`, values `{"cc":8,"lines":42}`, ceiling cc 7, lines 48, nothing at the base matched
26. `cli/src/internal/maintenance/maintenance.ts:9` new, `export async function pruneLogs(`, values `{"cc":9,"lines":40}`, ceiling cc 7, lines 48, nothing at the base matched
27. `cli/src/internal/maintenance/maintenance.ts:50` new, `export async function maintenance(`, values `{"cc":16,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
28. `cli/src/internal/maintenance/operator.ts:88` new, `async function stopDirectBackend(context: LocalContext): Promise<void> {`, values `{"cc":15,"lines":42}`, ceiling cc 7, lines 48, nothing at the base matched
29. `cli/src/internal/maintenance/operator.ts:140` new, `export async function startPanel(`, values `{"cc":6,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
30. `cli/src/internal/maintenance/upgrade.ts:19` new, `async function validateTree(`, values `{"cc":10,"lines":27}`, ceiling cc 7, lines 48, nothing at the base matched
31. `cli/src/internal/maintenance/upgrade.ts:47` new, `export async function stageUpgrade(`, values `{"cc":12,"lines":91}`, ceiling cc 7, lines 48, nothing at the base matched
32. `cli/src/internal/maintenance/upgrade.ts:139` new, `async function selectedUpgrade(`, values `{"cc":9,"lines":41}`, ceiling cc 7, lines 48, nothing at the base matched
33. `cli/src/internal/maintenance/upgrade.ts:188` new, `export async function replaceAndReload(`, values `{"cc":18,"lines":115}`, ceiling cc 7, lines 48, nothing at the base matched
34. `cli/src/internal/maintenance/upgrade.ts:304` new, `export async function reloadPanel(`, values `{"cc":15,"lines":76}`, ceiling cc 7, lines 48, nothing at the base matched
35. `cli/src/internal/maintenance/upgradeSelection.ts:8` new, `export async function prepareUpgradeSelection(`, values `{"cc":13,"lines":58}`, ceiling cc 7, lines 48, nothing at the base matched
36. `cli/src/internal/runtime/api.ts:14` new, `private async credential(): Promise<string> {`, values `{"cc":6,"lines":49}`, ceiling cc 7, lines 48, nothing at the base matched
37. `cli/src/internal/runtime/api.ts:64` new, `async call(`, values `{"cc":11,"lines":58}`, ceiling cc 7, lines 48, nothing at the base matched
38. `cli/src/internal/runtime/backendProcesses.ts:6` new, `export async function findDirectBackendPids(`, values `{"cc":15,"lines":46}`, ceiling cc 7, lines 48, nothing at the base matched
39. `cli/src/internal/runtime/commandLog.ts:14` new, `export async function loggedOperation<T>(`, values `{"cc":12,"lines":104}`, ceiling cc 7, lines 48, nothing at the base matched
40. `cli/src/internal/runtime/commandLog.ts:54` new, `const report = async (final: boolean, code: number) => {`, values `{"cc":9,"lines":21}`, ceiling cc 7, lines 48, nothing at the base matched
41. `cli/src/internal/runtime/containerEnvironment.ts:24` new, `async function requireContainerDirectory(`, values `{"cc":8,"lines":25}`, ceiling cc 7, lines 48, nothing at the base matched
42. `cli/src/internal/runtime/containerEnvironment.ts:50` new, `async function appendNetworkEntries(`, values `{"cc":8,"lines":38}`, ceiling cc 7, lines 48, nothing at the base matched
43. `cli/src/internal/runtime/containerEnvironment.ts:90` new, `export async function prepareContainerEnvironment(`, values `{"cc":7,"lines":55}`, ceiling cc 7, lines 48, nothing at the base matched
44. `cli/src/internal/runtime/containerRuntime.ts:27` new, `async function stopStartupGroups(pids: number[]): Promise<void> {`, values `{"cc":8,"lines":21}`, ceiling cc 7, lines 48, nothing at the base matched
45. `cli/src/internal/runtime/containerRuntime.ts:50` new, `export async function runContainer(`, values `{"cc":4,"lines":116}`, ceiling cc 7, lines 48, nothing at the base matched
46. `cli/src/internal/runtime/containerRuntime.ts:69` new, `return await cancellableOperation(signal, async () => {`, values `{"cc":13,"lines":77}`, ceiling cc 7, lines 48, nothing at the base matched
47. `cli/src/internal/runtime/context.ts:15` new, `export function createContext(`, values `{"cc":12,"lines":88}`, ceiling cc 7, lines 48, nothing at the base matched
48. `cli/src/internal/runtime/context.ts:107` new, `export async function sourceEnvironment(`, values `{"cc":12,"lines":90}`, ceiling cc 7, lines 48, nothing at the base matched
49. `cli/src/internal/runtime/lifecycle.ts:7` new, `export async function extendedLifecycle(`, values `{"cc":15,"lines":45}`, ceiling cc 7, lines 48, nothing at the base matched
50. `cli/src/internal/runtime/process.ts:44` new, `function signalGroup(child: ChildProcess, signal: NodeJS.Signals): void {`, values `{"cc":9,"lines":20}`, ceiling cc 7, lines 48, nothing at the base matched
51. `cli/src/internal/runtime/process.ts:65` new, `export async function runProcess(`, values `{"cc":10,"lines":183}`, ceiling cc 7, lines 48, nothing at the base matched
52. `cli/src/internal/runtime/process.ts:93` new, `return new Promise((resolve, reject) => {`, values `{"cc":10,"lines":154}`, ceiling cc 7, lines 48, nothing at the base matched
53. `cli/src/internal/runtime/process.ts:210` new, `child.once('close', (code, signal) => {`, values `{"cc":11,"lines":33}`, ceiling cc 7, lines 48, nothing at the base matched
54. `cli/src/internal/subscription/subscriptionRunner.ts:104` new, `export async function replaceDirectory(`, values `{"cc":8,"lines":35}`, ceiling cc 7, lines 48, nothing at the base matched
55. `cli/src/internal/subscription/subscriptionRunner.ts:140` new, `async function reconcile(`, values `{"cc":19,"lines":93}`, ceiling cc 7, lines 48, nothing at the base matched
56. `cli/src/internal/subscription/subscriptionRunner.ts:234` new, `export async function syncRepository(`, values `{"cc":14,"lines":100}`, ceiling cc 7, lines 48, nothing at the base matched
57. `cli/src/internal/subscription/subscriptionRunner.ts:335` new, `export async function syncRaw(`, values `{"cc":4,"lines":52}`, ceiling cc 7, lines 48, nothing at the base matched
58. `cli/src/internal/subscription/worker.ts:16` new, `export async function subscriptionWorker(`, values `{"cc":1,"lines":92}`, ceiling cc 7, lines 48, nothing at the base matched
59. `cli/src/internal/subscription/worker.ts:19` new, `return withCommandCancellation(async (signal) => {`, values `{"cc":17,"lines":88}`, ceiling cc 7, lines 48, nothing at the base matched
60. `cli/src/remote/api/client.ts:18` new, `export async function request(`, values `{"cc":34,"lines":106}`, ceiling cc 7, lines 48, nothing at the base matched
61. `cli/src/remote/auth/store.ts:17` new, `export function readConfig(): StoredConfig {`, values `{"cc":18,"lines":55}`, ceiling cc 7, lines 48, nothing at the base matched
62. `cli/src/remote/auth/url.ts:4` new, `export function panelUrl(value: string): string {`, values `{"cc":9,"lines":33}`, ceiling cc 7, lines 48, nothing at the base matched
63. `cli/src/remote/commands/auth.ts:52` new, `export async function session(): Promise<Session> {`, values `{"cc":9,"lines":28}`, ceiling cc 7, lines 48, nothing at the base matched
64. `cli/src/remote/commands/auth.ts:81` new, `export async function auth(`, values `{"cc":9,"lines":43}`, ceiling cc 7, lines 48, nothing at the base matched
65. `cli/src/remote/commands/dispatch.ts:8` new, `export async function dispatchRemote(`, values `{"cc":9,"lines":39}`, ceiling cc 7, lines 48, nothing at the base matched
66. `cli/src/remote/commands/open.ts:20` new, `async function input(value: string | boolean | undefined): Promise<unknown> {`, values `{"cc":10,"lines":31}`, ceiling cc 7, lines 48, nothing at the base matched
67. `cli/src/remote/commands/open.ts:80` new, `export async function openCommand(`, values `{"cc":66,"lines":173}`, ceiling cc 7, lines 48, nothing at the base matched
68. `cli/src/remote/commands/openCommands.ts:10` new, `.map((op) => {`, values `{"cc":14,"lines":60}`, ceiling cc 7, lines 48, nothing at the base matched
69. `cli/src/remote/commands/subscription.ts:47` new, `export async function subscription(`, values `{"cc":11,"lines":45}`, ceiling cc 7, lines 48, nothing at the base matched
70. `cli/src/remote/commands/task.ts:22` new, `export async function task(`, values `{"cc":18,"lines":55}`, ceiling cc 7, lines 48, nothing at the base matched
71. `cli/src/shared/cli/arguments.ts:37` new, `export function parse(args: string[], registry: CommandRegistry): Invocation {`, values `{"cc":32,"lines":115}`, ceiling cc 7, lines 48, nothing at the base matched
72. `cli/src/shared/cli/invoke.ts:7` new, `export async function invoke(`, values `{"cc":12,"lines":43}`, ceiling cc 7, lines 48, nothing at the base matched
73. `cli/src/shared/cli/options.ts:59` new, `export function assertOptionValues(`, values `{"cc":15,"lines":32}`, ceiling cc 7, lines 48, nothing at the base matched
74. `cli/src/shared/cli/registry.ts:40` new, `export function helpFor(`, values `{"cc":9,"lines":54}`, ceiling cc 7, lines 48, nothing at the base matched
75. `cli/test/compatibility/compat.test.cjs:79` new, `test('legacy maintenance logs output and reports lifecycle without sourcing config twice', async (t) => {`, values `{"cc":8,"lines":98}`, ceiling cc 7, lines 48, nothing at the base matched
76. `cli/test/compatibility/compat.test.cjs:178` new, `test('raw worker and compatibility route share subscription logs and lifecycle', async (t) => {`, values `{"cc":6,"lines":105}`, ceiling cc 7, lines 48, nothing at the base matched
77. `cli/test/compatibility/compat.test.cjs:287` new, `async (t) => {`, values `{"cc":12,"lines":130}`, ceiling cc 7, lines 48, nothing at the base matched
78. `cli/test/integration/differential.test.cjs:10` new, `test('unmodified legacy Shell and TS agree on shell hook state and account modes', async (t) => {`, values `{"cc":5,"lines":245}`, ceiling cc 7, lines 48, nothing at the base matched
79. `cli/test/integration/differential.test.cjs:110` new, `async () => {`, values `{"cc":5,"lines":78}`, ceiling cc 7, lines 48, nothing at the base matched
80. `cli/test/integration/differential.test.cjs:189` new, `await t.test('/dev/null preserves legacy stream routing', async () => {`, values `{"cc":6,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
81. `cli/test/integration/differential.test.cjs:256` new, `test('empty ignored-minute settings match legacy random delay at minute zero', async (t) => {`, values `{"cc":3,"lines":60}`, ceiling cc 7, lines 48, nothing at the base matched
82. `cli/test/integration/differential.test.cjs:317` new, `test('random delay follows legacy dispatch for script arguments and executable commands', async (t) => {`, values `{"cc":2,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
83. `cli/test/integration/differential.test.cjs:472` new, `test('later hooks can disable previously enabled shell options without mutating the prior snapshot', async (t) => {`, values `{"cc":1,"lines":55}`, ceiling cc 7, lines 48, nothing at the base matched
84. `cli/test/integration/executablePaths.test.cjs:8` new, `test('legacy Shell and TS resolve executable paths after selecting the working directory', t => {`, values `{"cc":16,"lines":84}`, ceiling cc 7, lines 48, nothing at the base matched
85. `cli/test/integration/loadingBoundary.test.cjs:8` new, `test('standalone public commands load only public modules and never local operators or backend dependencies', async (t) => {`, values `{"cc":7,"lines":66}`, ceiling cc 7, lines 48, nothing at the base matched
86. `cli/test/integration/stdin.test.cjs:8` new, `test('legacy Shell and TS preserve task stdin in each execution mode', t => {`, values `{"cc":8,"lines":39}`, ceiling cc 7, lines 48, nothing at the base matched
87. `cli/test/integration/unified.test.cjs:9` new, `test('unified ql groups and task shortcut preserve help, errors and local execution boundaries', async (t) => {`, values `{"cc":8,"lines":90}`, ceiling cc 7, lines 48, nothing at the base matched
88. `cli/test/internal/backendProcesses.test.cjs:63` new, `async (t) => {`, values `{"cc":4,"lines":78}`, ceiling cc 7, lines 48, nothing at the base matched
89. `cli/test/internal/backendProcesses.test.cjs:149` new, `async (t) => {`, values `{"cc":2,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
90. `cli/test/internal/backendProcesses.test.cjs:155` new, `await t.test(mode, async () => {`, values `{"cc":6,"lines":57}`, ceiling cc 7, lines 48, nothing at the base matched
91. `cli/test/internal/bootstrap.test.cjs:13` new, `async function fixture(t) {`, values `{"cc":3,"lines":70}`, ceiling cc 7, lines 48, nothing at the base matched
92. `cli/test/internal/bootstrap.test.cjs:84` new, `test('bootstrap installs prerequisites, uses the configured data directory and starts services in order', async (t) => {`, values `{"cc":3,"lines":71}`, ceiling cc 7, lines 48, nothing at the base matched
93. `cli/test/internal/bot.test.cjs:16` new, `async function fixture(t) {`, values `{"cc":3,"lines":49}`, ceiling cc 7, lines 48, nothing at the base matched
94. `cli/test/internal/bot.test.cjs:66` new, `test('bot prepares local repository and dependencies while preserving user configuration', async (t) => {`, values `{"cc":2,"lines":56}`, ceiling cc 7, lines 48, nothing at the base matched
95. `cli/test/internal/containerRuntime.test.cjs:181` new, `async (t) => {`, values `{"cc":6,"lines":73}`, ceiling cc 7, lines 48, nothing at the base matched
96. `cli/test/internal/contextDiagnostics.test.cjs:9` new, `test('all local entrypoints report missing, relative and non-directory roots before doing work', async (t) => {`, values `{"cc":7,"lines":56}`, ceiling cc 7, lines 48, nothing at the base matched
97. `cli/test/internal/cronEntrypoint.test.cjs:9` new, `test('cron bridge scopes environment to the panel table and preserves native reads, failures and unrelated tables', async (t) => {`, values `{"cc":1,"lines":72}`, ceiling cc 7, lines 48, nothing at the base matched
98. `cli/test/internal/esmDependencies.test.cjs:9` new, `test('ESM resolves global packages and exported subpaths with native import conditions and local fallback', async (t) => {`, values `{"cc":5,"lines":144}`, ceiling cc 7, lines 48, nothing at the base matched
99. `cli/test/internal/helperDiagnostics.test.cjs:13` new, `test('helper validation and generated wrappers honor locale: ${language}', async (t) => {`, values `{"cc":3,"lines":58}`, ceiling cc 7, lines 48, nothing at the base matched
100. `cli/test/internal/hostServices.test.cjs:10` new, `test('host startup registration uses ${manager} and propagates service failures', async (t) => {`, values `{"cc":10,"lines":62}`, ceiling cc 7, lines 48, nothing at the base matched
101. `cli/test/internal/languagePreload.test.cjs:10` new, `test('language hooks receive literal script arguments and temporary adapters are removed', async (t) => {`, values `{"cc":5,"lines":81}`, ceiling cc 7, lines 48, nothing at the base matched
102. `cli/test/internal/local.test.cjs:687` new, `test('lifecycle reports share execution identity and statistics survive a rejected final status', async (t) => {`, values `{"cc":1,"lines":72}`, ceiling cc 7, lines 48, nothing at the base matched
103. `cli/test/internal/local.test.cjs:827` new, `test('cancelling after hooks preserves the signal and final task status', async (t) => {`, values `{"cc":3,"lines":57}`, ceiling cc 7, lines 48, nothing at the base matched
104. `cli/test/internal/local.test.cjs:831` new, `await t.test('${extension}: ${signal}', async () => {`, values `{"cc":2,"lines":50}`, ceiling cc 7, lines 48, nothing at the base matched
105. `cli/test/internal/localApi.test.cjs:41` new, `test('local API preserves every legacy request shape and JSON special characters', async (t) => {`, values `{"cc":2,"lines":63}`, ceiling cc 7, lines 48, nothing at the base matched
106. `cli/test/internal/localApi.test.cjs:236` new, `test('local API errors preserve language, status and no-replay behavior: ${language}', async (t) => {`, values `{"cc":1,"lines":53}`, ceiling cc 7, lines 48, nothing at the base matched
107. `cli/test/internal/logRetention.test.cjs:11` new, `test('undated logs use modification calendar date and preserve active references at retention boundary', async (t) => {`, values `{"cc":2,"lines":66}`, ceiling cc 7, lines 48, nothing at the base matched
108. `cli/test/internal/networkRepository.test.cjs:21` new, `async (t) => {`, values `{"cc":20,"lines":314}`, ceiling cc 7, lines 48, nothing at the base matched
109. `cli/test/internal/networkRepository.test.cjs:85` new, `const serveGit = (req, res) => {`, values `{"cc":5,"lines":68}`, ceiling cc 7, lines 48, nothing at the base matched
110. `cli/test/internal/operator.test.cjs:196` new, `test('upgrade staging extracts both archives before publishing readiness and rejects archive symlinks', async (t) => {`, values `{"cc":2,"lines":90}`, ceiling cc 7, lines 48, nothing at the base matched
111. `cli/test/internal/operator.test.cjs:287` new, `test('check repairs dependencies and notifications, probes loopback and reloads with clean diagnostics', async (t) => {`, values `{"cc":2,"lines":81}`, ceiling cc 7, lines 48, nothing at the base matched
112. `cli/test/internal/operator.test.cjs:461` new, `test('interrupted reload restores old files and recovery subprocesses ignore the cancelled operation', async (t) => {`, values `{"cc":5,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
113. `cli/test/internal/operator.test.cjs:598` new, `test('partial overlay backup, removal and replacement failures restore the complete previous installation', async (t) => {`, values `{"cc":2,"lines":96}`, ceiling cc 7, lines 48, nothing at the base matched
114. `cli/test/internal/operator.test.cjs:600` new, `await t.test(phase, async () => {`, values `{"cc":4,"lines":92}`, ceiling cc 7, lines 48, nothing at the base matched
115. `cli/test/internal/operatorDiagnostics.test.cjs:12` new, `test('upgrade failures preserve files and localize using operation language: ${language}', async (t) => {`, values `{"cc":1,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
116. `cli/test/internal/operatorDiagnostics.test.cjs:78` new, `test('installed cron bridge localizes failure without exposing values: ${language}', async (t) => {`, values `{"cc":4,"lines":53}`, ceiling cc 7, lines 48, nothing at the base matched
117. `cli/test/internal/pathDiagnostics.test.cjs:15` new, `test('path and subscription failures use their own operation language: ${language}', async (t) => {`, values `{"cc":2,"lines":68}`, ceiling cc 7, lines 48, nothing at the base matched
118. `cli/test/internal/preload.test.cjs:10` new, `test('real language preloaders inject generated environment, hooks and designated accounts', async (t) => {`, values `{"cc":5,"lines":102}`, ceiling cc 7, lines 48, nothing at the base matched
119. `cli/test/internal/processDiagnostics.test.cjs:6` new, `test('subprocess failures localize and reap failed writers: ${language}', async () => {`, values `{"cc":4,"lines":59}`, ceiling cc 7, lines 48, nothing at the base matched
120. `cli/test/internal/runner.test.cjs:9` new, `test('cancelling random delay returns final JSON without starting the Shell task', async (t) => {`, values `{"cc":2,"lines":78}`, ceiling cc 7, lines 48, nothing at the base matched
121. `cli/test/internal/runner.test.cjs:18` new, `await t.test(signal, async () => {`, values `{"cc":1,"lines":67}`, ceiling cc 7, lines 48, nothing at the base matched
122. `cli/test/internal/runner.test.cjs:160` new, `test('runner signals cancel a live Shell session and emit a final JSON result', async (t) => {`, values `{"cc":2,"lines":94}`, ceiling cc 7, lines 48, nothing at the base matched
123. `cli/test/internal/runner.test.cjs:169` new, `await t.test(signal, async () => {`, values `{"cc":2,"lines":83}`, ceiling cc 7, lines 48, nothing at the base matched
124. `cli/test/internal/runner.test.cjs:255` new, `test('configuration and independent before hooks cancel without starting the task', async (t) => {`, values `{"cc":3,"lines":106}`, ceiling cc 7, lines 48, nothing at the base matched
125. `cli/test/internal/runner.test.cjs:265` new, `await t.test('${phase}: ${signal}', async () => {`, values `{"cc":3,"lines":93}`, ceiling cc 7, lines 48, nothing at the base matched
126. `cli/test/internal/scriptInventory.test.cjs:10` new, `test('no-argument runner lists legacy JS candidates without evaluating scripts or config', async (t) => {`, values `{"cc":1,"lines":63}`, ceiling cc 7, lines 48, nothing at the base matched
127. `cli/test/internal/subscriptionFilter.test.cjs:9` new, `test('subscription filters preserve legacy POSIX classes, escaping, anchors and option-like patterns', async (t) => {`, values `{"cc":4,"lines":51}`, ceiling cc 7, lines 48, nothing at the base matched
128. `cli/test/internal/subscriptionRunner.test.cjs:32` new, `test('local repository sync copies selected scripts/dependencies and preserves last good version on clone failure', async (t) => {`, values `{"cc":2,"lines":56}`, ceiling cc 7, lines 48, nothing at the base matched
129. `cli/test/internal/subscriptionRunner.test.cjs:89` new, `test('subscription reconciliation scopes removals to canonical paths and the current subscription', async (t) => {`, values `{"cc":1,"lines":92}`, ceiling cc 7, lines 48, nothing at the base matched
130. `cli/test/internal/subscriptionRunner.test.cjs:280` new, `test('subscription copy precedence preserves local dependencies while selected scripts win', async (t) => {`, values `{"cc":2,"lines":107}`, ceiling cc 7, lines 48, nothing at the base matched
131. `cli/test/internal/taskDelay.test.cjs:11` new, `test('delay extension ERE matching agrees with unchanged legacy function', async () => {`, values `{"cc":3,"lines":52}`, ceiling cc 7, lines 48, nothing at the base matched
132. `cli/test/internal/taskDiagnostics.test.cjs:15` new, `test('task validation exposes localized JSON and never starts invalid tasks: ${language}', async (t) => {`, values `{"cc":3,"lines":80}`, ceiling cc 7, lines 48, nothing at the base matched
133. `cli/test/internal/taskLifecycle.test.cjs:41` new, `for (const scenario of cases) await t.test(scenario.name, t => {`, values `{"cc":10,"lines":18}`, ceiling cc 7, lines 48, nothing at the base matched
134. `cli/test/linux/bot-install.cjs:13` new, `(async () => {`, values `{"cc":3,"lines":96}`, ceiling cc 7, lines 48, nothing at the base matched
135. `cli/test/linux/container-bot.test.cjs:32` new, `async (t) => {`, values `{"cc":4,"lines":109}`, ceiling cc 7, lines 48, nothing at the base matched
136. `cli/test/linux/container-cron.cjs:10` new, `(async () => {`, values `{"cc":6,"lines":86}`, ceiling cc 7, lines 48, nothing at the base matched
137. `cli/test/linux/entrypoints.test.cjs:11` new, `test('2.x startup selection survives repeated linking and can return to original Shell', async (t) => {`, values `{"cc":6,"lines":130}`, ceiling cc 7, lines 48, nothing at the base matched
138. `cli/test/linux/evaluation-entrypoints.cjs:6` new, `async function installEvaluationEntrypoints() {`, values `{"cc":6,"lines":52}`, ceiling cc 7, lines 48, nothing at the base matched
139. `cli/test/linux/host-boot.cjs:9` new, `(async () => {`, values `{"cc":16,"lines":98}`, ceiling cc 7, lines 48, nothing at the base matched
140. `cli/test/linux/image-selection.cjs:7` new, `(async () => {`, values `{"cc":6,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
141. `cli/test/linux/mounted-data.test.cjs:13` new, `async (t) => {`, values `{"cc":1,"lines":62}`, ceiling cc 7, lines 48, nothing at the base matched
142. `cli/test/linux/openapi.cjs:8` new, `(async () => {`, values `{"cc":2,"lines":54}`, ceiling cc 7, lines 48, nothing at the base matched
143. `cli/test/linux/panel-account.cjs:7` new, `exports.verifyAccountMaintenance = async function verifyAccountMaintenance(`, values `{"cc":1,"lines":50}`, ceiling cc 7, lines 48, nothing at the base matched
144. `cli/test/linux/panel-operator.cjs:7` new, `exports.verifyOperator = async function verifyOperator(api) {`, values `{"cc":4,"lines":63}`, ceiling cc 7, lines 48, nothing at the base matched
145. `cli/test/linux/panel-preload.cjs:10` new, `(async () => {`, values `{"cc":16,"lines":129}`, ceiling cc 7, lines 48, nothing at the base matched
146. `cli/test/linux/panel-repository.cjs:8` new, `exports.verifyRepository = async function verifyRepository(api, cli) {`, values `{"cc":6,"lines":125}`, ceiling cc 7, lines 48, nothing at the base matched
147. `cli/test/linux/panel-subscription.cjs:5` new, `exports.verifySubscription = async function verifySubscription(api, cli) {`, values `{"cc":8,"lines":65}`, ceiling cc 7, lines 48, nothing at the base matched
148. `cli/test/linux/panel-upgrade.cjs:8` new, `(async () => {`, values `{"cc":7,"lines":106}`, ceiling cc 7, lines 48, nothing at the base matched
149. `cli/test/linux/panel.cjs:11` new, `(async () => {`, values `{"cc":16,"lines":204}`, ceiling cc 7, lines 48, nothing at the base matched
150. `cli/test/linux/published-archive.test.cjs:18` new, `async (t) => {`, values `{"cc":3,"lines":80}`, ceiling cc 7, lines 48, nothing at the base matched
151. `cli/test/linux/services.test.cjs:14` new, `async (t) => {`, values `{"cc":2,"lines":87}`, ceiling cc 7, lines 48, nothing at the base matched
152. `cli/test/linux/ssh-repository.test.cjs:15` new, `async (t) => {`, values `{"cc":9,"lines":133}`, ceiling cc 7, lines 48, nothing at the base matched
153. `cli/test/linux/upgrade.test.cjs:14` new, `async (t) => {`, values `{"cc":2,"lines":87}`, ceiling cc 7, lines 48, nothing at the base matched
154. `cli/test/linux/upgrade.test.cjs:16` new, `await t.test(manager, async (t) => {`, values `{"cc":3,"lines":83}`, ceiling cc 7, lines 48, nothing at the base matched
155. `cli/test/remote/apiDiagnostics.test.cjs:5` new, `test('API diagnostics preserve resource scope, uncertainty, codes and secret suppression in both languages', async (t) => {`, values `{"cc":10,"lines":87}`, ceiling cc 7, lines 48, nothing at the base matched
156. `cli/test/remote/apiDiagnostics.test.cjs:44` new, `(error) => {`, values `{"cc":8,"lines":28}`, ceiling cc 7, lines 48, nothing at the base matched
157. `cli/test/remote/cli.test.cjs:10` new, `async function fixture(t) {`, values `{"cc":1,"lines":130}`, ceiling cc 7, lines 48, nothing at the base matched
158. `cli/test/remote/cli.test.cjs:15` new, `const server = http.createServer(async (req, res) => {`, values `{"cc":15,"lines":83}`, ceiling cc 7, lines 48, nothing at the base matched
159. `cli/test/remote/cli.test.cjs:175` new, `test('list pagination, exact task operations and log tail preserve the 2.x API contract', async (t) => {`, values `{"cc":2,"lines":52}`, ceiling cc 7, lines 48, nothing at the base matched
160. `cli/test/remote/openapi.test.cjs:105` new, `test('all added named operations reach their registered method/path, including upload/download and anonymous routes', async t => {`, values `{"cc":12,"lines":22}`, ceiling cc 7, lines 48, nothing at the base matched
161. `cli/test/remote/rawCredentials.test.cjs:12` new, `test('raw subscriptions retain legacy netrc authentication and preserve files on denied access', async (t) => {`, values `{"cc":2,"lines":74}`, ceiling cc 7, lines 48, nothing at the base matched
162. `cli/test/remote/requestBodyCoverage.test.cjs:16` new, `const visit = node => {`, values `{"cc":9,"lines":15}`, ceiling cc 7, lines 48, nothing at the base matched
163. `cli/test/remote/subscription.test.cjs:27` new, `test('subscription management uses panel API, projects credentials out and never invokes local repo/raw', async (t) => {`, values `{"cc":2,"lines":100}`, ceiling cc 7, lines 48, nothing at the base matched
164. `cli/test/shared/diagnostics.test.cjs:8` new, `test('legacy and worker entrypoints localize invalid input before accessing an installation', () => {`, values `{"cc":4,"lines":79}`, ceiling cc 7, lines 48, nothing at the base matched
165. `cli/test/shared/diagnostics.test.cjs:186` new, `test('legacy and worker failures preserve stderr diagnostics and localize the final JSON', async (t) => {`, values `{"cc":4,"lines":54}`, ceiling cc 7, lines 48, nothing at the base matched
166. `cli/test/shared/helpCoverage.test.cjs:14` new, `test('every registered command and option is discoverable in both help languages', () => {`, values `{"cc":10,"lines":49}`, ceiling cc 7, lines 48, nothing at the base matched
167. `cli/test/shared/i18n.test.cjs:113` new, `test('task, maintenance and subscription failure warnings honor locale without failing completed work', async (t) => {`, values `{"cc":8,"lines":103}`, ceiling cc 7, lines 48, nothing at the base matched

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R049

- Repository: `apollographql/apollo-client` (TypeScript), change 4 of 10
- Commit: `37f700eb4c65`, judged against its first parent `d4f877012044`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at the floor; lines at a derived percentile, the base site already over
- Derived cc: `5`, the floor of 5, over 13,259 function(s) at d4f8770; recorded scope: whole repository
- Derived lines: `108`, 95th percentile of 13,259 functions at d4f8770, floor 25; recorded scope: whole repository

### Commit message

> Force rerender `useLazyQuery` when variables change while request is in-flight (#13464)
>
> Fixes #13459
>
> Force rerenders `useLazyQuery` when `execute` is called while a network
> request is in-flight so that the returned `variables` are set
> immediately. `ObservableQuery` purposely drops emits when the value
> would otherwise be deep equal to the previous emit and because of this,
> `useLazyQuery` didn't get an emit that would have rerendered the new
> variables. `variables` are not tracked in the emitted `ObservableQuery`
> result.
>
> <!-- This is an auto-generated comment: release notes by coderabbit.ai
> -->
>
> ## Summary by CodeRabbit
>
> - **Bug Fixes**
> - Improved lazy queries when executed with new variables while a
> previous request is still in progress.
> - Results now immediately reflect the latest variables and correctly
> transition to the completed state.
> - Updated query behavior when variables change during an in-flight
> request, preserving accurate loading and result information.
>
> - **Tests**
> - Added coverage for variable changes during active requests, including
> cancellation, updated results, network status, and render behavior.
>
> <!-- end of auto-generated comment: release notes by coderabbit.ai -->

### Files the change touched

```text
 .changeset/brown-swans-hammer.md                |   5 ++
 .size-limits.json                               |   8 ++--
 src/react/hooks/__tests__/useLazyQuery.test.tsx | 167 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/react/hooks/__tests__/useQuery.test.tsx     |  64 +++++++++++++++++++++++++
 src/react/hooks/useLazyQuery.ts                 |  33 ++++++++++++-
 5 files changed, 272 insertions(+), 5 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 5 or body > 108 lines).

1. `src/react/hooks/useLazyQuery.ts:616` new, `(...args) => {`, values `{"cc":7,"lines":51}`, ceiling cc 5, lines 108, nothing at the base matched

   ```text
   613 | 
   614 |   const execute: useLazyQuery.ExecFunction<TData, TVariables> =
   615 |     React.useCallback(
   616 |       (...args) => {
   617 |         invariant(
   618 |           !calledDuringRender(),
   619 |           "useLazyQuery: 'execute' should not be called during render. To start a query during render, use the 'useQuery' hook."
   620 |         );
   621 | 
   622 |         const [executeOptions] = args;
   623 | 
   624 |         let fetchPolicy = observable.options.fetchPolicy;
   625 | 
   626 |         if (fetchPolicy === "standby") {
   627 |           fetchPolicy = observable.options.initialFetchPolicy;
   628 |         }
   629 | 
   630 |         const previousResult = resultRef.current;
   631 |         const previousVariables = observable.variables;
   632 | 
   633 |         const promise = observable.reobserve({
   634 |           fetchPolicy,
   635 |           // If `variables` is not given, reset back to empty variables by
   636 |           // ensuring the key exists in options
   637 |           variables: executeOptions?.variables,
   638 |           context: executeOptions?.context ?? {},
   639 |         });
   640 | 
   641 |         // If a query is already in-flight and execute is called again with
   642 |         // different variables, useLazyQuery doesn't emit a new value until the
   643 |         // network request finishes because ObservableQuery doesn't emit a new
   644 |         // value when it is deep equal to the previous one. ObservableQuery
   645 |         // doesn't track variables as part of the result.
   646 |         //
   647 |         // Below forces the hook to rerender with the new variables immediately
   648 |         // instead of waiting for ObservableQuery to emit the network result.
   649 |         //
   650 |         // See https://github.com/apollographql/apollo-client/issues/13459
   651 |         if (
   652 |           observable.options.notifyOnNetworkStatusChange &&
   653 |           resultRef.current === previousResult &&
   654 |           !equal(observable.variables, previousVariables)
   655 |         ) {
   656 |           // useSyncExternalStore compares the snapshot using Object.is so we
   ```

2. `src/react/hooks/useLazyQuery.ts:485` worsened, `export const useLazyQuery: useLazyQuery.Signature = function useLazyQuery<`, values `{"cc":2,"lines":211}`, ceiling cc 5, lines 108, base site `src/react/hooks/useLazyQuery.ts:485` with `{"cc":2,"lines":180}`

   ```text
   482 |   "subscribeToMore",
   483 | ] as const;
   484 | 
   485 | export const useLazyQuery: useLazyQuery.Signature = function useLazyQuery<
   486 |   TData = unknown,
   487 |   TVariables extends OperationVariables = OperationVariables,
   488 |   TStates extends DataState<TData>["dataState"] = DataState<TData>["dataState"],
   489 | >(
   490 |   query: DocumentNode | TypedDocumentNode<TData, TVariables>,
   491 |   options?: useLazyQuery.Options<NoInfer<TData>, NoInfer<TVariables>>
   492 | ): useLazyQuery.ResultTuple<TData, TVariables, TStates> {
   493 |   const client = useApolloClient(options?.client);
   494 |   const previousDataRef = React.useRef<TData>(undefined);
   495 |   const resultRef = React.useRef<ObservableQuery.Result<TData>>(undefined);
   496 |   const forceUpdateRef = React.useRef<() => void>(() => {});
   497 |   const stableOptions = useDeepMemo(() => options, [options]);
   498 |   const calledDuringRender = useRenderGuard();
   499 | 
   500 |   function createObservable() {
   501 |     return client.watchQuery({
   502 |       ...options,
   503 |       query,
   504 |       initialFetchPolicy: options?.fetchPolicy,
   505 |       fetchPolicy: "standby",
   506 |       [variablesUnknownSymbol]: true,
   507 |     } as ApolloClient.WatchQueryOptions<TData, TVariables>);
   508 |   }
   509 | 
   510 |   const [currentClient, setCurrentClient] = React.useState(client);
   511 |   const [observable, setObservable] = React.useState(createObservable);
   512 | 
   513 |   if (currentClient !== client) {
   514 |     setCurrentClient(client);
   515 |     setObservable(createObservable());
   516 |   }
   517 | 
   518 |   // TODO: Revisit after we have RxJS in place. We should be able to use
   519 |   // observable.getCurrentResult() (or equivalent) to get these values which
   520 |   // will hopefully alleviate the need for us to use refs to track these values.
   521 |   const updateResult = React.useCallback(
   522 |     (result: ObservableQuery.Result<TData>, forceUpdate: () => void) => {
   523 |       const previousData = resultRef.current?.data;
   524 | 
   525 |       if (previousData && !equal(previousData, result.data)) {
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R050

- Repository: `apollographql/apollo-client` (TypeScript), change 5 of 10
- Commit: `d4f877012044`, judged against its first parent `d3c455ffc0c0`, exit 1
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived CHANGELOG.md: `71250`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `650`, the word count at the derivation commit, rounded up to the next 50
- Derived COLLABORATORS.md: `350`, the word count at the derivation commit, rounded up to the next 50
- Derived CONTRIBUTING.md: `2200`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `700`, the word count at the derivation commit, rounded up to the next 50
- Derived ROADMAP.md: `250`, the word count at the derivation commit, rounded up to the next 50
- Derived VERSIONING_POLICY.md: `850`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> Version Packages (#13461)
>
> This PR was opened by the [Changesets
> release](https://github.com/changesets/action) GitHub action. When
> you're ready to do a release, you can merge this and the packages will
> be published to npm automatically. If you're not ready to do a release
> yet, that's fine, whenever you add more changesets to main, this PR will
> be updated.
>
>
> # Releases
> ## @apollo/client-graphql-codegen@2.2.0
>
> ### Minor Changes
>
> - [#13310](https://github.com/apollographql/apollo-client/pull/13310)
> [`8ab63fc`](https://github.com/apollographql/apollo-client/commit/8ab63fc4bbf9f2c5b5f225ba2c54c2a255f0632e)
> Thanks [@jerelmiller](https://github.com/jerelmiller)! - Introduce a new
> GraphQL Codegen plugin to generate the input object configuration needed
> to configure custom scalars for each field.
>
>     ```ts
>     // codegen.ts
> import type { CustomScalarsPluginConfig } from
> "@apollo/client-graphql-codegen/custom-scalars";
>
>     const config: CodegenConfig = {
>       // ...
>       generates: {
>         "./path/to/custom-scalars.ts": {
> (854 more lines)

### Files the change touched

```text
 .changeset/afraid-starfishes-smash.md       | 101 --------------------------------------------------------------------
 .changeset/angry-baboons-decide.md          |  97 -----------------------------------------------------------------
 .changeset/beige-colts-flow.md              |   5 ----
 .changeset/big-scissors-hope.md             |  32 ----------------------
 .changeset/chilly-actors-complain.md        |   5 ----
 .changeset/cold-comics-add.md               |  18 ------------
 .changeset/cyan-camels-think.md             |   5 ----
 .changeset/dirty-donuts-punch.md            |   5 ----
 .changeset/eighty-files-sort.md             |  36 ------------------------
 .changeset/empty-pears-tell.md              |   5 ----
 .changeset/fast-geckos-help.md              |  34 -----------------------
 .changeset/fifty-books-return.md            |  32 ----------------------
 .changeset/forty-trainers-switch.md         |   5 ----
 .changeset/fuzzy-hairs-tie.md               |  34 -----------------------
 .changeset/gorgeous-tools-dream.md          |   5 ----
 .changeset/honest-lobsters-run.md           |   7 -----
 .changeset/hungry-onions-sleep.md           |   6 ----
 .changeset/incremental-codegen-complete.md  |   5 ----
 .changeset/khaki-singers-rush.md            |   7 -----
 .changeset/lazy-diff-diagnostics.md         |   7 -----
 .changeset/loud-bulldogs-pay.md             |   5 ----
 .changeset/nasty-keys-brush.md              |   5 ----
 .changeset/nice-donuts-subscribe.md         |  16 -----------
 .changeset/overridable-from-option-value.md |  36 ------------------------
 .changeset/serious-bugs-move.md             |   7 -----
 .changeset/serious-pumas-knock.md           |   5 ----
 .changeset/short-fishes-enjoy.md            |   5 ----
 .changeset/short-kids-retire.md             |   5 ----
 .changeset/silent-berries-help.md           |  51 ----------------------------------
 .changeset/smooth-flies-refuse.md           |   5 ----
 ...
 44 files changed, 646 insertions(+), 711 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":71250,"words":74350}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R051

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: doc-size, FAIL
- Decision group: document CHANGELOG.md
- Derived CHANGELOG.md: `68100`, the word count at the derivation commit, rounded up to the next 50
- Derived CLAUDE.md: `650`, the word count at the derivation commit, rounded up to the next 50
- Derived COLLABORATORS.md: `350`, the word count at the derivation commit, rounded up to the next 50
- Derived CONTRIBUTING.md: `2200`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `700`, the word count at the derivation commit, rounded up to the next 50
- Derived ROADMAP.md: `250`, the word count at the derivation commit, rounded up to the next 50
- Derived VERSIONING_POLICY.md: `850`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `CHANGELOG.md` new, values `{"ceiling":68100,"words":71246}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R052

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `integration-tests/type-tests/cacheOverride/classicSignature/index.ts:36` new, `// @ts-expect-error cache isn't TestCache`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched

   ```text
   33 | test("ApolloClient constructor", () => {
   34 |   {
   35 |     const client = new ApolloClient({
   36 |       // @ts-expect-error cache isn't TestCache
   37 |       cache: new InMemoryCache(),
   38 |       link: ApolloLink.empty(),
   39 |     });
   40 | 
   ```

2. `integration-tests/type-tests/cacheOverride/classicSignature/index.ts:74` new, `// @ts-expect-error wrong TCache subtype`, values `{"count":3,"escape":"ts-ignore"}`, nothing at the base matched

   ```text
   71 |     },
   72 |   });
   73 | 
   74 |   // @ts-expect-error wrong TCache subtype
   75 |   client.mutate<Data, Variables, ApolloCache>({
   76 |     mutation,
   77 |     update: (cache) => {
   78 |       expectTypeOf(cache).toEqualTypeOf<TestCache>();
   ```

3. `integration-tests/type-tests/cacheOverride/classicSignature/index.ts:164` new, `expectTypeOf(options!.update!).toEqualTypeOf<`, values `{"count":20,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   161 |         expectTypeOf(cache).toEqualTypeOf<TestCache>();
   162 |       },
   163 |       onCompleted: (_, options) => {
   164 |         expectTypeOf(options!.update!).toEqualTypeOf<
   165 |           MutationUpdaterFunction<Data, Variables, TestCache>
   166 |         >;
   167 |       },
   168 |       onError: (_, options) => {
   ```

4. `integration-tests/type-tests/cacheOverride/defaults/index.ts:143` new, `expectTypeOf(options!.update!).toEqualTypeOf<`, values `{"count":16,"escape":"non-null assertion"}`, nothing at the base matched
5. `integration-tests/type-tests/cacheOverride/invalid/index.ts:36` new, `// @ts-expect-error The cache type declared in TypeOverrides does not extend 'ApolloCache' and cannot be used with Apollo Client. See https://www.apollographql.`, values `{"count":9,"escape":"ts-ignore"}`, nothing at the base matched
6. `integration-tests/type-tests/cacheOverride/invalid/index.ts:140` new, `// @ts-expect-error property 'batch' doesn't exist in 'The cache type declared in TypeOverrides does not extend 'ApolloCache' and cannot be used with Apollo Cli`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
7. `integration-tests/type-tests/cacheOverride/invalid/index.ts:142` new, `// @ts-expect-error inferred any`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
8. `integration-tests/type-tests/cacheOverride/invalid/index.ts:144` new, `expectTypeOf(cache).toEqualTypeOf<any>();`, values `{"count":1,"escape":"any"}`, nothing at the base matched
9. `integration-tests/type-tests/cacheOverride/invalid/index.ts:147` new, `expectTypeOf(this).toEqualTypeOf<any>();`, values `{"count":1,"escape":"any"}`, nothing at the base matched
10. `integration-tests/type-tests/cacheOverride/invalid/index.ts:171` new, `expectTypeOf(options!.update!).toEqualTypeOf<`, values `{"count":12,"escape":"non-null assertion"}`, nothing at the base matched
11. `integration-tests/type-tests/cacheOverride/modernSignature/index.ts:36` new, `// @ts-expect-error cache isn't TestCache`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
12. `integration-tests/type-tests/cacheOverride/modernSignature/index.ts:80` new, `// @ts-expect-error wrong TCache subtype`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
13. `integration-tests/type-tests/cacheOverride/modernSignature/index.ts:134` new, `expectTypeOf(options!.update!).toEqualTypeOf<`, values `{"count":4,"escape":"non-null assertion"}`, nothing at the base matched
14. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:9` new, `interface Scalars extends Record<string, { serialized: any; parsed: any }> {`, values `{"count":2,"escape":"any"}`, nothing at the base matched
15. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:16` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
16. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:22` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
17. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:63` new, `expectTypeOf(value).toEqualTypeOf<any>();`, values `{"count":2,"escape":"any"}`, nothing at the base matched
18. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:79` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
19. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:93` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
20. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:107` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
21. `integration-tests/type-tests/customScalars/all-any/differentTypes/index.ts:128` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
22. `integration-tests/type-tests/customScalars/all-any/empty/index.ts:9` new, `extends Record<string, { serialized: any; parsed: any }> {}`, values `{"count":2,"escape":"any"}`, nothing at the base matched
23. `integration-tests/type-tests/customScalars/all-any/empty/index.ts:41` new, `expectTypeOf(value).toEqualTypeOf<any>();`, values `{"count":5,"escape":"any"}`, nothing at the base matched
24. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:8` new, `interface Scalars extends Record<string, { serialized: any; parsed: any }> {`, values `{"count":2,"escape":"any"}`, nothing at the base matched
25. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:85` new, `expectTypeOf(value).toEqualTypeOf<any>();`, values `{"count":3,"escape":"any"}`, nothing at the base matched
26. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:101` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
27. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:115` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
28. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:129` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
29. `integration-tests/type-tests/customScalars/all-any/matchingTypes/index.ts:150` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
30. `integration-tests/type-tests/customScalars/all-any/mixed/index.ts:8` new, `interface Scalars extends Record<string, { serialized: any; parsed: any }> {`, values `{"count":2,"escape":"any"}`, nothing at the base matched
31. `integration-tests/type-tests/customScalars/all-any/mixed/index.ts:16` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
32. `integration-tests/type-tests/customScalars/all-any/mixed/index.ts:19` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
33. `integration-tests/type-tests/customScalars/all-any/mixed/index.ts:97` new, `expectTypeOf(value).toEqualTypeOf<any>();`, values `{"count":2,"escape":"any"}`, nothing at the base matched
34. `integration-tests/type-tests/customScalars/all-structured/differentTypes/index.ts:18` new, `// @ts-expect-error 'scalars' is required`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
35. `integration-tests/type-tests/customScalars/all-structured/differentTypes/index.ts:22` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
36. `integration-tests/type-tests/customScalars/all-structured/differentTypes/index.ts:26` new, `// @ts-expect-error 'DateTime' is not assignable to index signature`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
37. `integration-tests/type-tests/customScalars/all-structured/empty/index.ts:63` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
38. `integration-tests/type-tests/customScalars/all-structured/empty/index.ts:73` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
39. `integration-tests/type-tests/customScalars/all-structured/empty/index.ts:83` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
40. `integration-tests/type-tests/customScalars/all-structured/empty/index.ts:98` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
41. `integration-tests/type-tests/customScalars/all-structured/matchingTypes/index.ts:44` new, `// @ts-expect-error JSONObject doesn't match serialized/parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
42. `integration-tests/type-tests/customScalars/all-structured/matchingTypes/index.ts:81` new, `// @ts-expect-error cannot return undefined`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
43. `integration-tests/type-tests/customScalars/all-structured/matchingTypes/index.ts:91` new, `// @ts-expect-error 'JSONObject''s 'unknown' types are not assignable to the string index.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
44. `integration-tests/type-tests/customScalars/all-structured/mixed/index.ts:20` new, `// @ts-expect-error 'scalars' is required`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
45. `integration-tests/type-tests/customScalars/all-structured/mixed/index.ts:24` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
46. `integration-tests/type-tests/customScalars/all-structured/mixed/index.ts:28` new, `// @ts-expect-error 'DateTime' is not assignable to index signature`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
47. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:17` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
48. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:23` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
49. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:80` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
50. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:94` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
51. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:108` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
52. `integration-tests/type-tests/customScalars/all-unknown/differentTypes/index.ts:129` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
53. `integration-tests/type-tests/customScalars/all-unknown/empty/index.ts:67` new, `// @ts-expect-error value is unknown`, values `{"count":4,"escape":"ts-ignore"}`, nothing at the base matched
54. `integration-tests/type-tests/customScalars/all-unknown/matchingTypes/index.ts:102` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
55. `integration-tests/type-tests/customScalars/all-unknown/matchingTypes/index.ts:107` new, `// @ts-expect-error value is unknown`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
56. `integration-tests/type-tests/customScalars/all-unknown/matchingTypes/index.ts:117` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
57. `integration-tests/type-tests/customScalars/all-unknown/matchingTypes/index.ts:131` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
58. `integration-tests/type-tests/customScalars/all-unknown/matchingTypes/index.ts:152` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
59. `integration-tests/type-tests/customScalars/all-unknown/mixed/index.ts:17` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
60. `integration-tests/type-tests/customScalars/all-unknown/mixed/index.ts:20` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
61. `integration-tests/type-tests/customScalars/differentTypes/index.ts:17` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
62. `integration-tests/type-tests/customScalars/differentTypes/index.ts:23` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
63. `integration-tests/type-tests/customScalars/differentTypes/index.ts:41` new, `// @ts-expect-error not a declared scalar`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
64. `integration-tests/type-tests/customScalars/differentTypes/index.ts:100` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
65. `integration-tests/type-tests/customScalars/differentTypes/index.ts:110` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
66. `integration-tests/type-tests/customScalars/differentTypes/index.ts:120` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
67. `integration-tests/type-tests/customScalars/differentTypes/index.ts:135` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
68. `integration-tests/type-tests/customScalars/differentTypes/index.ts:226` new, `// @ts-expect-error scalar not registered`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
69. `integration-tests/type-tests/customScalars/differentTypes/index.ts:234` new, `// @ts-expect-error non-null markers are not part of the scalar type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
70. `integration-tests/type-tests/customScalars/empty/index.ts:11` new, `// @ts-expect-error: Scalar types must be declared in ApolloCache.Scalars before usage. See https://www.apollographql.com/docs/react/data/typescript#declaring-s`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
71. `integration-tests/type-tests/customScalars/empty/index.ts:20` new, `// @ts-expect-error no scalars are declared`, values `{"count":3,"escape":"ts-ignore"}`, nothing at the base matched
72. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:52` new, `// @ts-expect-error not a declared scalar`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
73. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:92` new, `// @ts-expect-error wrong serialized type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
74. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:102` new, `// @ts-expect-error cannot return undefined`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
75. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:112` new, `// @ts-expect-error missing return`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
76. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:127` new, `// @ts-expect-error wrong parsed type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
77. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:207` new, `// @ts-expect-error scalar not registered`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
78. `integration-tests/type-tests/customScalars/matchingTypes/index.ts:215` new, `// @ts-expect-error non-null markers are not part of the scalar type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
79. `integration-tests/type-tests/customScalars/mixed/index.ts:16` new, `// @ts-expect-error 'scalars' is required.`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
80. `integration-tests/type-tests/customScalars/mixed/index.ts:19` new, `// @ts-expect-error 'DateTime' is missing from 'scalars'.`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
81. `integration-tests/type-tests/customScalars/mixed/index.ts:64` new, `// @ts-expect-error not a declared scalar`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
82. `integration-tests/type-tests/customScalars/mixed/index.ts:149` new, `// @ts-expect-error scalar not registered`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
83. `integration-tests/type-tests/customScalars/mixed/index.ts:157` new, `// @ts-expect-error non-null markers are not part of the scalar type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
84. `integration-tests/type-tests/fromOptionValue/override/index.ts:46` new, `// @ts-expect-error __typename is required`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
85. `integration-tests/type-tests/fromOptionValue/override/index.ts:48` new, `// @ts-expect-error __typename must match the fragment type`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
86. `integration-tests/type-tests/fromOptionValue/override/index.ts:50` new, `// @ts-expect-error id may not be null`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
87. `integration-tests/type-tests/fromOptionValue/override/index.ts:52` new, `// @ts-expect-error id may not be undefined`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
88. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1315` new, `const greeting1 = (diff1.result as any).greeting;`, values `{"count":2,"escape":"any"}`, nothing at the base matched
89. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1316` new, `const greeting3 = (diff3.result as any).greeting;`, values `{"count":2,"escape":"any"}`, nothing at the base matched
90. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1350` new, `const greeting4 = (diff4.result as any).greeting;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
91. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1379` new, `const greeting5 = (diff5.result as any).greeting;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
92. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1411` new, `const greeting6 = (diff6.result as any).greeting;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
93. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1438` new, `const greeting7 = (diff7.result as any).greeting;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
94. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1518` new, `const partialFriends = (partialDiff.result as any).friends;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
95. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1549` new, `const strippedFriends = (strippedDiff.result as any).friends;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
96. `src/cache/inmemory/__tests__/readFromStore.ts:2278` new, `// @ts-ignore`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
97. `src/cache/inmemory/__tests__/scalars.ts:1578` new, `const existingStartTime = rawCacheData(cache)["Event:1"]!.startTime;`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
98. `src/cache/inmemory/__tests__/scalars.ts:1587` new, `expect(rawCacheData(cache)["Event:1"]!.startTime).toBe(existingStartTime);`, values `{"count":2,"escape":"non-null assertion"}`, nothing at the base matched
99. `src/cache/inmemory/__tests__/scalars.ts:1645` new, `const modifiedEvent = rawCacheData(cache).ROOT_QUERY!.event;`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
100. `src/cache/inmemory/__tests__/scalars.ts:1648` new, `expect((modifiedEvent as any).startTime).toBe(replacementEvent.startTime);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
101. `src/cache/inmemory/__tests__/scalars.ts:2482` new, `expect(restoredEvent!.startTime).toBe(event.startTime);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
102. `src/cache/inmemory/__tests__/scalars.ts:5091` new, `// @ts-expect-error TODO: Need to figure out types`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
103. `src/cache/inmemory/__tests__/scalars.ts:5102` new, `expect(result!.event.startTime).toBe(initialValue!.event.startTime);`, values `{"count":2,"escape":"non-null assertion"}`, nothing at the base matched
104. `src/cache/inmemory/__tests__/scalars.ts:5111` new, `expect(result!.startTime).toBe(initialValue!.event.startTime);`, values `{"count":2,"escape":"non-null assertion"}`, nothing at the base matched
105. `src/cache/inmemory/inMemoryCache.ts:230` new, `return this.config.scalars?.[key as string] as any;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
106. `src/cache/inmemory/readFromStore.ts:129` new, `data: any;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
107. `src/cache/inmemory/readFromStore.ts:137` new, `array: any[];`, values `{"count":1,"escape":"any"}`, nothing at the base matched
108. `src/cache/inmemory/readFromStore.ts:816` new, `}: PruneSelectionSetOptions): PruneResult<any> {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
109. `src/cache/inmemory/readFromStore.ts:935` new, `}: PruneArrayOptions): PruneResult<any> {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
110. `src/cache/inmemory/readFromStore.ts:940` new, `let pruned: any[] = [];`, values `{"count":1,"escape":"any"}`, nothing at the base matched
111. `src/cache/inmemory/readFromStore.ts:951` new, `let prunedResult: PruneResult<any> = {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
112. `src/cache/inmemory/readFromStore.ts:1091` new, `function shouldPrune({ dataState }: ExecResult<any>, context: ReadContext) {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
113. `src/core/ObservableQuery.ts:105` new, `// @ts-expect-error this is just too generic to be typed correctly`, values `{"count":2,"escape":"ts-ignore"}`, nothing at the base matched
114. `src/core/ObservableQuery.ts:2163` new, `dataStateErrorCache.get(notification.error) ?? ("complete" as any);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
115. `src/core/ObservableQuery.ts:2317` new, `function warnOnFeud(query: DocumentNode, diff: Cache.DiffResult<any>) {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
116. `src/core/QueryInfo.ts:320` new, `streamInfo.lookupArray(item.path as any[]).state.truncate = true;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
117. `src/core/QueryInfo.ts:626` new, `result.data = diff.result as any;`, values `{"count":1,"escape":"any"}`, nothing at the base matched
118. `src/core/__tests__/client.mutate/customScalars.test.ts:505` new, `lastCreatedEvent: result.data!.createEvent,`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
119. `src/core/__tests__/client.subscribe/customScalars.test.ts:474` new, `expect(second.data!.eventCreated).toBe(first.data!.eventCreated);`, values `{"count":2,"escape":"non-null assertion"}`, nothing at the base matched
120. `src/core/__tests__/client.watchQuery/customScalars.test.ts:497` new, `event: (subscriptionData.data as any).eventUpdated,`, values `{"count":1,"escape":"any"}`, nothing at the base matched
121. `src/core/__tests__/client.watchQuery/customScalars.test.ts:1842` new, `] as any,`, values `{"count":2,"escape":"any"}`, nothing at the base matched
122. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:939` new, `} as any);`, values `{"count":3,"escape":"any"}`, nothing at the base matched
123. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4022` new, `{ items: [{ __typename: "Friend", id: "2" }] as any, id: "2" },`, values `{"count":1,"escape":"any"}`, nothing at the base matched
124. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4217` new, `items: [{ __typename: "Friend", id: "2", name: "Han" }] as any,`, values `{"count":1,"escape":"any"}`, nothing at the base matched
125. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:5275` new, `{ id: "1", items: [{ __typename: "Friend", name: "Han" }] as any },`, values `{"count":1,"escape":"any"}`, nothing at the base matched
126. `src/core/dataStateErrorCache.ts:14` new, `DataState<any>["dataState"]`, values `{"count":1,"escape":"any"}`, nothing at the base matched
127. `src/declarations.d.ts:14` new, `extends Record<string, { serialized: any; parsed: any }> {}`, values `{"count":2,"escape":"any"}`, nothing at the base matched
128. `src/incremental/__benches__/types.bench.ts:19` new, `// @ts-ignore`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
129. `src/incremental/handlers/graphql17Alpha9.ts:146` new, `const entry = this._streamInfo.lookupArray(pending.path as any[]);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
130. `src/incremental/handlers/graphql17Alpha9.ts:247` new, `const details = this._streamInfo.peekArray(path as any[]);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
131. `src/react/hooks/__tests__/useBackgroundQuery/testUtils.tsx:53` new, `const [queryRef, { refetch }] = renderHook(props as any);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
132. `src/react/hooks/__tests__/useLoadableQuery/customScalars.test.tsx:238` new, `previousData = snapshot.result!.data;`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
133. `src/react/hooks/__tests__/useLoadableQuery/customScalars.test.tsx:278` new, `expect(snapshot.result!.data).toBe(previousData);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
134. `src/react/hooks/__tests__/useLoadableQuery/testUtils.tsx:59` new, `const [loadQuery, queryRef, { refetch }] = renderHook(props as any);`, values `{"count":1,"escape":"any"}`, nothing at the base matched
135. `src/react/hooks/__tests__/useLoadableQuery/testUtils.tsx:91` new, `initialSnapshot: { loadQuery: null as any, refetch: null as any },`, values `{"count":2,"escape":"any"}`, nothing at the base matched
136. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:64` new, `result: null as useReadQuery.Result<any> | null,`, values `{"count":4,"escape":"any"}`, nothing at the base matched
137. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:391` new, `event: (subscriptionData.data as any).eventUpdated,`, values `{"count":1,"escape":"any"}`, nothing at the base matched
138. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:584` new, `previousData = snapshot.result!.data;`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
139. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:625` new, `expect(snapshot.result!.data).toBe(previousData);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
140. `src/react/hooks/__tests__/useSubscription.test.tsx:3935` new, `// @ts-expect-error skipToken does not replace the required options object`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
141. `src/react/hooks/__tests__/useSubscription.test.tsx:3961` new, `// @ts-expect-error missing required variable`, values `{"count":1,"escape":"ts-ignore"}`, nothing at the base matched
142. `src/react/hooks/__tests__/useSubscription/customScalars.test.tsx:234` new, `expect(getCurrentSnapshot().data!.eventCreated).toBe(`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
143. `src/react/hooks/__tests__/useSubscription/customScalars.test.tsx:235` new, `previousData!.eventCreated`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
144. `src/react/query-preloader/__tests__/createQueryPreloader/customScalars.test.tsx:189` new, `previousData = snapshot.result!.data;`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
145. `src/react/query-preloader/__tests__/createQueryPreloader/customScalars.test.tsx:206` new, `expect(renderStream.getCurrentRender().snapshot.result!.data).toBe(`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched
146. `src/utilities/internal/coerceScalarFieldsToParsed.ts:95` new, `data: any,`, values `{"count":1,"escape":"any"}`, nothing at the base matched
147. `src/utilities/internal/coerceScalarFieldsToParsed.ts:97` new, `): any {`, values `{"count":1,"escape":"any"}`, nothing at the base matched
148. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:211` worsened, `] as any,`, values `{"count":4,"escape":"any"}`, base site `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:206` with `{"count":1,"escape":"any"}`
149. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:1001` worsened, `const merged: any[] = [];`, values `{"count":2,"escape":"any"}`, base site `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:901` with `{"count":1,"escape":"any"}`
150. `src/react/hooks/__tests__/useSubscription.test.tsx:3815` worsened, `// @ts-expect-error`, values `{"count":7,"escape":"ts-ignore"}`, base site `src/react/hooks/__tests__/useSubscription.test.tsx:3665` with `{"count":5,"escape":"ts-ignore"}`

### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R053

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: stubs, FAIL
- Decision group: stubs

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: where the code stands in for work nobody did.

1. `codegen/custom-scalars/__tests__/plugin.test.ts:1530` new, `// TODO: Determine whether we want to configure custom scalars on the interface`, values `{"count":2,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched

   ```text
   1527 |   });
   1528 | });
   1529 | 
   1530 | // TODO: Determine whether we want to configure custom scalars on the interface
   1531 | // types or concrete types. Configuring the interface types might reduce the
   1532 | // size of the object, but is more complex in order to avoid writing to the
   1533 | // concrete types.
   1534 | test("outputs type policies for concrete types when selecting custom scalar fields from an interface", async () => {
   ```

2. `src/cache/inmemory/__tests__/scalars.ts:5091` new, `// @ts-expect-error TODO: Need to figure out types`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched

   ```text
   5088 |       event: {
   5089 |         __typename: "Event",
   5090 |         id: "1",
   5091 |         // @ts-expect-error TODO: Need to figure out types
   5092 |         startTime: "2026-01-01T00:00:00.000Z",
   5093 |       },
   5094 |     },
   5095 |   });
   ```

3. `src/core/__tests__/client.mutate/customScalars.test.ts:799` new, `// TODO: Determine if this is correct`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched

   ```text
   796 |         createEvent: {
   797 |           __typename: "Event",
   798 |           id: "1",
   799 |           // TODO: Determine if this is correct
   800 |           startDate: new Date(2026, 0, 1),
   801 |           endDate: null,
   802 |         },
   803 |       },
   ```

4. `src/core/__tests__/client.query/customScalars.test.ts:623` new, `// TODO: Determine if this is correct`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched
5. `src/core/__tests__/client.subscribe/customScalars.test.ts:608` new, `// TODO: Determine if this is correct`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched
6. `src/core/__tests__/client.watchQuery/customScalars.test.ts:1172` new, `// TODO: Determine if this is correct`, values `{"count":1,"remedy":"do the work the comment names, or record it in the tracker and delete the comment","stub":"comment marker"}`, nothing at the base matched

### Remedy klin printed

> Do what the marker stands in for. A placeholder an agent left behind is not work, and accepting one is a decision for a person, in the config, in a reviewed commit.

## R054

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: complexity, ERR
- Decision group: cc at the floor; cc at the floor, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over; unparsed record
- Derived cc: `5`, the floor of 5, over 11,419 function(s) at 70e3a11; recorded scope: whole repository
- Derived lines: `107`, 95th percentile of 11,419 functions at 70e3a11, floor 25; recorded scope: whole repository

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 5 or body > 107 lines).

1. `codegen/custom-scalars/plugin.ts:53` new, `export const plugin: PluginFunction<CustomScalarsPluginConfig, string> = async (`, values `{"cc":23,"lines":121}`, ceiling cc 5, lines 107, nothing at the base matched

   ```text
   50 |   js: [".js", ".jsx"],
   51 | };
   52 | 
   53 | export const plugin: PluginFunction<CustomScalarsPluginConfig, string> = async (
   54 |   schema,
   55 |   documents,
   56 |   config,
   57 |   info
   58 | ) => {
   59 |   const { includeScalars, filterByDocuments = true } = config;
   60 |   const ext = extname(info?.outputFile ?? "").toLowerCase();
   61 | 
   62 |   if (includeScalars?.length === 0) {
   63 |     return buildOutput({ inputObjects: {}, typePolicies: {} }, ext);
   64 |   }
   65 | 
   66 |   const types = Object.values(schema.getTypeMap());
   67 |   const customScalars = new Set<string>();
   68 |   let inputObjects: InputObjectMap = new Map();
   69 | 
   70 |   for (const type of types) {
   71 |     if (isCustomScalar(type, config)) {
   72 |       customScalars.add(type.name);
   73 |     } else if (isInputObjectType(type)) {
   74 |       const fields: Record<string, InputFieldType> = {};
   75 | 
   76 |       for (const [fieldName, field] of Object.entries(type.getFields())) {
   77 |         const inputType = getNamedType(field.type);
   78 | 
   79 |         if (isCustomScalar(inputType, config) || isInputObjectType(inputType)) {
   80 |           fields[fieldName] = {
   81 |             name: inputType.name,
   82 |             type: field.type.toString().replaceAll("!", ""),
   83 |           };
   84 |         }
   85 |       }
   86 | 
   87 |       if (Object.keys(fields).length > 0) {
   88 |         inputObjects.set(type.name, fields);
   89 |       }
   90 |     }
   91 |   }
   92 | 
   93 |   if (customScalars.size === 0) {
   ```

2. `codegen/custom-scalars/plugin.ts:249` new, `function getInputObjectsWithCustomScalars(`, values `{"cc":11,"lines":37}`, ceiling cc 5, lines 107, nothing at the base matched

   ```text
   246 | `.trim();
   247 | }
   248 | 
   249 | function getInputObjectsWithCustomScalars(
   250 |   inputObjects: InputObjectMap,
   251 |   customScalars: Set<string>
   252 | ) {
   253 |   const useful = new Set<string>();
   254 |   const dependents = new Map<string, string[]>();
   255 |   const queue: string[] = [];
   256 | 
   257 |   for (const [name, fields] of inputObjects) {
   258 |     for (const inputType of Object.values(fields)) {
   259 |       if (customScalars.has(inputType.name)) {
   260 |         if (!useful.has(name)) {
   261 |           useful.add(name);
   262 |           queue.push(name);
   263 |         }
   264 |       } else if (inputObjects.has(inputType.name)) {
   265 |         let deps = dependents.get(inputType.name);
   266 |         if (!deps) {
   267 |           dependents.set(inputType.name, (deps = []));
   268 |         }
   269 |         deps.push(name);
   270 |       }
   271 |     }
   272 |   }
   273 | 
   274 |   while (queue.length) {
   275 |     const name = queue.pop()!;
   276 |     for (const dependent of dependents.get(name) ?? []) {
   277 |       if (!useful.has(dependent)) {
   278 |         useful.add(dependent);
   279 |         queue.push(dependent);
   280 |       }
   281 |     }
   282 |   }
   283 | 
   284 |   return useful;
   285 | }
   286 | 
   ```

3. `codegen/custom-scalars/plugin.ts:304` new, `Field(node) {`, values `{"cc":7,"lines":21}`, ceiling cc 5, lines 107, nothing at the base matched

   ```text
   301 |         usedInputObjects.add(type.name);
   302 |       }
   303 |     },
   304 |     Field(node) {
   305 |       const parentType = typeInfo.getParentType();
   306 |       const fieldTypeName = getNamedType(typeInfo.getType())?.name;
   307 | 
   308 |       if (!parentType || !fieldTypeName || !customScalars.has(fieldTypeName)) {
   309 |         return;
   310 |       }
   311 | 
   312 |       const typenames =
   313 |         isInterfaceType(parentType) ?
   314 |           schema.getPossibleTypes(parentType).map((type) => type.name)
   315 |         : [parentType.name];
   316 | 
   317 |       for (const typename of typenames) {
   318 |         let fields = usedFields.get(typename);
   319 |         if (!fields) {
   320 |           usedFields.set(typename, (fields = new Set()));
   321 |         }
   322 |         fields.add(node.name.value);
   323 |       }
   324 |     },
   325 |   });
   ```

4. `codegen/custom-scalars/plugin.ts:336` new, `function getReachableInputObjects(`, values `{"cc":8,"lines":33}`, ceiling cc 5, lines 107, nothing at the base matched
5. `integration-tests/type-tests/cacheOverride/classicSignature/index.ts:157` new, `test("useMutation", () => {`, values `{"cc":1,"lines":192}`, ceiling cc 5, lines 107, nothing at the base matched
6. `integration-tests/type-tests/cacheOverride/defaults/index.ts:136` new, `test("useMutation", () => {`, values `{"cc":1,"lines":153}`, ceiling cc 5, lines 107, nothing at the base matched
7. `integration-tests/type-tests/cacheOverride/invalid/index.ts:164` new, `test("useMutation", () => {`, values `{"cc":1,"lines":113}`, ceiling cc 5, lines 107, nothing at the base matched
8. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:1072` new, `test("returns a referentially stable result across reads, rebuilding only the paths changed by a write, when partial @defer boundaries are stripped with returnP`, values `{"cc":1,"lines":375}`, ceiling cc 5, lines 107, nothing at the base matched
9. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:3501` new, `test("complete is only true when dataState is complete", () => {`, values `{"cc":1,"lines":137}`, ceiling cc 5, lines 107, nothing at the base matched
10. `src/cache/inmemory/__tests__/cache.diff/incremental.test.ts:7308` new, `test("strips nested partial @defer fields under the same list field contributed by sibling fragments with returnPartialData: false", () => {`, values `{"cc":2,"lines":122}`, ceiling cc 5, lines 107, nothing at the base matched
11. `src/cache/inmemory/__tests__/scalars.ts:1263` new, `test("stores parsed scalar values across a complex nested write", () => {`, values `{"cc":1,"lines":184}`, ceiling cc 5, lines 107, nothing at the base matched
12. `src/cache/inmemory/__tests__/scalars.ts:1994` new, `test("cache.extract() serializes all stored parsed scalar values", () => {`, values `{"cc":1,"lines":189}`, ceiling cc 5, lines 107, nothing at the base matched
13. `src/cache/inmemory/__tests__/scalars.ts:2287` new, `test("cache.restore() parses all serialized scalar values before storing them", () => {`, values `{"cc":1,"lines":169}`, ceiling cc 5, lines 107, nothing at the base matched
14. `src/cache/inmemory/__tests__/scalars.ts:3828` new, `test("parses scalar values across a complex nested query", () => {`, values `{"cc":1,"lines":232}`, ceiling cc 5, lines 107, nothing at the base matched
15. `src/cache/inmemory/__tests__/scalars.ts:4804` new, `test("deep merges scalar option with policies.addTypePolicies", () => {`, values `{"cc":1,"lines":187}`, ceiling cc 5, lines 107, nothing at the base matched
16. `src/cache/inmemory/entityStore.ts:475` new, `private coerceValue(`, values `{"cc":10,"lines":51}`, ceiling cc 5, lines 107, nothing at the base matched
17. `src/cache/inmemory/inMemoryCache.ts:265` new, `public serializeVariables<`, values `{"cc":7,"lines":28}`, ceiling cc 5, lines 107, nothing at the base matched
18. `src/cache/inmemory/inMemoryCache.ts:294` new, `private serializeVariablesValue(`, values `{"cc":9,"lines":53}`, ceiling cc 5, lines 107, nothing at the base matched
19. `src/cache/inmemory/policies.ts:516` new, `private updateTypePolicy(`, values `{"cc":5,"lines":128}`, ceiling cc 5, lines 107, nothing at the base matched
20. `src/cache/inmemory/readFromStore.ts:192` new, `constructor(config: StoreReaderConfig) {`, values `{"cc":5,"lines":113}`, ceiling cc 5, lines 107, nothing at the base matched
21. `src/cache/inmemory/readFromStore.ts:320` new, `public diffQueryAgainstStore<T>({`, values `{"cc":20,"lines":126}`, ceiling cc 5, lines 107, nothing at the base matched
22. `src/cache/inmemory/readFromStore.ts:810` new, `private prunePartialBoundariesImpl({`, values `{"cc":8,"lines":118}`, ceiling cc 5, lines 107, nothing at the base matched
23. `src/cache/inmemory/readFromStore.ts:837` new, `workSet.forEach((selection) => {`, values `{"cc":15,"lines":78}`, ceiling cc 5, lines 107, nothing at the base matched
24. `src/cache/inmemory/readFromStore.ts:929` new, `private prunePartialStreamArrayImpl({`, values `{"cc":9,"lines":73}`, ceiling cc 5, lines 107, nothing at the base matched
25. `src/cache/inmemory/readFromStore.ts:1091` new, `function shouldPrune({ dataState }: ExecResult<any>, context: ReadContext) {`, values `{"cc":6,"lines":22}`, ceiling cc 5, lines 107, nothing at the base matched
26. `src/core/QueryInfo.ts:303` new, `private getIncrementalInfo({ prune }: { prune: boolean }) {`, values `{"cc":9,"lines":24}`, ceiling cc 5, lines 107, nothing at the base matched
27. `src/core/__tests__/ApolloClient/general.test.ts:2879` new, `it("stops repeated refetches when queries feud over non-normalized data", async () => {`, values `{"cc":1,"lines":159}`, ceiling cc 5, lines 107, nothing at the base matched
28. `src/core/__tests__/ApolloClient/general.test.ts:3228` new, `it("fetches a clobbered value again after reading a complete result in between", async () => {`, values `{"cc":1,"lines":113}`, ceiling cc 5, lines 107, nothing at the base matched
29. `src/core/__tests__/ApolloClient/general.test.ts:3541` new, `it("applies read functions when feud-stopping skips refetches", async () => {`, values `{"cc":1,"lines":162}`, ceiling cc 5, lines 107, nothing at the base matched
30. `src/core/__tests__/ApolloClient/general.test.ts:3789` new, `it("delivers repeated cache updates to a cache-first query kept partial by a read function", async () => {`, values `{"cc":1,"lines":115}`, ceiling cc 5, lines 107, nothing at the base matched
31. `src/core/__tests__/client.mutate/customScalars.test.ts:529` new, `test("parses custom scalar fields in queries triggered by refetchQueries", async () => {`, values `{"cc":1,"lines":147}`, ceiling cc 5, lines 107, nothing at the base matched
32. `src/core/__tests__/client.watchQuery/customScalars.test.ts:1441` new, `test("parses custom scalar fields across '@defer' payloads when refetching (defer20220824)", async () => {`, values `{"cc":1,"lines":161}`, ceiling cc 5, lines 107, nothing at the base matched
33. `src/core/__tests__/client.watchQuery/customScalars.test.ts:1603` new, `test("parses custom scalar fields across '@defer' payloads when refetching (graphql17Alpha9)", async () => {`, values `{"cc":1,"lines":165}`, ceiling cc 5, lines 107, nothing at the base matched
34. `src/core/__tests__/client.watchQuery/customScalars.test.ts:1979` new, `test("parses custom scalar fields when feud-stopping skips refetches", async () => {`, values `{"cc":1,"lines":163}`, ceiling cc 5, lines 107, nothing at the base matched
35. `src/core/__tests__/client.watchQuery/customScalars.test.ts:2143` new, `test("parses custom scalar fields when feud-stopping skips refetches with overlapping fields", async () => {`, values `{"cc":1,"lines":238}`, ceiling cc 5, lines 107, nothing at the base matched
36. `src/core/__tests__/client.watchQuery/defer20220824.test.ts:352` new, `test("delivers cache updates written while a deferred response is still streaming", async () => {`, values `{"cc":1,"lines":112}`, ceiling cc 5, lines 107, nothing at the base matched
37. `src/core/__tests__/client.watchQuery/defer20220824.test.ts:465` new, `test("delivers cache updates written while a deferred response is still streaming with 'returnPartialData: true'", async () => {`, values `{"cc":1,"lines":116}`, ceiling cc 5, lines 107, nothing at the base matched
38. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:820` new, `test("does not surface incomplete cached defer-only fields under an overlapping parent with returnPartialData: false", async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
39. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:940` new, `test("does not surface incomplete cached fields inside a list item '@defer' boundary while sibling items are clean defer gaps with returnPartialData: false", as`, values `{"cc":1,"lines":147}`, ceiling cc 5, lines 107, nothing at the base matched
40. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1088` new, `test("does not surface incomplete cached fields inside a later sibling '@defer' boundary while an earlier sibling is still pending with returnPartialData: false`, values `{"cc":1,"lines":142}`, ceiling cc 5, lines 107, nothing at the base matched
41. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1231` new, `test("does not surface incomplete cached deep defer-only fields under an overlapped path with returnPartialData: false", async () => {`, values `{"cc":1,"lines":138}`, ceiling cc 5, lines 107, nothing at the base matched
42. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1370` new, `test("does not treat cached '__typename'-only data inside a '@defer' boundary as incomplete deferred fields with returnPartialData: false", async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
43. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1490` new, `test('returns partial non-deferred cached data with a "cache-first" fetch policy and returnPartialData', async () => {`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
44. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1655` new, `test('returns partial deferred cached data as "partial" while streaming with a "cache-first" fetch policy and returnPartialData', async () => {`, values `{"cc":1,"lines":117}`, ceiling cc 5, lines 107, nothing at the base matched
45. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1773` new, `test('reports partial cached data inside a defer boundary as "partial" when the boundary completes with errors with a "cache-first" fetch policy and returnParti`, values `{"cc":1,"lines":125}`, ceiling cc 5, lines 107, nothing at the base matched
46. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:1899` new, `test("reports partial data correctly when a mid-stream request is abandoned and the query is subscribed to again", async () => {`, values `{"cc":1,"lines":158}`, ceiling cc 5, lines 107, nothing at the base matched
47. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2150` new, `test('reports overlapping deferred and non-deferred fields as "streaming" when only the deferred-only fields are missing', async () => {`, values `{"cc":1,"lines":110}`, ceiling cc 5, lines 107, nothing at the base matched
48. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2261` new, `test('reports "streaming" when non-deferred fields for the same response key are split across sibling selection sets', async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
49. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2381` new, `test('reports "streaming" when non-deferred fields for the same response key come from a field and a fragment', async () => {`, values `{"cc":1,"lines":121}`, ceiling cc 5, lines 107, nothing at the base matched
50. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2503` new, `test('reports "streaming" when nested non-deferred fields are split across sibling selection sets', async () => {`, values `{"cc":1,"lines":153}`, ceiling cc 5, lines 107, nothing at the base matched
51. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2657` new, `test('reports "streaming" when one of multiple sibling '@defer' fragments has fully arrived and another is still pending', async () => {`, values `{"cc":1,"lines":121}`, ceiling cc 5, lines 107, nothing at the base matched
52. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:2967` new, `test('evaluates '@defer(if: $variable)' as deferred when the variable is true, reporting "streaming" while deferred fields are pending', async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
53. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3087` new, `test("evaluates '@defer(if: $variable)' for overlapping fields so disabled-defer selections stay non-deferred", async () => {`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
54. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3197` new, `test('reports "streaming" instead of "partial" when the only unfulfilled field of a deferred fragment is excluded by '@skip'', async () => {`, values `{"cc":1,"lines":112}`, ceiling cc 5, lines 107, nothing at the base matched
55. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3310` new, `test("reports the correct data state for '@defer' on a named fragment spread with partial cached data", async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
56. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3644` new, `test("treats nested fragments inside a '@defer' boundary when deciding if the fragment has started", async () => {`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
57. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3754` new, `test("treats a named fragment spread inside a '@defer' boundary when deciding if the fragment has started", async () => {`, values `{"cc":1,"lines":111}`, ceiling cc 5, lines 107, nothing at the base matched
58. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:3866` new, `test('reports "partial" when a nested defer-only field is present under an overlapping parent while another defer-only field is still missing', async () => {`, values `{"cc":1,"lines":138}`, ceiling cc 5, lines 107, nothing at the base matched
59. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4005` new, `test("merges non-deferred selections that contribute different subfields under the same response key", async () => {`, values `{"cc":1,"lines":119}`, ceiling cc 5, lines 107, nothing at the base matched
60. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4125` new, `test('reports overlapping fields from a fragment spread repeated at the same selection-set level as "streaming"', async () => {`, values `{"cc":1,"lines":115}`, ceiling cc 5, lines 107, nothing at the base matched
61. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4241` new, `test('reports overlapping fields from the same fragment spread used at different selection-set levels as "streaming"', async () => {`, values `{"cc":1,"lines":138}`, ceiling cc 5, lines 107, nothing at the base matched
62. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4486` new, `test('reports "streaming" when one list item\'s '@defer' has arrived and another item is still pending', async () => {`, values `{"cc":1,"lines":129}`, ceiling cc 5, lines 107, nothing at the base matched
63. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4616` new, `test('reports overlapping deferred and non-deferred list fields as "streaming" when only defer-only item fields are missing', async () => {`, values `{"cc":1,"lines":128}`, ceiling cc 5, lines 107, nothing at the base matched
64. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:4745` new, `test('reports "partial" when one list item has a started '@defer' that is still incomplete while other items are only clean defer gaps', async () => {`, values `{"cc":1,"lines":175}`, ceiling cc 5, lines 107, nothing at the base matched
65. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5021` new, `test('reports "partial" when a later '@defer' boundary is incomplete and an earlier sibling '@defer' is still fully pending', async () => {`, values `{"cc":1,"lines":152}`, ceiling cc 5, lines 107, nothing at the base matched
66. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5174` new, `test('reports "streaming" when two '@defer' fragments overlap and only the second fragment\'s exclusive fields are missing', async () => {`, values `{"cc":1,"lines":131}`, ceiling cc 5, lines 107, nothing at the base matched
67. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5306` new, `test('keeps a delivered '@defer' fragment\'s fields while a sibling fragment at the same path is still pending with a "network-only" fetch policy', async () => `, values `{"cc":1,"lines":120}`, ceiling cc 5, lines 107, nothing at the base matched
68. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5427` new, `test('reports "partial" when a deep defer-only field is present under an overlapped path while another deep defer-only field is still missing', async () => {`, values `{"cc":1,"lines":158}`, ceiling cc 5, lines 107, nothing at the base matched
69. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5586` new, `test('reports overlapping fields gated by '@include' as "streaming" when the included non-deferred fields are present and only defer-only fields are missing', a`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
70. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:5899` new, `test('reports "streaming" for nested '@defer' when the outer fragment has arrived and the inner fragment is still pending', async () => {`, values `{"cc":1,"lines":124}`, ceiling cc 5, lines 107, nothing at the base matched
71. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:6024` new, `test("does not treat '__typename'-only presence under a '@defer' as the fragment having started", async () => {`, values `{"cc":1,"lines":128}`, ceiling cc 5, lines 107, nothing at the base matched
72. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:6153` new, `test('reports residual deferred cached data as "complete" while streaming with a "cache-first" fetch policy and returnPartialData', async () => {`, values `{"cc":1,"lines":112}`, ceiling cc 5, lines 107, nothing at the base matched
73. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:6371` new, `test('keeps complete cached deferred data when a defer boundary completes with errors with a "cache-and-network" fetch policy', async () => {`, values `{"cc":1,"lines":123}`, ceiling cc 5, lines 107, nothing at the base matched
74. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:6908` new, `test('reports residual deferred cached data as "complete" while streaming with a "cache-and-network" fetch policy and returnPartialData', async () => {`, values `{"cc":1,"lines":112}`, ceiling cc 5, lines 107, nothing at the base matched
75. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:7021` new, `test('prunes a list item\'s cached defer boundary while a sibling item has already streamed in with a "cache-first" fetch policy', async () => {`, values `{"cc":1,"lines":138}`, ceiling cc 5, lines 107, nothing at the base matched
76. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:7254` new, `test('keeps a non-deferred fragment\'s fields at a path with a pending defer boundary with a "network-only" fetch policy', async () => {`, values `{"cc":1,"lines":108}`, ceiling cc 5, lines 107, nothing at the base matched
77. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:7567` new, `test('does not return a partial cached defer boundary while streaming with a "network-only" fetch policy', async () => {`, values `{"cc":1,"lines":108}`, ceiling cc 5, lines 107, nothing at the base matched
78. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:7772` new, `test('does not return cached defer boundaries for list items while streaming with a "network-only" fetch policy', async () => {`, values `{"cc":1,"lines":114}`, ceiling cc 5, lines 107, nothing at the base matched
79. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:7887` new, `test('prunes a list item\'s cached defer boundary while a sibling item has already streamed in with a "network-only" fetch policy', async () => {`, values `{"cc":1,"lines":134}`, ceiling cc 5, lines 107, nothing at the base matched
80. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:8022` new, `test('prunes each list item\'s sibling '@defer' boundaries independently by label with a "network-only" fetch policy', async () => {`, values `{"cc":1,"lines":187}`, ceiling cc 5, lines 107, nothing at the base matched
81. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:8210` new, `test("prunes the right list item's defer boundary when a spread fragment's nested '@defer' shares a label across sibling boundaries", async () => {`, values `{"cc":1,"lines":264}`, ceiling cc 5, lines 107, nothing at the base matched
82. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:8475` new, `test("prunes correctly when a spread fragment's nested '@defer' sits at the same path as the sibling boundaries that spread it", async () => {`, values `{"cc":1,"lines":232}`, ceiling cc 5, lines 107, nothing at the base matched
83. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:8708` new, `test("prunes correctly when the same spread fragment's nested '@defer' resolves to two different paths", async () => {`, values `{"cc":1,"lines":240}`, ceiling cc 5, lines 107, nothing at the base matched
84. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:8949` new, `test('does not leak complete or partial cached defer boundaries while streaming with a "network-only" fetch policy and returnPartialData: true', async () => {`, values `{"cc":1,"lines":129}`, ceiling cc 5, lines 107, nothing at the base matched
85. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:9079` new, `test('keeps residual deferred cache data as "complete" while streaming after a refetch', async () => {`, values `{"cc":1,"lines":177}`, ceiling cc 5, lines 107, nothing at the base matched
86. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:9257` new, `test('keeps residual deferred cache data as "complete" while streaming after a refetch that previously had incremental errors', async () => {`, values `{"cc":1,"lines":230}`, ceiling cc 5, lines 107, nothing at the base matched
87. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:9780` new, `test("applies field read functions to partial non-deferred cached data before and after deferred network results", async () => {`, values `{"cc":1,"lines":123}`, ceiling cc 5, lines 107, nothing at the base matched
88. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:9904` new, `test("applies field read functions when partial cache data in a defer boundary is merged with streaming results", async () => {`, values `{"cc":1,"lines":141}`, ceiling cc 5, lines 107, nothing at the base matched
89. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10046` new, `test("applies field read functions with overlapping non-deferred and deferred fields while streaming", async () => {`, values `{"cc":1,"lines":112}`, ceiling cc 5, lines 107, nothing at the base matched
90. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10159` new, `test("applies field read functions when complete cache data inside a defer boundary fulfills the selection in intermediate chunks", async () => {`, values `{"cc":1,"lines":137}`, ceiling cc 5, lines 107, nothing at the base matched
91. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10297` new, `test("delivers cache updates written while a deferred response is still streaming", async () => {`, values `{"cc":1,"lines":118}`, ceiling cc 5, lines 107, nothing at the base matched
92. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10416` new, `test("keeps dataState streaming for an optimistic cache write while a deferred response is still streaming", async () => {`, values `{"cc":1,"lines":118}`, ceiling cc 5, lines 107, nothing at the base matched
93. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10535` new, `test("delivers an optimistic cache write while a deferred response is still streaming", async () => {`, values `{"cc":1,"lines":114}`, ceiling cc 5, lines 107, nothing at the base matched
94. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:10650` new, `test("delivers cache updates written while a deferred response is still streaming with 'returnPartialData: true'", async () => {`, values `{"cc":1,"lines":118}`, ceiling cc 5, lines 107, nothing at the base matched
95. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:2376` new, `test('returns partial cached stream list data as "partial" while streaming with a "cache-first" fetch policy and returnPartialData', async () => {`, values `{"cc":1,"lines":122}`, ceiling cc 5, lines 107, nothing at the base matched
96. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:2499` new, `test("reports data as partial if a cache merge function returns partial data", async () => {`, values `{"cc":1,"lines":141}`, ceiling cc 5, lines 107, nothing at the base matched
97. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:2834` new, `test("hides a partial cached stream array until the network supplies stream items with returnPartialData: false", async () => {`, values `{"cc":1,"lines":120}`, ceiling cc 5, lines 107, nothing at the base matched
98. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:3277` new, `test("only returns streamed items with field read functions with complete array and a network-only fetch policy", async () => {`, values `{"cc":1,"lines":115}`, ceiling cc 5, lines 107, nothing at the base matched
99. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:3475` new, `test("keeps all cached stream items including incomplete ones until the last chunk with returnPartialData: true", async () => {`, values `{"cc":1,"lines":123}`, ceiling cc 5, lines 107, nothing at the base matched
100. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:3693` new, `test("prunes undelivered defer fragments from partial cached fields inside a '@defer' boundary on streamed list items with returnPartialData: false", async () =`, values `{"cc":1,"lines":204}`, ceiling cc 5, lines 107, nothing at the base matched
101. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:3898` new, `test("prunes each streamed list item's sibling '@defer' boundaries independently by label", async () => {`, values `{"cc":1,"lines":208}`, ceiling cc 5, lines 107, nothing at the base matched
102. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4107` new, `test("does not surface complete cached '@defer' boundaries on streamed list items with returnPartialData: false", async () => {`, values `{"cc":1,"lines":179}`, ceiling cc 5, lines 107, nothing at the base matched
103. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4385` new, `test("applies field read functions to streamed items while truncating length with an incomplete stream item with returnPartialData: false", async () => {`, values `{"cc":1,"lines":113}`, ceiling cc 5, lines 107, nothing at the base matched
104. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4499` new, `test("applies field read functions to partial cached stream list data before and during streaming", async () => {`, values `{"cc":1,"lines":150}`, ceiling cc 5, lines 107, nothing at the base matched
105. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4650` new, `test("only reports streamed items from a partial stream array with a cache-first fetch policy", async () => {`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
106. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4760` new, `test("only reports streamed items from a partial stream array with a cache-and-network fetch policy", async () => {`, values `{"cc":1,"lines":109}`, ceiling cc 5, lines 107, nothing at the base matched
107. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4870` new, `test("applies field read functions to residual complete stream list items while later items are still streaming", async () => {`, values `{"cc":1,"lines":123}`, ceiling cc 5, lines 107, nothing at the base matched
108. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:4994` new, `test("applies field read functions when @stream delivers nested objects with overlapping non-stream fields", async () => {`, values `{"cc":1,"lines":200}`, ceiling cc 5, lines 107, nothing at the base matched
109. `src/core/__tests__/client.watchQuery/streamGraphQL17Alpha9.test.ts:5211` new, `test("does not emit when no data added when a '@stream' completes while a '@defer' boundary is still pending", async () => {`, values `{"cc":1,"lines":124}`, ceiling cc 5, lines 107, nothing at the base matched
110. `src/react/hooks/__tests__/useBackgroundQuery/customScalars.test.tsx:146` new, `test("preserves referential identity when refetching identical scalar values", async () => {`, values `{"cc":1,"lines":115}`, ceiling cc 5, lines 107, nothing at the base matched
111. `src/react/hooks/__tests__/useLazyQuery/customScalars.test.tsx:224` new, `test("preserves referential identity when re-executing with identical scalar values", async () => {`, values `{"cc":1,"lines":156}`, ceiling cc 5, lines 107, nothing at the base matched
112. `src/react/hooks/__tests__/useLoadableQuery/customScalars.test.tsx:163` new, `test("preserves referential identity when refetching identical scalar values", async () => {`, values `{"cc":1,"lines":120}`, ceiling cc 5, lines 107, nothing at the base matched
113. `src/react/hooks/__tests__/useLoadableQuery/testUtils.tsx:18` new, `export async function renderUseLoadableQueryHook<`, values `{"cc":1,"lines":108}`, ceiling cc 5, lines 107, nothing at the base matched
114. `src/react/hooks/__tests__/useQuery/customScalars.test.tsx:150` new, `test("preserves referential identity when refetching identical scalar values", async () => {`, values `{"cc":1,"lines":111}`, ceiling cc 5, lines 107, nothing at the base matched
115. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:26` new, `test("serializes scalar variables passed to refetch", async () => {`, values `{"cc":1,"lines":146}`, ceiling cc 5, lines 107, nothing at the base matched
116. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:173` new, `test("serializes scalar variables passed to fetchMore", async () => {`, values `{"cc":1,"lines":148}`, ceiling cc 5, lines 107, nothing at the base matched
117. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:322` new, `test("serializes scalar variables passed to subscribeToMore", async () => {`, values `{"cc":1,"lines":151}`, ceiling cc 5, lines 107, nothing at the base matched
118. `src/react/hooks/__tests__/useQueryRefHandlers/customScalars.test.tsx:474` new, `test("preserves referential identity when refetching identical scalar values", async () => {`, values `{"cc":1,"lines":156}`, ceiling cc 5, lines 107, nothing at the base matched
119. `src/react/hooks/__tests__/useSuspenseQuery/streamGraphQL17Alpha9.test.tsx:1561` new, `test("reports data as partial if a cache merge function returns partial data", async () => {`, values `{"cc":1,"lines":147}`, ceiling cc 5, lines 107, nothing at the base matched
120. `src/utilities/internal/addDeferFragmentLabels.ts:12` new, `Directive(node) {`, values `{"cc":6,"lines":24}`, ceiling cc 5, lines 107, nothing at the base matched
121. `src/utilities/internal/coerceScalarFieldsToParsed.ts:18` new, `export function coerceScalarFieldsToParsed(`, values `{"cc":1,"lines":125}`, ceiling cc 5, lines 107, nothing at the base matched
122. `src/utilities/internal/coerceScalarFieldsToParsed.ts:49` new, `function coerceField(`, values `{"cc":11,"lines":43}`, ceiling cc 5, lines 107, nothing at the base matched
123. `src/utilities/internal/coerceScalarFieldsToParsed.ts:108` new, `workSet.forEach((selection) => {`, values `{"cc":7,"lines":25}`, ceiling cc 5, lines 107, nothing at the base matched
124. `src/utilities/internal/isDeferredFragment.ts:17` new, `return !!fragmentSelection.directives?.some((directive) => {`, values `{"cc":7,"lines":18}`, ceiling cc 5, lines 107, nothing at the base matched
125. `src/utilities/internal/isStreamField.ts:17` new, `return !!field.directives?.some((directive) => {`, values `{"cc":7,"lines":18}`, ceiling cc 5, lines 107, nothing at the base matched
126. `src/cache/inmemory/entityStore.ts:123` worsened, `public merge(older: string | StoreObject, newer: StoreObject | string): void {`, values `{"cc":15,"lines":100}`, ceiling cc 5, lines 107, base site `src/cache/inmemory/entityStore.ts:115` with `{"cc":13,"lines":90}`
127. `src/cache/inmemory/policies.ts:555` worsened, `Object.keys(fields).forEach((fieldName) => {`, values `{"cc":20,"lines":87}`, ceiling cc 5, lines 107, base site `src/cache/inmemory/policies.ts:551` with `{"cc":11,"lines":47}`
128. `src/cache/inmemory/readFromStore.ts:470` worsened, `private execSelectionSetImpl({`, values `{"cc":7,"lines":250}`, ceiling cc 5, lines 107, base site `src/cache/inmemory/readFromStore.ts:274` with `{"cc":7,"lines":133}`
129. `src/cache/inmemory/readFromStore.ts:519` worsened, `workSet.forEach((selection) => {`, values `{"cc":32,"lines":180}`, ceiling cc 5, lines 107, base site `src/cache/inmemory/readFromStore.ts:319` with `{"cc":16,"lines":75}`
130. `src/cache/inmemory/readFromStore.ts:747` worsened, `array = array.map((item, i) => {`, values `{"cc":10,"lines":54}`, ceiling cc 5, lines 107, base site `src/cache/inmemory/readFromStore.ts:431` with `{"cc":6,"lines":38}`
131. `src/core/ObservableQuery.ts:401` worsened, `constructor({`, values `{"cc":7,"lines":78}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:370` with `{"cc":6,"lines":65}`
132. `src/core/ObservableQuery.ts:636` worsened, `private getInitialResult(`, values `{"cc":9,"lines":58}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:589` with `{"cc":9,"lines":56}`
133. `src/core/ObservableQuery.ts:647` worsened, `const cacheResult = (): ObservableQuery.Result<TData> => {`, values `{"cc":6,"lines":23}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:600` with `{"cc":7,"lines":21}`
134. `src/core/ObservableQuery.ts:1582` worsened, `private _reobserve(`, values `{"cc":27,"lines":164}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:1537` with `{"cc":26,"lines":159}`
135. `src/core/ObservableQuery.ts:1819` worsened, `public notify(scheduled = false) {`, values `{"cc":20,"lines":128}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:1769` with `{"cc":11,"lines":64}`
136. `src/core/ObservableQuery.ts:2073` worsened, `> = filterMap((notification) => {`, values `{"cc":32,"lines":133}`, ceiling cc 5, lines 107, base site `src/core/ObservableQuery.ts:1938` with `{"cc":31,"lines":132}`
137. `src/core/QueryInfo.ts:146` worsened, `public markQueryResult(`, values `{"cc":10,"lines":156}`, ceiling cc 5, lines 107, base site `src/core/QueryInfo.ts:209` with `{"cc":4,"lines":141}`
138. `src/core/QueryInfo.ts:255` worsened, `update: (cache) => {`, values `{"cc":9,"lines":43}`, ceiling cc 5, lines 107, base site `src/core/QueryInfo.ts:264` with `{"cc":8,"lines":79}`
139. `src/core/QueryManager.ts:888` worsened, `private getObservableFromLink<TData = unknown>(`, values `{"cc":11,"lines":153}`, ceiling cc 5, lines 107, base site `src/core/QueryManager.ts:878` with `{"cc":11,"lines":152}`
140. `src/core/QueryManager.ts:1042` worsened, `private getResultsFromLink<TData, TVariables extends OperationVariables>(`, values `{"cc":1,"lines":116}`, ceiling cc 5, lines 107, base site `src/core/QueryManager.ts:1031` with `{"cc":1,"lines":114}`
141. `src/core/QueryManager.ts:1159` worsened, `public fetchObservableWithInfo<TData, TVariables extends OperationVariables>(`, values `{"cc":6,"lines":168}`, ceiling cc 5, lines 107, base site `src/core/QueryManager.ts:1146` with `{"cc":6,"lines":167}`
142. `src/core/QueryManager.ts:1551` worsened, `private fetchQueryByPolicy<TData, TVariables extends OperationVariables>(`, values `{"cc":11,"lines":268}`, ceiling cc 5, lines 107, base site `src/core/QueryManager.ts:1537` with `{"cc":11,"lines":230}`
143. `src/core/QueryManager.ts:1592` worsened, `const resultsFromCache = (`, values `{"cc":13,"lines":106}`, ceiling cc 5, lines 107, base site `src/core/QueryManager.ts:1575` with `{"cc":13,"lines":102}`
144. `src/core/__tests__/client.watchQuery/defer20220824.test.ts:170` worsened, `async (fetchPolicy) => {`, values `{"cc":3,"lines":180}`, ceiling cc 5, lines 107, base site `src/core/__tests__/client.watchQuery/defer20220824.test.ts:170` with `{"cc":2,"lines":179}`
145. `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:206` worsened, `async (fetchPolicy) => {`, values `{"cc":3,"lines":193}`, ceiling cc 5, lines 107, base site `src/core/__tests__/client.watchQuery/deferGraphQL17Alpha9.test.ts:180` with `{"cc":2,"lines":191}`
146. `src/incremental/handlers/graphql17Alpha9.ts:127` worsened, `handle(`, values `{"cc":19,"lines":159}`, ceiling cc 5, lines 107, base site `src/incremental/handlers/graphql17Alpha9.ts:109` with `{"cc":19,"lines":153}`
147. `src/react/hooks/__tests__/useSuspenseQuery/streamGraphQL17Alpha9.test.tsx:370` worsened, `test('does not suspend streamed queries with partial data in the cache and using a "cache-first" fetch policy with 'returnPartialData'', async () => {`, values `{"cc":1,"lines":128}`, ceiling cc 5, lines 107, base site `src/react/hooks/__tests__/useSuspenseQuery/streamGraphQL17Alpha9.test.tsx:371` with `{"cc":1,"lines":122}`
148. `src/react/hooks/useSubscription.ts:228` worsened, `export function useSubscription<`, values `{"cc":14,"lines":174}`, ceiling cc 5, lines 107, base site `src/react/hooks/useSubscription.ts:216` with `{"cc":13,"lines":163}`
149. `src/utilities/internal/types/Exact.ts` unparsed, `the TypeScript grammar rejected it`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R055

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: dead-symbols, ERR
- Decision group: dead-symbols

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `src/incremental/__benches__/types.bench.ts:20` new, `type _TypeCacheWarmup =`, values `{"dead":1}`, nothing at the base matched

   ```text
   17 |   | { __typename: "Unrelated"; extra?: never }
   18 | );
   19 | // @ts-ignore
   20 | type _TypeCacheWarmup =
   21 |   | GraphQLCodegenIncremental.Complete<UnrelatedStreaming>
   22 |   | GraphQLCodegenIncremental.Partial<UnrelatedStreaming>;
   23 | 
   24 | test("assembles inline single-field @defer", (prefix) => {
   ```

2. `src/utilities/internal/types/Exact.ts` unparsed, `the TypeScript grammar rejected it`

### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R056

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: reachability, FAIL
- Decision group: reachability
- Derived reachability: `[{"name":"scripts/codemods/ac3-to-ac4/src/util/find*.ts","pattern":"find*.ts","roots":["scripts/codemods/ac3-to-ac4/src/util"]},{"name":"scripts/codemods/ac3-to-ac4/src/util/get*.ts","pattern":"get*.ts","roots":["scripts/codemods/ac3-to-ac4/src/util"]},{"name":"src/config/jest/*Equal.ts","pattern":"*Equal.ts","roots":["src/config/jest"]},{"name":"src/dev/load*.ts","pattern":"load*.ts","roots":["src/dev"]},{"name":"src/errors/*Error.ts","pattern":"*Error.ts","roots":["src/errors"]},{"name":"src/testing/internal/multipart/mock*.ts","pattern":"mock*.ts","roots":["src/testing/internal/multipart"]},{"name":"src/testing/matchers/toBe*.ts","pattern":"toBe*.ts","roots":["src/testing/matchers"]},{"name":"src/testing/matchers/toEmit*.ts","pattern":"toEmit*.ts","roots":["src/testing/matchers"]},{"name":"src/testing/matchers/toHave*.ts","pattern":"toHave*.ts","roots":["src/testing/matchers"]},{"name":"src/utilities/internal/*Array.ts","pattern":"*Array.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/*Deep.ts","pattern":"*Deep.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/*Definition.ts","pattern":"*Definition.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/*Document.ts","pattern":"*Document.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/*Field.ts","pattern":"*Field.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/*Promise.ts","pattern":"*Promise.ts","roots":["src/utilities/internal"]},{"name":"src/utilities/internal/is*.ts","pattern":"is*.ts","roots":["src/utilities/internal"]}]`, the file families the derivation commit proves reached

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: where no other file references a declaration of the file.

1. `src/utilities/internal/__tests__/isDeferredFragment.test.ts` new, `file`, values `{"sibling":"src/utilities/internal/isDeferredFragment.ts","unreached":1}`, nothing at the base matched
2. `src/utilities/internal/__tests__/isStreamField.test.ts` new, `file`, values `{"sibling":"src/utilities/internal/isDeferredFragment.ts","unreached":1}`, nothing at the base matched
3. `src/utilities/internal/isTypenameField.ts` new, `file`, values `{"sibling":"src/utilities/internal/argumentsObjectFromField.ts","unreached":1}`, nothing at the base matched

### Remedy klin printed

> Wire this file into the application through a real source reference, or delete it if the implementation is unused. If a public-api break names what an unreached file held, decide the two separately: restore the public contract where the task keeps it, or leave the break for a person to accept where the task removes it, and keep and wire the implementation if it is still needed, and delete it only if it is unused.

## R057

- Repository: `apollographql/apollo-client` (TypeScript), change 7 of 10
- Commit: `0c925a434823`, judged against its first parent `70e3a11d93c8`, exit 2
- Gate: public-api, ERR
- Decision group: public-api

### Commit message

> Release 4.3 (#13243)
>
> Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
> Co-authored-by: Dale Seo <5466341+DaleSeo@users.noreply.github.com>
> Co-authored-by: jerelmiller <565661+jerelmiller@users.noreply.github.com>
> Co-authored-by: Amariah Abishai <110414493+AmariahAK@users.noreply.github.com>
> Co-authored-by: DeepSeek V4 Pro agent <agent@atlarix.dev>
> Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>
> Co-authored-by: jcostello-atlassian <64562665+jcostello-atlassian@users.noreply.github.com>
> Co-authored-by: atharv-sys32 <atharvpandey245@gmail.com>
> Co-authored-by: Parker <parker.ragland@apollographql.com>
> Co-authored-by: apollo-librarian[bot] <212934294+apollo-librarian[bot]@users.noreply.github.com>

### Files the change touched

```text
 .api-reports/api-report-cache.api.md                         | 206 ++++++++++++++++++++++++---
 .api-reports/api-report-core.api.md                          |  83 +++++++----
 .api-reports/api-report-incremental.api.md                   | 154 ++++++++++++++++++--
 .api-reports/api-report-link_utils.api.md                    |   2 +-
 .api-reports/api-report-local-state.api.md                   |  13 +-
 .api-reports/api-report-masking.api.md                       |   6 +-
 .api-reports/api-report-react.api.md                         | 285 ++++++++++++++++++-------------------
 .api-reports/api-report-react_ssr.api.md                     |   4 +-
 .api-reports/api-report-testing.api.md                       |   4 +-
 .api-reports/api-report-utilities.api.md                     |   4 +
 .api-reports/api-report-utilities_internal.api.md            |  93 ++++++++++--
 .api-reports/api-report-utilities_subscriptions_relay.api.md |   2 +-
 .api-reports/api-report.api.md                               | 389 +++++++++++++++++++++++++++++++++++++++++----------
 .changeset/afraid-starfishes-smash.md                        | 101 +++++++++++++
 .changeset/angry-baboons-decide.md                           |  97 +++++++++++++
 .changeset/beige-colts-flow.md                               |   5 +
 .changeset/big-scissors-hope.md                              |  32 +++++
 .changeset/chilly-actors-complain.md                         |   5 +
 .changeset/cold-comics-add.md                                |  18 +++
 .changeset/cyan-camels-think.md                              |   5 +
 .changeset/dirty-donuts-punch.md                             |   5 +
 .changeset/eighty-files-sort.md                              |  36 +++++
 .changeset/empty-pears-tell.md                               |   5 +
 .changeset/fast-geckos-help.md                               |  34 +++++
 .changeset/fifty-books-return.md                             |  32 +++++
 .changeset/forty-trainers-switch.md                          |   5 +
 .changeset/fuzzy-hairs-tie.md                                |  34 +++++
 .changeset/gorgeous-tools-dream.md                           |   5 +
 .changeset/honest-lobsters-run.md                            |   7 +
 .changeset/hungry-onions-sleep.md                            |   6 +
 ...
 266 files changed, 60772 insertions(+), 2206 deletions(-)
```

### Findings

Condition: where an external surface or item the base exposed is gone or its declared contract changed.

1. `@apollo/client ".":23` new, `MutationOptions (type)`, values `{"break":1,"kind":"changed","now":"type MutationOptions<TData = unknown, TVariables extends OperationVariables = OperationVariables, TCache extends Cache.Implementation = Cache.Implementation> = ApolloClient.MutateOptions<TData, TVariables, TCache>","origin":"src/core/deprecated.ts:23","was":"type MutationOptions<TData = unknown, TVariables extends OperationVariables = OperationVariables, TCache extends ApolloCache = ApolloCache> = ApolloClient.MutateOptions<TData, TVariables, TCache>"}`, nothing at the base matched
2. `@apollo/client ".":40` new, `RefetchQueriesOptions (type)`, values `{"break":1,"kind":"changed","now":"type RefetchQueriesOptions<TCache extends Cache.Implementation, TResult> = ApolloClient.RefetchQueriesOptions<TCache, TResult>","origin":"src/core/deprecated.ts:40","was":"type RefetchQueriesOptions<TCache extends ApolloCache, TResult> = ApolloClient.RefetchQueriesOptions<TCache, TResult>"}`, nothing at the base matched
3. `@apollo/client ".":289` new, `InternalRefetchQueriesOptions (type)`, values `{"break":1,"kind":"changed","now":"interface InternalRefetchQueriesOptions<TCache extends Cache.Implementation, TResult> extends Omit<ApolloClient.RefetchQueriesOptions<TCache, TResult>, \"include\"> { include?: InternalRefetchQueriesInclude; removeOptimistic?: string }","origin":"src/core/types.ts:289","was":"interface InternalRefetchQueriesOptions<TCache extends ApolloCache, TResult> extends Omit<ApolloClient.RefetchQueriesOptions<TCache, TResult>, \"include\"> { include?: InternalRefetchQueriesInclude; removeOptimistic?: string }"}`, nothing at the base matched
4. `@apollo/client ".":324` new, `ObservableQuery (type)`, values `{"break":1,"kind":"changed","now":"class ObservableQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables> implements Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>, InteropObservable<ObservableQuery.Result<MaybeMasked<TData>>> { constructor(_: { queryManager: QueryManager; options: ApolloClient.WatchQueryOptions<TData, TVariables>; transformedQuery?: DocumentNode | TypedDocumentNode<TData, TVariables>; queryId?: string; }); public applyOptions(_: Partial<ObservableQuery.Options<TData, TVariables>>): void; public async setVariables(_: TVariables): Promise<ApolloClient.QueryResult<TData>>; public fetchMore<TFetchData = TData, TFetchVars extends OperationVariables = TVariables, TErrorPolicy extends ErrorPolicy = \"none\">(_: ObservableQuery.FetchMoreOptions<TData, TVariables, TFetchData, TFetchVars> &{ errorPolicy?: TErrorPolicy; }): Promise<ApolloClient.QueryResult<TFetchData, TErrorPolicy>>; public get query(): TypedDocumentNode<TData, TVariables>; public get variables(): TVariables; public getCacheDiff(_?: ?): ?; public getCurrentResult(): ObservableQuery.Result<MaybeMasked<TData>>; public hasObservers(): ?; public notify(_?: ?): ?; public pipe!: Observable<ObservableQuery.Result<MaybeMasked<TData>>>[\"pipe\"]; public readonly options: ObservableQuery.Options<TData, TVariables>; public readonly queryName?: string; public refetch(_?: Partial<TVariables>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<TData>>; public reobserve(_?: Partial<ObservableQuery.Options<TData, TVariables>>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<MaybeMasked<TData>>>; public reset(): ?; public startPolling(_: number): ?; public stop(): ?; public stopPolling(): ?; public subscribe!: (_: | Partial<Observer<ObservableQuery.Result<MaybeMasked<TData>>>> | ((_: ObservableQuery.Result<MaybeMasked<TData>>) => void)) => Subscription; public subscribeToMore<TSubscriptionData = TData, TSubscriptionVariables extends OperationVariables = TVariables>(_: ObservableQuery.SubscribeToMoreOptions<TData, TSubscriptionVariables, TSubscriptionData, TVariables>): () => void; public updateQuery(_: UpdateQueryMapFn<TData, TVariables>): void; public[\"@@observable\"]: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>; public[Symbol.observable]!: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>> }","origin":"src/core/ObservableQuery.ts:324","was":"class ObservableQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables> implements Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>, InteropObservable<ObservableQuery.Result<MaybeMasked<TData>>> { constructor(_: { queryManager: QueryManager; options: ApolloClient.WatchQueryOptions<TData, TVariables>; transformedQuery?: DocumentNode | TypedDocumentNode<TData, TVariables>; queryId?: string; }); public _lastWrite?: unknown; public applyOptions(_: Partial<ObservableQuery.Options<TData, TVariables>>): void; public async setVariables(_: TVariables): Promise<ApolloClient.QueryResult<TData>>; public fetchMore<TFetchData = TData, TFetchVars extends OperationVariables = TVariables, TErrorPolicy extends ErrorPolicy = \"none\">(_: ObservableQuery.FetchMoreOptions<TData, TVariables, TFetchData, TFetchVars> &{ errorPolicy?: TErrorPolicy; }): Promise<ApolloClient.QueryResult<TFetchData, TErrorPolicy>>; public get query(): TypedDocumentNode<TData, TVariables>; public get variables(): TVariables; public getCacheDiff(_?: ?): ?; public getCurrentResult(): ObservableQuery.Result<MaybeMasked<TData>>; public hasObservers(): ?; public notify(_?: ?): ?; public pipe!: Observable<ObservableQuery.Result<MaybeMasked<TData>>>[\"pipe\"]; public readonly options: ObservableQuery.Options<TData, TVariables>; public readonly queryName?: string; public refetch(_?: Partial<TVariables>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<TData>>; public reobserve(_?: Partial<ObservableQuery.Options<TData, TVariables>>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<MaybeMasked<TData>>>; public reset(): ?; public startPolling(_: number): ?; public stop(): ?; public stopPolling(): ?; public subscribe!: (_: | Partial<Observer<ObservableQuery.Result<MaybeMasked<TData>>>> | ((_: ObservableQuery.Result<MaybeMasked<TData>>) => void)) => Subscription; public subscribeToMore<TSubscriptionData = TData, TSubscriptionVariables extends OperationVariables = TVariables>(_: ObservableQuery.SubscribeToMoreOptions<TData, TSubscriptionVariables, TSubscriptionData, TVariables>): () => void; public updateQuery(_: UpdateQueryMapFn<TData, TVariables>): void; public[\"@@observable\"]: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>; public[Symbol.observable]!: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>> }"}`, nothing at the base matched
5. `@apollo/client ".":375` new, `MutationUpdaterFunction (type)`, values `{"break":1,"kind":"changed","now":"type MutationUpdaterFunction<TData, TVariables extends OperationVariables, TCache extends Cache.Implementation> = (_: TCache, _: FormattedExecutionResult<Unmasked<TData>>, _: { context?: DefaultContext; variables: TVariables; }) => void","origin":"src/core/types.ts:375","was":"type MutationUpdaterFunction<TData, TVariables extends OperationVariables, TCache extends ApolloCache> = (_: TCache, _: FormattedExecutionResult<Unmasked<TData>>, _: { context?: DefaultContext; variables?: TVariables; }) => void"}`, nothing at the base matched
6. `@apollo/client ".":1035` new, `ApolloClient (type)`, values `{"break":1,"kind":"changed","now":"class ApolloClient { constructor(_: ApolloClient.Options); get documentTransform(): ?; get localState(): LocalState | undefined; public __actionHookForDevTools(_: () => any): ?; public __requestRaw(_: ApolloLink.Request): Observable<ApolloLink.Result<unknown>>; public cache: Cache.Implementation; public clearStore(): Promise<any[]>; public declare getMemoryInternals?: typeof getApolloClientMemoryInternals; public defaultOptions: ApolloClient.DefaultOptions; public disableNetworkFetches!: never; public extract(_?: boolean): ?; public get defaultContext(): ?; public get prioritizeCacheValues(): ?; public getObservableQueries(_?: RefetchQueriesInclude): Set<ObservableQuery<any>>; public link: ApolloLink; public mutate: ApolloClient.mutate.Signature; public onClearStore(_: () => Promise<any>): () => void; public onResetStore(_: () => Promise<any>): () => void; public query: ApolloClient.query.Signature; public queryDeduplication: boolean; public reFetchObservableQueries: (_?: boolean) => Promise<ApolloClient.QueryResult<any>[]>; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly devtoolsConfig: ApolloClient.DevtoolsOptions; public readonly refetchEventManager: RefetchEventManager | undefined; public refetchObservableQueries(_?: boolean): Promise<ApolloClient.QueryResult<any>[]>; public refetchQueries<TCache extends Cache.Implementation = Cache.Implementation, TResult = Promise<ApolloClient.QueryResult<any>>>(_: ApolloClient.RefetchQueriesOptions<TCache, TResult>): ApolloClient.RefetchQueriesResult<TResult>; public resetStore(): Promise<ApolloClient.QueryResult<any>[] | null>; public restore(_: unknown): ?; public set prioritizeCacheValues(_: boolean): ?; public setLink(_: ApolloLink): ?; public stop(): ?; public subscribe<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.SubscribeOptions<TData, TVariables>): SubscriptionObservable<ApolloClient.SubscribeResult<MaybeMasked<TData>>>; public version: string; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloClient.ObservableFragment<Array<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloClient.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloClient.ObservableFragment<Array<TData | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloClient.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloClient.ObservableFragment<TData>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables>): ApolloClient.ObservableFragment<TData | null>; public watchQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchQueryOptions<TData, TVariables>): ObservableQuery<TData, TVariables>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteQueryOptions<TData, TVariables>): Reference | undefined; set localState(_: LocalState): ? }","origin":"src/core/ApolloClient.ts:1035","was":"class ApolloClient { constructor(_: ApolloClient.Options); get documentTransform(): ?; get localState(): LocalState | undefined; public __actionHookForDevTools(_: () => any): ?; public __requestRaw(_: ApolloLink.Request): Observable<ApolloLink.Result<unknown>>; public cache: ApolloCache; public clearStore(): Promise<any[]>; public declare getMemoryInternals?: typeof getApolloClientMemoryInternals; public defaultOptions: ApolloClient.DefaultOptions; public disableNetworkFetches!: never; public extract(_?: boolean): ?; public get defaultContext(): ?; public get prioritizeCacheValues(): ?; public getObservableQueries(_?: RefetchQueriesInclude): Set<ObservableQuery<any>>; public link: ApolloLink; public mutate: ApolloClient.mutate.Signature; public onClearStore(_: () => Promise<any>): () => void; public onResetStore(_: () => Promise<any>): () => void; public query: ApolloClient.query.Signature; public queryDeduplication: boolean; public reFetchObservableQueries: (_?: boolean) => Promise<ApolloClient.QueryResult<any>[]>; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly devtoolsConfig: ApolloClient.DevtoolsOptions; public readonly refetchEventManager: RefetchEventManager | undefined; public refetchObservableQueries(_?: boolean): Promise<ApolloClient.QueryResult<any>[]>; public refetchQueries<TCache extends ApolloCache = ApolloCache, TResult = Promise<ApolloClient.QueryResult<any>>>(_: ApolloClient.RefetchQueriesOptions<TCache, TResult>): ApolloClient.RefetchQueriesResult<TResult>; public resetStore(): Promise<ApolloClient.QueryResult<any>[] | null>; public restore(_: unknown): ?; public set prioritizeCacheValues(_: boolean): ?; public setLink(_: ApolloLink): ?; public stop(): ?; public subscribe<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.SubscribeOptions<TData, TVariables>): SubscriptionObservable<ApolloClient.SubscribeResult<MaybeMasked<TData>>>; public version: string; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloClient.ObservableFragment<Array<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloClient.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloClient.ObservableFragment<Array<TData | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloClient.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloClient.ObservableFragment<TData>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables>): ApolloClient.ObservableFragment<TData | null>; public watchQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchQueryOptions<TData, TVariables>): ObservableQuery<TData, TVariables>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteQueryOptions<TData, TVariables>): Reference | undefined; set localState(_: LocalState): ? }"}`, nothing at the base matched
7. `@apollo/client "./cache":97` new, `InMemoryCache (type)`, values `{"break":1,"kind":"changed","now":"class InMemoryCache extends ApolloCache { constructor(..._: { } extends InMemoryCache.ScalarsOption? [_?: InMemoryCacheConfig]: [_: InMemoryCacheConfig]); protected broadcastWatches(_?: BroadcastOptions): ?; protected config: InMemoryCacheConfig; public batch<TUpdateResult>(_: Cache.BatchOptions<InMemoryCache, TUpdateResult>): TUpdateResult; public configuresScalars(): boolean; public declare getMemoryInternals?: typeof getInMemoryCacheMemoryInternals; public diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables> &{ [handleIncrementalSymbol]: DiffIncrementalInfo | undefined; }): Cache.InternalDiffResultWithDataState<TData>; public diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables>): Cache.DiffResult<TData>; public evict(_: Cache.EvictOptions): boolean; public extract(_?: boolean): NormalizedCacheObject; public fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string): boolean; public gc(_?: { resetResultCache?: boolean; }): ?; public getRootTypename(_: OperationTypeNode): string; public getScalar<TKey extends keyof ApolloCache.Scalars>(_: TKey): ApolloCache.GetScalarType<TKey> extends(Scalar<infer TSerialized, infer TParsed>)? IsLooselyEqual<TSerialized, TParsed> extends true? ApolloCache.GetScalarType<TKey> | undefined: ApolloCache.GetScalarType<TKey>: never; public getScalarTypeForField(_: string, _: string): ScalarType | undefined; public identify(_: StoreObject | Reference): string | undefined; public lookupFragment(_: string): FragmentDefinitionNode | null; public modify<Entity extends Record<string, any> = Record<string, any>>(_: Cache.ModifyOptions<Entity>): boolean; public performTransaction(_: (_: InMemoryCache) => any, _?: string | null): ?; public read<TData = unknown>(_: Cache.ReadOptions<TData, OperationVariables> &{ returnPartialData: true; }): TData | DeepPartial<TData> | null; public read<TData = unknown>(_: Cache.ReadOptions<TData, OperationVariables>): TData | null; public readonly assumeImmutableResults: ?; public readonly makeVar: ?; public readonly policies: Policies; public release(_: string, _?: boolean): number; public removeOptimistic(_: string): ?; public reset(_?: Cache.ResetOptions): Promise<void>; public resolvesClientField(_: string, _: string): boolean; public restore(_: NormalizedCacheObject): this; public retain(_: string, _?: boolean): number; public serializeVariables<TVariables extends OperationVariables = OperationVariables>(_: DocumentNode | TypedDocumentNode<any, TVariables>, _: NoInfer<TVariables>): TVariables; public serializeVariables<TVariables extends OperationVariables = OperationVariables>(_: DocumentNode | TypedDocumentNode<any, TVariables>, _: NoInfer<TVariables> | undefined): TVariables | undefined; public transformDocument(_: DocumentNode): DocumentNode; public watch<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WatchOptions<TData, TVariables>): () => void; public write<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteOptions<TData, TVariables>): Reference | undefined }","origin":"src/cache/inmemory/inMemoryCache.ts:97","was":"class InMemoryCache extends ApolloCache { constructor(_?: InMemoryCacheConfig); protected broadcastWatches(_?: BroadcastOptions): ?; protected config: InMemoryCacheConfig; public batch<TUpdateResult>(_: Cache.BatchOptions<InMemoryCache, TUpdateResult>): TUpdateResult; public declare getMemoryInternals?: typeof getInMemoryCacheMemoryInternals; public diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables>): Cache.DiffResult<TData>; public evict(_: Cache.EvictOptions): boolean; public extract(_?: boolean): NormalizedCacheObject; public fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string): boolean; public gc(_?: { resetResultCache?: boolean; }): ?; public identify(_: StoreObject | Reference): string | undefined; public lookupFragment(_: string): FragmentDefinitionNode | null; public modify<Entity extends Record<string, any> = Record<string, any>>(_: Cache.ModifyOptions<Entity>): boolean; public performTransaction(_: (_: InMemoryCache) => any, _?: string | null): ?; public read<TData = unknown>(_: Cache.ReadOptions<TData, OperationVariables> &{ returnPartialData: true; }): TData | DeepPartial<TData> | null; public read<TData = unknown>(_: Cache.ReadOptions<TData, OperationVariables>): TData | null; public readonly assumeImmutableResults: ?; public readonly makeVar: ?; public readonly policies: Policies; public release(_: string, _?: boolean): number; public removeOptimistic(_: string): ?; public reset(_?: Cache.ResetOptions): Promise<void>; public resolvesClientField(_: string, _: string): boolean; public restore(_: NormalizedCacheObject): this; public retain(_: string, _?: boolean): number; public transformDocument(_: DocumentNode): DocumentNode; public watch<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WatchOptions<TData, TVariables>): () => void; public write<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteOptions<TData, TVariables>): Reference | undefined }"}`, nothing at the base matched
8. `@apollo/client "./cache":155` new, `FieldPolicy (type)`, values `{"break":1,"kind":"changed","now":"type FieldPolicy<TExisting = any, TIncoming = TExisting, TReadResult = TIncoming, TReadOptions extends FieldReadFunctionOptions = FieldReadFunctionOptions, TMergeOptions extends FieldMergeFunctionOptions = FieldMergeFunctionOptions> = { keyArgs?: KeySpecifier | KeyArgsFunction | false; read?: FieldReadFunction<TExisting, TReadResult, TReadOptions>; merge?: FieldMergeFunction<TExisting, TIncoming, TMergeOptions> | boolean; scalar?: ScalarType; }","origin":"src/cache/inmemory/policies.ts:155","was":"type FieldPolicy<TExisting = any, TIncoming = TExisting, TReadResult = TIncoming, TReadOptions extends FieldReadFunctionOptions = FieldReadFunctionOptions, TMergeOptions extends FieldMergeFunctionOptions = FieldMergeFunctionOptions> = { keyArgs?: KeySpecifier | KeyArgsFunction | false; read?: FieldReadFunction<TExisting, TReadResult, TReadOptions>; merge?: FieldMergeFunction<TExisting, TIncoming, TMergeOptions> | boolean; }"}`, nothing at the base matched
9. `@apollo/client "./cache":158` new, `InMemoryCacheConfig (type)`, values `{"break":1,"kind":"changed","now":"type InMemoryCacheConfig = ApolloReducerConfig &{ resultCaching?: boolean; possibleTypes?: PossibleTypesMap; typePolicies?: TypePolicies; fragments?: FragmentRegistryAPI; inputObjects?: InputObjectsOption; } &({ } extends InMemoryCache.ScalarsOption? InMemoryCache.ScalarsOption extends Record<string, never>? { scalars?: Record<string, ` Scalar types must be declared in ApolloCache.Scalars before usage. See https://www.apollographql.com/docs/react/data/typescript#declaring-scalar-types. `>; }: { scalars?: InMemoryCache.ScalarsOption }: { scalars: InMemoryCache.ScalarsOption })","origin":"src/cache/inmemory/types.ts:158","was":"interface InMemoryCacheConfig extends ApolloReducerConfig { fragments?: FragmentRegistryAPI; possibleTypes?: PossibleTypesMap; resultCaching?: boolean; typePolicies?: TypePolicies }"}`, nothing at the base matched
10. `@apollo/client "./cache":224` new, `ApolloCache (type)`, values `{"break":1,"kind":"changed","now":"abstract class ApolloCache { protected onAfterBroadcast: ?; public abstract diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables> &{ [handleIncrementalSymbol]: DiffIncrementalInfo | undefined; }): Cache.InternalDiffResultWithDataState<TData> | Cache.DiffResult<TData>; public abstract diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables>): Cache.DiffResult<TData>; public abstract evict(_: Cache.EvictOptions): boolean; public abstract extract(_?: boolean): unknown; public abstract fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string): boolean; public abstract performTransaction(_: Transaction, _?: string | null): void; public abstract read<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadOptions<TData, TVariables>): Unmasked<TData> | null; public abstract removeOptimistic(_: string): void; public abstract reset(_?: Cache.ResetOptions): Promise<void>; public abstract restore(_: unknown): this; public abstract watch<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WatchOptions<TData, TVariables>): () => void; public abstract write<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteOptions<TData, TVariables>): Reference | undefined; public batch<U>(_: Cache.BatchOptions<this, U>): U; public configuresScalars(): ?; public declare getMemoryInternals?: typeof getApolloCacheMemoryInternals; public gc(): string[]; public getRootTypename(_: OperationTypeNode): string; public getScalar<TKey extends keyof ApolloCache.Scalars>(_: TKey): ApolloCache.GetScalarType<TKey> | undefined; public getScalarTypeForField(_: string, _: string): ScalarType | undefined; public identify(_: StoreObject | Reference): string | undefined; public lookupFragment(_: string): FragmentDefinitionNode | null; public modify<Entity extends Record<string, any> = Record<string, any>>(_: Cache.ModifyOptions<Entity>): boolean; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly assumeImmutableResults: boolean; public recordOptimisticTransaction(_: Transaction, _: string): ?; public resolvesClientField? (_: string, _: string): boolean; public serializeVariables<TVariables extends OperationVariables = OperationVariables>(_: DocumentNode | TypedDocumentNode<any, TVariables>, _: NoInfer<TVariables>): TVariables; public serializeVariables<TVariables extends OperationVariables = OperationVariables>(_: DocumentNode | TypedDocumentNode<any, TVariables>, _: NoInfer<TVariables> | undefined): TVariables | undefined; public transformDocument(_: DocumentNode): DocumentNode; public transformForLink(_: DocumentNode): DocumentNode; public updateFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.UpdateFragmentOptions<TData, TVariables>, _: (_: Unmasked<TData> | null) => Unmasked<TData> | null | void): Unmasked<TData> | null; public updateQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.UpdateQueryOptions<TData, TVariables>, _: (_: Unmasked<TData> | null) => Unmasked<TData> | null | void): Unmasked<TData> | null; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloCache.ObservableFragment<Array<Unmasked<TData>>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloCache.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloCache.ObservableFragment<Array<Unmasked<TData> | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloCache.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloCache.ObservableFragment<Unmasked<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables>): ApolloCache.ObservableFragment<Unmasked<TData> | null>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteQueryOptions<TData, TVariables>): Reference | undefined }","origin":"src/cache/core/cache.ts:224","was":"abstract class ApolloCache { protected onAfterBroadcast: ?; public abstract diff<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.DiffOptions<TData, TVariables>): Cache.DiffResult<TData>; public abstract evict(_: Cache.EvictOptions): boolean; public abstract extract(_?: boolean): unknown; public abstract fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string): boolean; public abstract performTransaction(_: Transaction, _?: string | null): void; public abstract read<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadOptions<TData, TVariables>): Unmasked<TData> | null; public abstract removeOptimistic(_: string): void; public abstract reset(_?: Cache.ResetOptions): Promise<void>; public abstract restore(_: unknown): this; public abstract watch<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WatchOptions<TData, TVariables>): () => void; public abstract write<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteOptions<TData, TVariables>): Reference | undefined; public batch<U>(_: Cache.BatchOptions<this, U>): U; public declare getMemoryInternals?: typeof getApolloCacheMemoryInternals; public gc(): string[]; public identify(_: StoreObject | Reference): string | undefined; public lookupFragment(_: string): FragmentDefinitionNode | null; public modify<Entity extends Record<string, any> = Record<string, any>>(_: Cache.ModifyOptions<Entity>): boolean; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly assumeImmutableResults: boolean; public recordOptimisticTransaction(_: Transaction, _: string): ?; public resolvesClientField? (_: string, _: string): boolean; public transformDocument(_: DocumentNode): DocumentNode; public transformForLink(_: DocumentNode): DocumentNode; public updateFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.UpdateFragmentOptions<TData, TVariables>, _: (_: Unmasked<TData> | null) => Unmasked<TData> | null | void): Unmasked<TData> | null; public updateQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.UpdateQueryOptions<TData, TVariables>, _: (_: Unmasked<TData> | null) => Unmasked<TData> | null | void): Unmasked<TData> | null; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloCache.ObservableFragment<Array<Unmasked<TData>>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloCache.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloCache.ObservableFragment<Array<Unmasked<TData> | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloCache.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloCache.ObservableFragment<Unmasked<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloCache.WatchFragmentOptions<TData, TVariables>): ApolloCache.ObservableFragment<Unmasked<TData> | null>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: Cache.WriteQueryOptions<TData, TVariables>): Reference | undefined }"}`, nothing at the base matched
11. `@apollo/client "./cache":363` new, `Policies (type)`, values `{"break":1,"kind":"changed","now":"class Policies { constructor(private config: { cache: InMemoryCache; dataIdFromObject?: KeyFieldsFunction; possibleTypes?: PossibleTypesMap; typePolicies?: TypePolicies; }); public addPossibleTypes(_: PossibleTypesMap): ?; public addTypePolicies(_: TypePolicies): ?; public fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string | undefined, _?: Record<string, any>, _?: Record<string, any>): boolean; public getMergeFunction(_: string | undefined, _: string, _: string | undefined): FieldMergeFunction | undefined; public getReadFunction(_: string | undefined, _: string): FieldReadFunction | undefined; public getScalarTypeForField(_: string, _: string): ScalarType | undefined; public getStoreFieldName(_: FieldSpecifier): string; public hasKeyArgs(_: string | undefined, _: string): ?; public identify(_: StoreObject, _?: Partial<KeyFieldsContext>): [string?, StoreObject?]; public readField<V = StoreValue>(_: ReadFieldOptions, _: ReadMergeModifyContext): SafeReadonly<V> | undefined; public readonly cache: InMemoryCache; public readonly rootIdsByTypename: Record<string, string>; public readonly rootTypenamesById: Record<string, string>; public readonly usingPossibleTypes: ?; public runMergeFunction(_: StoreValue, _: StoreValue, _: MergeInfo, _: WriteContext, _?: StorageType): ? }","origin":"src/cache/inmemory/policies.ts:363","was":"class Policies { constructor(private config: { cache: InMemoryCache; dataIdFromObject?: KeyFieldsFunction; possibleTypes?: PossibleTypesMap; typePolicies?: TypePolicies; }); public addPossibleTypes(_: PossibleTypesMap): ?; public addTypePolicies(_: TypePolicies): ?; public fragmentMatches(_: InlineFragmentNode | FragmentDefinitionNode, _: string | undefined, _?: Record<string, any>, _?: Record<string, any>): boolean; public getMergeFunction(_: string | undefined, _: string, _: string | undefined): FieldMergeFunction | undefined; public getReadFunction(_: string | undefined, _: string): FieldReadFunction | undefined; public getStoreFieldName(_: FieldSpecifier): string; public hasKeyArgs(_: string | undefined, _: string): ?; public identify(_: StoreObject, _?: Partial<KeyFieldsContext>): [string?, StoreObject?]; public readField<V = StoreValue>(_: ReadFieldOptions, _: ReadMergeModifyContext): SafeReadonly<V> | undefined; public readonly cache: InMemoryCache; public readonly rootIdsByTypename: Record<string, string>; public readonly rootTypenamesById: Record<string, string>; public readonly usingPossibleTypes: ?; public runMergeFunction(_: StoreValue, _: StoreValue, _: MergeInfo, _: WriteContext, _?: StorageType): ? }"}`, nothing at the base matched
12. `@apollo/client "./core":23` new, `MutationOptions (type)`, values `{"break":1,"kind":"changed","now":"type MutationOptions<TData = unknown, TVariables extends OperationVariables = OperationVariables, TCache extends Cache.Implementation = Cache.Implementation> = ApolloClient.MutateOptions<TData, TVariables, TCache>","origin":"src/core/deprecated.ts:23","was":"type MutationOptions<TData = unknown, TVariables extends OperationVariables = OperationVariables, TCache extends ApolloCache = ApolloCache> = ApolloClient.MutateOptions<TData, TVariables, TCache>"}`, nothing at the base matched
13. `@apollo/client "./core":40` new, `RefetchQueriesOptions (type)`, values `{"break":1,"kind":"changed","now":"type RefetchQueriesOptions<TCache extends Cache.Implementation, TResult> = ApolloClient.RefetchQueriesOptions<TCache, TResult>","origin":"src/core/deprecated.ts:40","was":"type RefetchQueriesOptions<TCache extends ApolloCache, TResult> = ApolloClient.RefetchQueriesOptions<TCache, TResult>"}`, nothing at the base matched
14. `@apollo/client "./core":289` new, `InternalRefetchQueriesOptions (type)`, values `{"break":1,"kind":"changed","now":"interface InternalRefetchQueriesOptions<TCache extends Cache.Implementation, TResult> extends Omit<ApolloClient.RefetchQueriesOptions<TCache, TResult>, \"include\"> { include?: InternalRefetchQueriesInclude; removeOptimistic?: string }","origin":"src/core/types.ts:289","was":"interface InternalRefetchQueriesOptions<TCache extends ApolloCache, TResult> extends Omit<ApolloClient.RefetchQueriesOptions<TCache, TResult>, \"include\"> { include?: InternalRefetchQueriesInclude; removeOptimistic?: string }"}`, nothing at the base matched
15. `@apollo/client "./core":324` new, `ObservableQuery (type)`, values `{"break":1,"kind":"changed","now":"class ObservableQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables> implements Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>, InteropObservable<ObservableQuery.Result<MaybeMasked<TData>>> { constructor(_: { queryManager: QueryManager; options: ApolloClient.WatchQueryOptions<TData, TVariables>; transformedQuery?: DocumentNode | TypedDocumentNode<TData, TVariables>; queryId?: string; }); public applyOptions(_: Partial<ObservableQuery.Options<TData, TVariables>>): void; public async setVariables(_: TVariables): Promise<ApolloClient.QueryResult<TData>>; public fetchMore<TFetchData = TData, TFetchVars extends OperationVariables = TVariables, TErrorPolicy extends ErrorPolicy = \"none\">(_: ObservableQuery.FetchMoreOptions<TData, TVariables, TFetchData, TFetchVars> &{ errorPolicy?: TErrorPolicy; }): Promise<ApolloClient.QueryResult<TFetchData, TErrorPolicy>>; public get query(): TypedDocumentNode<TData, TVariables>; public get variables(): TVariables; public getCacheDiff(_?: ?): ?; public getCurrentResult(): ObservableQuery.Result<MaybeMasked<TData>>; public hasObservers(): ?; public notify(_?: ?): ?; public pipe!: Observable<ObservableQuery.Result<MaybeMasked<TData>>>[\"pipe\"]; public readonly options: ObservableQuery.Options<TData, TVariables>; public readonly queryName?: string; public refetch(_?: Partial<TVariables>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<TData>>; public reobserve(_?: Partial<ObservableQuery.Options<TData, TVariables>>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<MaybeMasked<TData>>>; public reset(): ?; public startPolling(_: number): ?; public stop(): ?; public stopPolling(): ?; public subscribe!: (_: | Partial<Observer<ObservableQuery.Result<MaybeMasked<TData>>>> | ((_: ObservableQuery.Result<MaybeMasked<TData>>) => void)) => Subscription; public subscribeToMore<TSubscriptionData = TData, TSubscriptionVariables extends OperationVariables = TVariables>(_: ObservableQuery.SubscribeToMoreOptions<TData, TSubscriptionVariables, TSubscriptionData, TVariables>): () => void; public updateQuery(_: UpdateQueryMapFn<TData, TVariables>): void; public[\"@@observable\"]: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>; public[Symbol.observable]!: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>> }","origin":"src/core/ObservableQuery.ts:324","was":"class ObservableQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables> implements Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>, InteropObservable<ObservableQuery.Result<MaybeMasked<TData>>> { constructor(_: { queryManager: QueryManager; options: ApolloClient.WatchQueryOptions<TData, TVariables>; transformedQuery?: DocumentNode | TypedDocumentNode<TData, TVariables>; queryId?: string; }); public _lastWrite?: unknown; public applyOptions(_: Partial<ObservableQuery.Options<TData, TVariables>>): void; public async setVariables(_: TVariables): Promise<ApolloClient.QueryResult<TData>>; public fetchMore<TFetchData = TData, TFetchVars extends OperationVariables = TVariables, TErrorPolicy extends ErrorPolicy = \"none\">(_: ObservableQuery.FetchMoreOptions<TData, TVariables, TFetchData, TFetchVars> &{ errorPolicy?: TErrorPolicy; }): Promise<ApolloClient.QueryResult<TFetchData, TErrorPolicy>>; public get query(): TypedDocumentNode<TData, TVariables>; public get variables(): TVariables; public getCacheDiff(_?: ?): ?; public getCurrentResult(): ObservableQuery.Result<MaybeMasked<TData>>; public hasObservers(): ?; public notify(_?: ?): ?; public pipe!: Observable<ObservableQuery.Result<MaybeMasked<TData>>>[\"pipe\"]; public readonly options: ObservableQuery.Options<TData, TVariables>; public readonly queryName?: string; public refetch(_?: Partial<TVariables>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<TData>>; public reobserve(_?: Partial<ObservableQuery.Options<TData, TVariables>>): ObservableQuery.ResultPromise<ApolloClient.QueryResult<MaybeMasked<TData>>>; public reset(): ?; public startPolling(_: number): ?; public stop(): ?; public stopPolling(): ?; public subscribe!: (_: | Partial<Observer<ObservableQuery.Result<MaybeMasked<TData>>>> | ((_: ObservableQuery.Result<MaybeMasked<TData>>) => void)) => Subscription; public subscribeToMore<TSubscriptionData = TData, TSubscriptionVariables extends OperationVariables = TVariables>(_: ObservableQuery.SubscribeToMoreOptions<TData, TSubscriptionVariables, TSubscriptionData, TVariables>): () => void; public updateQuery(_: UpdateQueryMapFn<TData, TVariables>): void; public[\"@@observable\"]: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>>; public[Symbol.observable]!: () => Subscribable<ObservableQuery.Result<MaybeMasked<TData>>> }"}`, nothing at the base matched
16. `@apollo/client "./core":375` new, `MutationUpdaterFunction (type)`, values `{"break":1,"kind":"changed","now":"type MutationUpdaterFunction<TData, TVariables extends OperationVariables, TCache extends Cache.Implementation> = (_: TCache, _: FormattedExecutionResult<Unmasked<TData>>, _: { context?: DefaultContext; variables: TVariables; }) => void","origin":"src/core/types.ts:375","was":"type MutationUpdaterFunction<TData, TVariables extends OperationVariables, TCache extends ApolloCache> = (_: TCache, _: FormattedExecutionResult<Unmasked<TData>>, _: { context?: DefaultContext; variables?: TVariables; }) => void"}`, nothing at the base matched
17. `@apollo/client "./core":1035` new, `ApolloClient (type)`, values `{"break":1,"kind":"changed","now":"class ApolloClient { constructor(_: ApolloClient.Options); get documentTransform(): ?; get localState(): LocalState | undefined; public __actionHookForDevTools(_: () => any): ?; public __requestRaw(_: ApolloLink.Request): Observable<ApolloLink.Result<unknown>>; public cache: Cache.Implementation; public clearStore(): Promise<any[]>; public declare getMemoryInternals?: typeof getApolloClientMemoryInternals; public defaultOptions: ApolloClient.DefaultOptions; public disableNetworkFetches!: never; public extract(_?: boolean): ?; public get defaultContext(): ?; public get prioritizeCacheValues(): ?; public getObservableQueries(_?: RefetchQueriesInclude): Set<ObservableQuery<any>>; public link: ApolloLink; public mutate: ApolloClient.mutate.Signature; public onClearStore(_: () => Promise<any>): () => void; public onResetStore(_: () => Promise<any>): () => void; public query: ApolloClient.query.Signature; public queryDeduplication: boolean; public reFetchObservableQueries: (_?: boolean) => Promise<ApolloClient.QueryResult<any>[]>; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly devtoolsConfig: ApolloClient.DevtoolsOptions; public readonly refetchEventManager: RefetchEventManager | undefined; public refetchObservableQueries(_?: boolean): Promise<ApolloClient.QueryResult<any>[]>; public refetchQueries<TCache extends Cache.Implementation = Cache.Implementation, TResult = Promise<ApolloClient.QueryResult<any>>>(_: ApolloClient.RefetchQueriesOptions<TCache, TResult>): ApolloClient.RefetchQueriesResult<TResult>; public resetStore(): Promise<ApolloClient.QueryResult<any>[] | null>; public restore(_: unknown): ?; public set prioritizeCacheValues(_: boolean): ?; public setLink(_: ApolloLink): ?; public stop(): ?; public subscribe<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.SubscribeOptions<TData, TVariables>): SubscriptionObservable<ApolloClient.SubscribeResult<MaybeMasked<TData>>>; public version: string; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloClient.ObservableFragment<Array<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloClient.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloClient.ObservableFragment<Array<TData | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloClient.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloClient.ObservableFragment<TData>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables>): ApolloClient.ObservableFragment<TData | null>; public watchQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchQueryOptions<TData, TVariables>): ObservableQuery<TData, TVariables>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteQueryOptions<TData, TVariables>): Reference | undefined; set localState(_: LocalState): ? }","origin":"src/core/ApolloClient.ts:1035","was":"class ApolloClient { constructor(_: ApolloClient.Options); get documentTransform(): ?; get localState(): LocalState | undefined; public __actionHookForDevTools(_: () => any): ?; public __requestRaw(_: ApolloLink.Request): Observable<ApolloLink.Result<unknown>>; public cache: ApolloCache; public clearStore(): Promise<any[]>; public declare getMemoryInternals?: typeof getApolloClientMemoryInternals; public defaultOptions: ApolloClient.DefaultOptions; public disableNetworkFetches!: never; public extract(_?: boolean): ?; public get defaultContext(): ?; public get prioritizeCacheValues(): ?; public getObservableQueries(_?: RefetchQueriesInclude): Set<ObservableQuery<any>>; public link: ApolloLink; public mutate: ApolloClient.mutate.Signature; public onClearStore(_: () => Promise<any>): () => void; public onResetStore(_: () => Promise<any>): () => void; public query: ApolloClient.query.Signature; public queryDeduplication: boolean; public reFetchObservableQueries: (_?: boolean) => Promise<ApolloClient.QueryResult<any>[]>; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>): Unmasked<TData> | null; public readFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadFragmentOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>): Unmasked<TData> | null; public readQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.ReadQueryOptions<TData, TVariables>, _: boolean): Unmasked<TData> | null; public readonly devtoolsConfig: ApolloClient.DevtoolsOptions; public readonly refetchEventManager: RefetchEventManager | undefined; public refetchObservableQueries(_?: boolean): Promise<ApolloClient.QueryResult<any>[]>; public refetchQueries<TCache extends ApolloCache = ApolloCache, TResult = Promise<ApolloClient.QueryResult<any>>>(_: ApolloClient.RefetchQueriesOptions<TCache, TResult>): ApolloClient.RefetchQueriesResult<TResult>; public resetStore(): Promise<ApolloClient.QueryResult<any>[] | null>; public restore(_: unknown): ?; public set prioritizeCacheValues(_: boolean): ?; public setLink(_: ApolloLink): ?; public stop(): ?; public subscribe<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.SubscribeOptions<TData, TVariables>): SubscriptionObservable<ApolloClient.SubscribeResult<MaybeMasked<TData>>>; public version: string; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData>>; }): ApolloClient.ObservableFragment<Array<TData>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<null>; }): ApolloClient.ObservableFragment<Array<null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: Array<ApolloCache.FromOptionValue<TData> | null>; }): ApolloClient.ObservableFragment<Array<TData | null>>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: null; }): ApolloClient.ObservableFragment<null>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables> &{ from: ApolloCache.FromOptionValue<TData>; }): ApolloClient.ObservableFragment<TData>; public watchFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchFragmentOptions<TData, TVariables>): ApolloClient.ObservableFragment<TData | null>; public watchQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WatchQueryOptions<TData, TVariables>): ObservableQuery<TData, TVariables>; public writeFragment<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteFragmentOptions<TData, TVariables>): Reference | undefined; public writeQuery<TData = unknown, TVariables extends OperationVariables = OperationVariables>(_: ApolloClient.WriteQueryOptions<TData, TVariables>): Reference | undefined; set localState(_: LocalState): ? }"}`, nothing at the base matched
18. `@apollo/client "./incremental":166` new, `Defer20220824Handler (type)`, values `{"break":1,"kind":"changed","now":"class Defer20220824Handler implements Incremental.Handler<Defer20220824Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: Record<string, any>): result is | Defer20220824Handler.SubsequentResult | Defer20220824Handler.InitialResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; startRequest<TData extends Record<string, unknown>>(_: Incremental.StartRequestOptions): ? }","origin":"src/incremental/handlers/defer20220824.ts:166","was":"class Defer20220824Handler implements Incremental.Handler<Defer20220824Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: Record<string, any>): result is | Defer20220824Handler.SubsequentResult | Defer20220824Handler.InitialResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; startRequest<TData extends Record<string, unknown>>(_: { query: DocumentNode; }): ? }"}`, nothing at the base matched
19. `@apollo/client "./incremental":166` new, `GraphQL17Alpha2Handler (type)`, values `{"break":1,"kind":"changed","now":"class Defer20220824Handler implements Incremental.Handler<Defer20220824Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: Record<string, any>): result is | Defer20220824Handler.SubsequentResult | Defer20220824Handler.InitialResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; startRequest<TData extends Record<string, unknown>>(_: Incremental.StartRequestOptions): ? }","origin":"src/incremental/handlers/defer20220824.ts:166","was":"class Defer20220824Handler implements Incremental.Handler<Defer20220824Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: Record<string, any>): result is | Defer20220824Handler.SubsequentResult | Defer20220824Handler.InitialResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; startRequest<TData extends Record<string, unknown>>(_: { query: DocumentNode; }): ? }"}`, nothing at the base matched
20. `@apollo/client "./incremental":311` new, `GraphQL17Alpha9Handler (type)`, values `{"break":1,"kind":"changed","now":"class GraphQL17Alpha9Handler implements Incremental.Handler<GraphQL17Alpha9Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: ApolloLink.Result<any>): result is | GraphQL17Alpha9Handler.InitialResult | GraphQL17Alpha9Handler.SubsequentResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; readonly documentTransform: ?; startRequest<TData>(_: Incremental.StartRequestOptions): ? }","origin":"src/incremental/handlers/graphql17Alpha9.ts:311","was":"class GraphQL17Alpha9Handler implements Incremental.Handler<GraphQL17Alpha9Handler.Chunk<any>> { extractErrors(_: ApolloLink.Result<any>): ?; isIncrementalResult(_: ApolloLink.Result<any>): result is | GraphQL17Alpha9Handler.InitialResult | GraphQL17Alpha9Handler.SubsequentResult; prepareRequest(_: ApolloLink.Request): ApolloLink.Request; startRequest<TData>(_: { query: DocumentNode }): ? }"}`, nothing at the base matched
21. `@apollo/client "./utilities":27` new, `CacheSizes (type)`, values `{"break":1,"kind":"changed","now":"interface CacheSizes { \"PersistedQueryLink.persistedQueryHashes\": number; \"cache.fragmentQueryDocuments\": number; \"documentTransform.cache\": number; \"fragmentRegistry.findFragmentSpreads\": number; \"fragmentRegistry.lookup\": number; \"fragmentRegistry.transform\": number; \"inMemoryCache.executeSelectionSet\": number; \"inMemoryCache.executeSubSelectedArray\": number; \"inMemoryCache.maybeBroadcastWatch\": number; \"inMemoryCache.prunePartialBoundaries\": number; \"inMemoryCache.prunePartialStreamArray\": number; \"queryManager.getDocumentInfo\": number; \"removeTypenameFromVariables.getVariableDefinitions\": number; canonicalStringify: number; checkDocument: number; isDeferredFragment: number; isStreamField: number; print: number }","origin":"src/utilities/caching/sizes.ts:27","was":"interface CacheSizes { \"PersistedQueryLink.persistedQueryHashes\": number; \"cache.fragmentQueryDocuments\": number; \"documentTransform.cache\": number; \"fragmentRegistry.findFragmentSpreads\": number; \"fragmentRegistry.lookup\": number; \"fragmentRegistry.transform\": number; \"inMemoryCache.executeSelectionSet\": number; \"inMemoryCache.executeSubSelectedArray\": number; \"inMemoryCache.maybeBroadcastWatch\": number; \"queryManager.getDocumentInfo\": number; \"removeTypenameFromVariables.getVariableDefinitions\": number; canonicalStringify: number; checkDocument: number; print: number }"}`, nothing at the base matched
22. `src/cache/core/Scalar.ts:3` unresolved, `export declare namespace Scalar { — @apollo/client "./cache" — an export form klin does not list: 'export =', a namespace, or an ambient module`
23. `src/cache/inmemory/inMemoryCache.ts:67` unresolved, `export declare namespace InMemoryCache { — @apollo/client "./cache" — an export form klin does not list: 'export =', a namespace, or an ambient module`
24. `src/core/types.ts:29` unresolved, `export declare namespace OverridableTypes { — @apollo/client "." — an export form klin does not list: 'export =', a namespace, or an ambient module`
25. `src/core/types.ts:29` unresolved, `export declare namespace OverridableTypes { — @apollo/client "./core" — an export form klin does not list: 'export =', a namespace, or an ambient module`
26. `src/incremental/GraphQLCodegenIncremental.ts:9` unresolved, `export declare namespace GraphQLCodegenIncremental { — @apollo/client "./incremental" — an export form klin does not list: 'export =', a namespace, or an ambien`

### Remedy klin printed

> Keep the surface, the item or the declared contract the base had where the task allows it. For a changed contract, a new item beside the unchanged one keeps the base's contract where that serves the task. Do not change what the task asked for only to satisfy this gate. If the break is intended, a person accepts it with an accepted entry in a reviewed commit, and until then CI refuses it.

## R058

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 2 of 10
- Commit: `391e72d780c2`, judged against its first parent `87df76a2ce58`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Onto is live on Product Hunt: corner card and README line
>
> The OpenStock team launched Onto today. A card in the corner of every page
> and a line at the top of the README ask for an upvote. The card closes for
> the current page only and stops on 3 Oct.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 README.md                     |  4 ++++
 app/layout.tsx                |  2 ++
 components/OntoLaunchCard.tsx | 53 +++++++++++++++++++++++++++++++++++++++++++++++++++++
 3 files changed, 59 insertions(+)
```

### Findings

Condition: where the code opts out of a check.

1. `components/OntoLaunchCard.tsx:35` new, `{/* eslint-disable-next-line @next/next/no-img-element -- Onto's mark, served from its own site */}`, values `{"count":1,"escape":"eslint-disable"}`, nothing at the base matched

   ```text
   32 |                 ×
   33 |             </button>
   34 |             <p className="m-0 mb-2 flex items-center gap-2 font-mono text-[10.5px] uppercase tracking-[0.12em] text-[var(--muted)]">
   35 |                 {/* eslint-disable-next-line @next/next/no-img-element -- Onto's mark, served from its own site */}
   36 |                 <img src="https://buildonto.dev/icon.png" alt="" width={16} height={16} className="h-4 w-4" />
   37 |                 From the OpenStock team
   38 |             </p>
   39 |             <p className="m-0 text-base font-semibold leading-snug text-[var(--text)]">We just launched Onto on Product Hunt.</p>
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R059

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 3 of 10
- Commit: `87df76a2ce58`, judged against its first parent `130a69734d26`, exit 1
- Gate: doc-size, FAIL
- Decision group: document README.md
- Derived API_DOCS.md: `450`, the word count at the derivation commit, rounded up to the next 50
- Derived MARKET_SUPPORT.md: `400`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `2550`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> docs(sponsor): explain how OpenStock is funded; show 13K+ users
>
> - README: a call-out under the intro and a rewritten Sponsor section.
>   It covers who uses OpenStock, the funding plan (OpenStock Cloud will
>   pay for hosting; sponsors fund reviews, fixes and features), tiers
>   with GitHub Sponsors links that preselect the amount, a one-time
>   option, the contact route and who receives the money.
> - Sponsor page: 13,000+ registered users in the hero and stats card
>   and in the Partner perk. A "Cloud pays for hosting, sponsors keep
>   the community moving" pair of cards under where the money goes.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 README.md                        | 29 ++++++++++++++++++++++++-----
 app/(marketing)/sponsor/page.tsx | 22 +++++++++++++++++-----
 lib/sponsors.ts                  |  5 ++++-
 3 files changed, 45 insertions(+), 11 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `README.md` new, values `{"ceiling":2550,"words":2765}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R060

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 3 of 10
- Commit: `87df76a2ce58`, judged against its first parent `130a69734d26`, exit 1
- Gate: complexity, FAIL
- Decision group: lines at a derived percentile, the base site already over
- Derived cc: `6`, 95th percentile of 636 functions at 130a697, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 636 functions at 130a697, floor 25; recorded scope: whole repository

### Commit message

> docs(sponsor): explain how OpenStock is funded; show 13K+ users
>
> - README: a call-out under the intro and a rewritten Sponsor section.
>   It covers who uses OpenStock, the funding plan (OpenStock Cloud will
>   pay for hosting; sponsors fund reviews, fixes and features), tiers
>   with GitHub Sponsors links that preselect the amount, a one-time
>   option, the contact route and who receives the money.
> - Sponsor page: 13,000+ registered users in the hero and stats card
>   and in the Partner perk. A "Cloud pays for hosting, sponsors keep
>   the community moving" pair of cards under where the money goes.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 README.md                        | 29 ++++++++++++++++++++++++-----
 app/(marketing)/sponsor/page.tsx | 22 +++++++++++++++++-----
 lib/sponsors.ts                  |  5 ++++-
 3 files changed, 45 insertions(+), 11 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 6 or body > 53 lines).

1. `app/(marketing)/sponsor/page.tsx:32` worsened, `export default async function SponsorPage() {`, values `{"cc":5,"lines":202}`, ceiling cc 6, lines 53, base site `app/(marketing)/sponsor/page.tsx:31` with `{"cc":5,"lines":191}`

   ```text
   29 |     ['Can we sponsor as a company?', 'Yes. Sponsor from your organisation’s GitHub account, then send us your logo and link. For invoices or a custom deal, talk with us.'],
   30 | ];
   31 | 
   32 | export default async function SponsorPage() {
   33 |     const repo = await getRepoStats();
   34 |     const goalPercent = Math.min(100, (SPONSOR_GOAL.current / SPONSOR_GOAL.target) * 100);
   35 |     const fundingTotal = FUNDING_USES.reduce((sum, use) => sum + (use.monthly ?? 0), 0);
   36 | 
   37 |     return (
   38 |         <div className="mx-auto max-w-[1200px] px-5">
   39 |             <section className="grid items-end gap-10 pt-14 md:pt-20 lg:grid-cols-[minmax(0,1fr)_380px]">
   40 |                 <div>
   41 |                     <p className="kicker flex items-center gap-2 text-brand-ink"><span className="live-dot" /> Open for sponsors</p>
   42 |                     <h1 className="mt-4 text-[44px] font-bold leading-[1.02] tracking-[-0.05em] md:text-[64px]">Back free<br />market data.</h1>
   43 |                     <p className="mt-5 max-w-xl text-[17px] leading-relaxed text-muted-foreground">
   44 |                         OpenStock is an open-source market terminal used by {REGISTERED_USERS} registered people{repo ? `, with ${formatCount(repo.stars)} stars on GitHub` : ''}.
   45 |                         Sponsors keep it free and independent for everyone who can’t pay for a terminal.
   46 |                     </p>
   47 |                     <div className="mt-8 flex flex-wrap gap-2">
   48 |                         <a href="#tiers" className="btn btn-primary h-11 px-5 text-[15px]">See tiers</a>
   49 |                         <a href={MAILTO} className="btn btn-ghost h-11 px-5 text-[15px]">Talk with us</a>
   50 |                     </div>
   51 |                 </div>
   52 | 
   53 |                 <div className="hatch">
   54 |                     <div className="card flex flex-col gap-4 p-5">
   55 |                         <div className="flex items-baseline justify-between gap-3">
   56 |                             <p className="kicker">Goal</p>
   57 |                             <p className="num text-[13px] text-faint">{SPONSOR_GOAL.current} of {SPONSOR_GOAL.target.toLocaleString('en-US')}</p>
   58 |                         </div>
   59 |                         <p className="text-[20px] font-bold tracking-[-0.02em]">{SPONSOR_GOAL.target.toLocaleString('en-US')} {SPONSOR_GOAL.label}</p>
   60 |                         <div className="h-2.5 overflow-hidden rounded-full bg-page shadow-[inset_0_0_0_1px_var(--line)]" role="progressbar" aria-valuenow={SPONSOR_GOAL.current} aria-valuemax={SPONSOR_GOAL.target}>
   61 |                             <div className="h-full rounded-full bg-brand" style={{ width: `max(${goalPercent}%, 6px)` }} />
   62 |                         </div>
   63 |                         {repo && (
   64 |                             <dl className="grid grid-cols-3 gap-2">
   65 |                                 {[['Users', REGISTERED_USERS.replace(',000', 'K')], ['Stars', formatCount(repo.stars)], ['Forks', formatCount(repo.forks)]].map(([label, value]) => (
   66 |                                     <div key={label} className="rounded-[12px] bg-page px-3.5 py-3 shadow-[inset_0_0_0_1px_var(--line)]">
   67 |                                         <dt className="kicker">{label}</dt>
   68 |                                         <dd className="bento-value mt-1 text-[22px]">{value}</dd>
   69 |                                     </div>
   70 |                                 ))}
   71 |                             </dl>
   72 |                         )}
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R061

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 4 of 10
- Commit: `130a69734d26`, judged against its first parent `e844ac413c74`, exit 1
- Gate: doc-size, FAIL
- Decision group: document MARKET_SUPPORT.md
- Derived API_DOCS.md: `450`, the word count at the derivation commit, rounded up to the next 50
- Derived MARKET_SUPPORT.md: `350`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `2550`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> fix(stocks): no more stuck charts for blocked exchanges or Hong Kong
>
> Some stock pages showed widgets that never loaded. I tested every
> mapped exchange in a real-browser session:
>
> - Hong Kong symbols broke in every widget: Finnhub pads codes
>   (0700.HK) but TradingView wants HKEX:700. The zero padding is now
>   stripped.
> - TradingView refuses the candle chart in free embeds for 13
>   exchanges (Tokyo, Hong Kong, London, Korea, Taiwan, Singapore, New
>   Zealand, Thailand, Malaysia, Istanbul, TSX Venture, Mexico,
>   Johannesburg), each confirmed with two tickers. Those pages now say
>   so and link to the chart on TradingView. Financials, technicals and
>   profile still load.
> - Prague mapped to PSE, which is the Philippine exchange on
>   TradingView; it is now PSECZ.
>
> Every other widget loaded in repeated loads and client navigations.
> A headless browser gets no chart data at all (TradingView blocks the
> HeadlessChrome user agent), so test with a normal user agent.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 MARKET_SUPPORT.md                   |  7 ++++---
 __tests__/utils.test.ts             | 18 +++++++++++++++++-
 app/(root)/stocks/[symbol]/page.tsx | 35 ++++++++++++++++++++++++++---------
 lib/markets.ts                      |  3 ++-
 lib/utils.ts                        | 14 ++++++++++++--
 5 files changed, 61 insertions(+), 16 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `MARKET_SUPPORT.md` new, values `{"ceiling":350,"words":355}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R062

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 4 of 10
- Commit: `130a69734d26`, judged against its first parent `e844ac413c74`, exit 1
- Gate: complexity, FAIL
- Decision group: lines at a derived percentile
- Derived cc: `6`, 95th percentile of 631 functions at e844ac4, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 631 functions at e844ac4, floor 25; recorded scope: whole repository

### Commit message

> fix(stocks): no more stuck charts for blocked exchanges or Hong Kong
>
> Some stock pages showed widgets that never loaded. I tested every
> mapped exchange in a real-browser session:
>
> - Hong Kong symbols broke in every widget: Finnhub pads codes
>   (0700.HK) but TradingView wants HKEX:700. The zero padding is now
>   stripped.
> - TradingView refuses the candle chart in free embeds for 13
>   exchanges (Tokyo, Hong Kong, London, Korea, Taiwan, Singapore, New
>   Zealand, Thailand, Malaysia, Istanbul, TSX Venture, Mexico,
>   Johannesburg), each confirmed with two tickers. Those pages now say
>   so and link to the chart on TradingView. Financials, technicals and
>   profile still load.
> - Prague mapped to PSE, which is the Philippine exchange on
>   TradingView; it is now PSECZ.
>
> Every other widget loaded in repeated loads and client navigations.
> A headless browser gets no chart data at all (TradingView blocks the
> HeadlessChrome user agent), so test with a normal user agent.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 MARKET_SUPPORT.md                   |  7 ++++---
 __tests__/utils.test.ts             | 18 +++++++++++++++++-
 app/(root)/stocks/[symbol]/page.tsx | 35 ++++++++++++++++++++++++++---------
 lib/markets.ts                      |  3 ++-
 lib/utils.ts                        | 14 ++++++++++++--
 5 files changed, 61 insertions(+), 16 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 6 or body > 53 lines).

1. `app/(root)/stocks/[symbol]/page.tsx:29` new, `export default async function StockDetails({ params }: StockDetailsPageProps) {`, values `{"cc":2,"lines":69}`, ceiling cc 6, lines 53, nothing at the base matched

   ```text
   26 | }
   27 | 
   28 | // Charts come straight from TradingView (live, no quota); only the header waits on Finnhub, and it streams.
   29 | export default async function StockDetails({ params }: StockDetailsPageProps) {
   30 |     const { symbol: rawSymbol } = await params;
   31 |     const symbol = decodeURIComponent(rawSymbol).toUpperCase();
   32 |     const tvSymbol = formatSymbolForTradingView(symbol);
   33 | 
   34 |     return (
   35 |         <>
   36 |             <Suspense fallback={<StockHeaderSkeleton />}>
   37 |                 <StockHeader symbol={symbol} />
   38 |             </Suspense>
   39 | 
   40 |             <div className="grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(320px,400px)]">
   41 |                 <div className="flex min-w-0 flex-col gap-3">
   42 |                     {isChartEmbeddable(tvSymbol) ? (
   43 |                         <Panel title="Chart" sub="Live from TradingView · switch interval and style in the chart">
   44 |                             <TradingViewWidget
   45 |                                 scriptUrl={`${scriptUrl}advanced-chart.js`}
   46 |                                 config={CANDLE_CHART_WIDGET_CONFIG(tvSymbol)}
   47 |                                 height={560}
   48 |                                 allowExpand
   49 |                             />
   50 |                         </Panel>
   51 |                     ) : (
   52 |                         // TradingView refuses this exchange's chart in embeds; say so instead of showing its error
   53 |                         <Panel title="Chart" sub="Only on TradingView for this exchange">
   54 |                             <div className="empty-state py-14">
   55 |                                 <span className="empty-icon"><CandlestickChart className="size-5" /></span>
   56 |                                 <h3>Chart not available here</h3>
   57 |                                 <p className="max-w-80 text-[13px]">
   58 |                                     TradingView doesn’t license {tvSymbol.split(':')[0]} charts for other sites. Financials, technicals and the profile below still work.
   59 |                                 </p>
   60 |                                 <a href={tradingViewSymbolUrl(tvSymbol)} target="_blank" rel="noopener noreferrer" className="btn btn-ghost mt-2">
   61 |                                     Open chart on TradingView
   62 |                                 </a>
   63 |                             </div>
   64 |                         </Panel>
   65 |                     )}
   66 |                     <Panel title="Financials" sub="Income statement, balance sheet and ratios">
   67 |                         <TradingViewWidget
   68 |                             scriptUrl={`${scriptUrl}financials.js`}
   69 |                             config={COMPANY_FINANCIALS_WIDGET_CONFIG(tvSymbol)}
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R063

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 5 of 10
- Commit: `e844ac413c74`, judged against its first parent `1819bdf8a584`, exit 1
- Gate: doc-size, FAIL
- Decision group: document README.md
- Derived API_DOCS.md: `450`, the word count at the derivation commit, rounded up to the next 50
- Derived MARKET_SUPPORT.md: `350`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `2500`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> feat(alerts): make price alerts an OpenStock Cloud feature
>
> Email price alerts now come with OpenStock Cloud ($5/month, coming
> soon) and with self-hosted instances in realtime mode. The free hourly
> site keeps charts, watchlists and research.
>
> - createAlert refuses new alerts unless alertsEnabled (realtime mode).
>   The check is on the server, not just the UI.
> - The alert checker job is only registered where alerts are on.
> - The alert dialog (stock page button and watchlist bell) shows a
>   Cloud card instead of the form. The Alerts panel says it's a Cloud
>   feature. Existing alerts are listed as paused and can still be
>   deleted; nothing is removed from the database.
> - Landing, About, Help, API docs, README and the welcome email no
>   longer promise free alerts.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 README.md                                 |   8 +++--
 __tests__/alert-actions.test.ts           |  13 ++++++++
 app/(marketing)/about/page.tsx            |   2 +-
 app/(marketing)/api-docs/page.tsx         |   4 +--
 app/(marketing)/help/page.tsx             |   4 +--
 app/(marketing)/page.tsx                  |   8 ++---
 app/(root)/profile/page.tsx               |   3 +-
 app/(root)/watchlist/page.tsx             |   3 +-
 app/api/inngest/route.ts                  |   4 ++-
 components/watchlist/AlertsPanel.tsx      |  75 +++++++++++++++++++++++++++---------------
 components/watchlist/CreateAlertModal.tsx | 125 ++++++++++++++++++++++++++++++++++++++++------------------------------
 lib/actions/alert.actions.ts              |   2 ++
 lib/market-data.ts                        |   3 ++
 lib/nodemailer/templates.ts               |   2 +-
 14 files changed, 160 insertions(+), 96 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `README.md` new, values `{"ceiling":2500,"words":2505}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R064

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 5 of 10
- Commit: `e844ac413c74`, judged against its first parent `1819bdf8a584`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `6`, 95th percentile of 628 functions at 1819bdf, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 628 functions at 1819bdf, floor 25; recorded scope: whole repository

### Commit message

> feat(alerts): make price alerts an OpenStock Cloud feature
>
> Email price alerts now come with OpenStock Cloud ($5/month, coming
> soon) and with self-hosted instances in realtime mode. The free hourly
> site keeps charts, watchlists and research.
>
> - createAlert refuses new alerts unless alertsEnabled (realtime mode).
>   The check is on the server, not just the UI.
> - The alert checker job is only registered where alerts are on.
> - The alert dialog (stock page button and watchlist bell) shows a
>   Cloud card instead of the form. The Alerts panel says it's a Cloud
>   feature. Existing alerts are listed as paused and can still be
>   deleted; nothing is removed from the database.
> - Landing, About, Help, API docs, README and the welcome email no
>   longer promise free alerts.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 README.md                                 |   8 +++--
 __tests__/alert-actions.test.ts           |  13 ++++++++
 app/(marketing)/about/page.tsx            |   2 +-
 app/(marketing)/api-docs/page.tsx         |   4 +--
 app/(marketing)/help/page.tsx             |   4 +--
 app/(marketing)/page.tsx                  |   8 ++---
 app/(root)/profile/page.tsx               |   3 +-
 app/(root)/watchlist/page.tsx             |   3 +-
 app/api/inngest/route.ts                  |   4 ++-
 components/watchlist/AlertsPanel.tsx      |  75 +++++++++++++++++++++++++++---------------
 components/watchlist/CreateAlertModal.tsx | 125 ++++++++++++++++++++++++++++++++++++++++------------------------------
 lib/actions/alert.actions.ts              |   2 ++
 lib/market-data.ts                        |   3 ++
 lib/nodemailer/templates.ts               |   2 +-
 14 files changed, 160 insertions(+), 96 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 6 or body > 53 lines).

1. `app/(root)/profile/page.tsx:20` worsened, `export default async function ProfilePage() {`, values `{"cc":9,"lines":78}`, ceiling cc 6, lines 53, base site `app/(root)/profile/page.tsx:19` with `{"cc":8,"lines":78}`

   ```text
   17 |     github: 'GitHub',
   18 | };
   19 | 
   20 | export default async function ProfilePage() {
   21 |     const session = await getSession();
   22 |     if (!session?.user) redirect('/sign-in');
   23 |     const { user } = session;
   24 | 
   25 |     const [accounts, watchlist, alerts] = await Promise.all([
   26 |         auth.api.listUserAccounts({ headers: await headers() }).catch(() => []),
   27 |         getUserWatchlist(),
   28 |         getUserAlerts(),
   29 |     ]);
   30 |     const linked = new Set(accounts.map((a: { providerId: string }) => a.providerId));
   31 |     const activeAlerts = alerts.filter((a: { triggered?: boolean }) => !a.triggered).length;
   32 |     const memberSince = new Date(user.createdAt).toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
   33 | 
   34 |     const stats = [
   35 |         { label: 'Watching', value: watchlist.length, icon: Star },
   36 |         { label: alertsEnabled ? 'Active alerts' : 'Paused alerts', value: activeAlerts, icon: Bell },
   37 |         { label: 'Sign-in methods', value: linked.size, icon: ShieldCheck },
   38 |     ];
   39 | 
   40 |     return (
   41 |         <>
   42 |             <section className="hatch">
   43 |                 <div className="card flex flex-wrap items-center gap-4 p-5">
   44 |                     <span className="grid size-14 flex-none place-items-center rounded-full bg-brand-soft text-[22px] font-bold text-brand-ink">
   45 |                         {user.name?.[0]?.toUpperCase() ?? '?'}
   46 |                     </span>
   47 |                     <div className="min-w-0 flex-1">
   48 |                         <h1 className="truncate text-[22px] font-bold tracking-[-0.03em]">{user.name}</h1>
   49 |                         <p className="truncate text-faint">{user.email} · member since {memberSince}</p>
   50 |                     </div>
   51 |                 </div>
   52 |                 <div className="bento mt-[3px]">
   53 |                     {stats.map(({ label, value, icon: Icon }) => (
   54 |                         <div key={label} className="bento-tile col-span-4 gap-2.5 max-lg:col-span-12">
   55 |                             <div className="bento-head"><span className="bento-ico"><Icon /></span>{label}</div>
   56 |                             <p className="bento-value text-[26px]">{value}</p>
   57 |                         </div>
   58 |                     ))}
   59 |                 </div>
   60 |             </section>
   ```

2. `components/watchlist/AlertsPanel.tsx:30` worsened, `export default function AlertsPanel({ alerts }: { alerts: AlertRow[] }) {`, values `{"cc":5,"lines":75}`, ceiling cc 6, lines 53, base site `components/watchlist/AlertsPanel.tsx:27` with `{"cc":2,"lines":57}`

   ```text
   27 |     return { label: 'Watching', className: 'pill is-up' };
   28 | };
   29 | 
   30 | export default function AlertsPanel({ alerts }: { alerts: AlertRow[] }) {
   31 |     const router = useRouter();
   32 |     const [pendingId, setPendingId] = useState<string | null>(null);
   33 | 
   34 |     const handleDelete = async (alert: AlertRow) => {
   35 |         setPendingId(alert._id);
   36 |         try {
   37 |             await deleteAlert(alert._id);
   38 |             toast.success(`Alert for ${alert.symbol} removed`);
   39 |             router.refresh();
   40 |         } catch {
   41 |             toast.error("Couldn’t remove the alert");
   42 |         } finally {
   43 |             setPendingId(null);
   44 |         }
   45 |     };
   46 | 
   47 |     if (alerts.length === 0 && !alertsEnabled) {
   48 |         return (
   49 |             <div className="empty-state py-10">
   50 |                 <span className="empty-icon"><BellRing className="size-5" /></span>
   51 |                 <h3>Alerts come with Cloud</h3>
   52 |                 <p className="max-w-64 text-[13px]">Email price alerts are part of OpenStock Cloud, $5 a month and coming soon.</p>
   53 |                 <Link href="/#data" className="btn btn-ghost mt-2">See OpenStock Cloud</Link>
   54 |             </div>
   55 |         );
   56 |     }
   57 | 
   58 |     if (alerts.length === 0) {
   59 |         return (
   60 |             <div className="empty-state py-10">
   61 |                 <span className="empty-icon"><BellRing className="size-5" /></span>
   62 |                 <h3>No alerts yet</h3>
   63 |                 <p className="max-w-60 text-[13px]">Use the bell on any row, or “Set alert” on a stock page. We email you when it fires.</p>
   64 |             </div>
   65 |         );
   66 |     }
   67 | 
   68 |     return (
   69 |         <>
   70 |             {!alertsEnabled && (
   ```

3. `components/watchlist/CreateAlertModal.tsx:25` worsened, `export default function CreateAlertModal({ symbol, currentPrice, currency = 'USD', children, onAlertCreated }: CreateAlertModalProps) {`, values `{"cc":16,"lines":113}`, ceiling cc 6, lines 53, base site `components/watchlist/CreateAlertModal.tsx:23` with `{"cc":15,"lines":96}`

   ```text
   22 |     { value: 'BELOW', label: 'Falls below' },
   23 | ] as const;
   24 | 
   25 | export default function CreateAlertModal({ symbol, currentPrice, currency = 'USD', children, onAlertCreated }: CreateAlertModalProps) {
   26 |     const [open, setOpen] = useState(false);
   27 |     const [condition, setCondition] = useState<'ABOVE' | 'BELOW'>('ABOVE');
   28 |     const [targetPrice, setTargetPrice] = useState('');
   29 |     const [loading, setLoading] = useState(false);
   30 | 
   31 |     const target = parseFloat(targetPrice);
   32 |     const hasPrice = !!currentPrice && currentPrice > 0;
   33 |     const distance = hasPrice && Number.isFinite(target) ? ((target - currentPrice!) / currentPrice!) * 100 : null;
   34 |     const alreadyMet = distance !== null && (condition === 'ABOVE' ? distance <= 0 : distance >= 0);
   35 | 
   36 |     const onOpenChange = (next: boolean) => {
   37 |         setOpen(next);
   38 |         if (next) setTargetPrice(hasPrice ? currentPrice!.toFixed(2) : '');
   39 |     };
   40 | 
   41 |     const handleSubmit = async (e: React.FormEvent) => {
   42 |         e.preventDefault();
   43 |         setLoading(true);
   44 |         try {
   45 |             await createAlert({ symbol, targetPrice: target, condition });
   46 |             toast.success(`Alert set: ${symbol} ${condition === 'ABOVE' ? 'above' : 'below'} ${formatPrice(target, currency)}`);
   47 |             setOpen(false);
   48 |             onAlertCreated?.();
   49 |         } catch (error) {
   50 |             console.error(error);
   51 |             toast.error("Couldn’t create the alert");
   52 |         } finally {
   53 |             setLoading(false);
   54 |         }
   55 |     };
   56 | 
   57 |     return (
   58 |         <Dialog open={open} onOpenChange={onOpenChange}>
   59 |             {children && <DialogTrigger asChild>{children}</DialogTrigger>}
   60 |             <DialogContent className="sm:max-w-[420px] gap-5">
   61 |                 {alertsEnabled ? (
   62 |                     <>
   63 |                         <div>
   64 |                             <p className="kicker text-brand-ink">Price alert</p>
   65 |                             <DialogTitle className="mt-1 text-xl font-bold tracking-tight mono">{symbol}</DialogTitle>
   ```

4. `lib/actions/alert.actions.ts:16` worsened, `export async function createAlert(params: {`, values `{"cc":9,"lines":28}`, ceiling cc 6, lines 53, base site `lib/actions/alert.actions.ts:15` with `{"cc":8,"lines":27}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R065

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 8 of 10
- Commit: `75a9ebb6f597`, judged against its first parent `3cf500ad4c82`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over
- Derived cc: `6`, 95th percentile of 628 functions at 3cf500a, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 628 functions at 3cf500a, floor 25; recorded scope: whole repository

### Commit message

> fix(dashboard): TradingView widgets collapsed to 150px
>
> TradingView's autosize sets the container's inline height to 100%,
> overwriting the 560px height React set on it. The parent had no
> height, so every iframe fell back to 150px. At that size Leaders
> showed "No data here yet" and Quotes showed empty columns. The height
> now lives on the frame, which TradingView never touches.
>
> Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

### Files the change touched

```text
 components/TradingViewWidget.tsx | 8 ++++++--
 1 file changed, 6 insertions(+), 2 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 6 or body > 53 lines).

1. `components/TradingViewWidget.tsx:17` worsened, `const TradingViewWidget = ({ scriptUrl, config, height = 600, className, allowExpand = false }: TradingViewWidgetProps) => {`, values `{"cc":9,"lines":45}`, ceiling cc 6, lines 53, base site `components/TradingViewWidget.tsx:17` with `{"cc":9,"lines":41}`

   ```text
   14 | }
   15 | 
   16 | // Fullscreen resizes the same element in place: moving or re-creating the embed would reset the chart.
   17 | const TradingViewWidget = ({ scriptUrl, config, height = 600, className, allowExpand = false }: TradingViewWidgetProps) => {
   18 |     const [isExpanded, setIsExpanded] = useState(false);
   19 | 
   20 |     useEffect(() => {
   21 |         if (!isExpanded) return;
   22 |         const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setIsExpanded(false);
   23 |         document.body.style.overflow = 'hidden';
   24 |         window.addEventListener('keydown', onKey);
   25 |         return () => {
   26 |             document.body.style.overflow = '';
   27 |             window.removeEventListener('keydown', onKey);
   28 |         };
   29 |     }, [isExpanded]);
   30 | 
   31 |     const containerRef = useTradingViewWidget(scriptUrl, { ...config, width: "100%", height: "100%", autosize: true });
   32 | 
   33 |     return (
   34 |         <>
   35 |             {isExpanded && <div className="fixed inset-0 z-50 bg-page/70 backdrop-blur-[3px]" onClick={() => setIsExpanded(false)} />}
   36 |             {/* Height lives on the frame: TradingView's autosize overwrites the container's own height with 100% */}
   37 |             <div
   38 |                 className={cn("tv-frame group", isExpanded && "fixed inset-3 z-50 shadow-[0_0_0_4px_var(--frame),0_24px_60px_oklch(0_0_0/0.6)]")}
   39 |                 style={isExpanded ? undefined : { height }}
   40 |             >
   41 |                 {allowExpand && (
   42 |                     <button
   43 |                         type="button"
   44 |                         onClick={() => setIsExpanded(!isExpanded)}
   45 |                         className={cn("icon-btn absolute top-2 right-2 z-10 bg-card/80", !isExpanded && "opacity-0 group-hover:opacity-100 focus-visible:opacity-100")}
   46 |                         aria-label={isExpanded ? "Exit full screen" : "Full screen"}
   47 |                         title={isExpanded ? "Exit full screen (Esc)" : "Full screen"}
   48 |                     >
   49 |                         {isExpanded ? <Minimize2 /> : <Maximize2 />}
   50 |                     </button>
   51 |                 )}
   52 | 
   53 |                 <div
   54 |                     ref={containerRef}
   55 |                     className={cn('tradingview-widget-container', className)}
   56 |                     style={{ height: '100%', width: '100%' }}
   57 |                 />
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R066

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 9 of 10
- Commit: `3cf500ad4c82`, judged against its first parent `eac13c65be69`, exit 2
- Gate: doc-size, FAIL
- Decision group: document README.md
- Derived API_DOCS.md: `450`, the word count at the derivation commit, rounded up to the next 50
- Derived MARKET_SUPPORT.md: `800`, the word count at the derivation commit, rounded up to the next 50
- Derived README.md: `2250`, the word count at the derivation commit, rounded up to the next 50

### Commit message

> Merge pull request #106 from Open-Dev-Society/feat/openstock-redesign
>
> Redesign, security fixes, multimarket, landing page and sponsors

### Files the change touched

```text
 MARKET_SUPPORT.md                      | 167 ++++++-------------------------------------------
 README.md                              |  65 ++++++++++++++-----
 __tests__/alert-actions.test.ts        |  78 +++++++++++++++++++++++
 __tests__/market-clock.test.ts         |  54 ++++++++++++++++
 __tests__/reset-password-email.test.ts |  12 +++-
 __tests__/utils.test.ts                |  24 ++++++-
 app/(auth)/forgot-password/page.tsx    |   2 +-
 app/(auth)/layout.tsx                  |  25 +++++---
 app/(auth)/sign-in/page.tsx            |  19 +++---
 app/(auth)/sign-up/page.tsx            | 155 ++++++++++++++++++++--------------------------
 app/(marketing)/about/page.tsx         |  64 +++++++++++++++++++
 app/(marketing)/api-docs/page.tsx      | 129 ++++++++++++++++++++++++++++++++++++++
 app/(marketing)/help/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(marketing)/layout.tsx             |  77 +++++++++++++++++++++++
 app/(marketing)/page.tsx               | 226 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/sponsor/page.tsx       | 221 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/terms/page.tsx         |  87 ++++++++++++++++++++++++++
 app/(root)/about/page.tsx              | 133 ---------------------------------------
 app/(root)/api-docs/page.tsx           | 247 -------------------------------------------------------------------------
 app/(root)/dashboard/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(root)/help/page.tsx               | 124 -------------------------------------
 app/(root)/layout.tsx                  |  35 ++++++-----
 app/(root)/loading.tsx                 |  21 +++++++
 app/(root)/page.tsx                    |  64 -------------------
 app/(root)/profile/page.tsx            |  96 ++++++++++++++++++++++++++++
 app/(root)/stocks/[symbol]/page.tsx    | 131 +++++++++++++++++----------------------
 app/(root)/terms/page.tsx              |  98 -----------------------------
 app/(root)/watchlist/page.tsx          | 107 ++++++++++++++++----------------
 app/api/auth/[...all]/route.ts         |   5 ++
 app/api/quotes/route.ts                |  23 +++++++
 ...
 104 files changed, 5999 insertions(+), 3947 deletions(-)
```

### Findings

Condition: over its word ceiling.

1. `README.md` new, values `{"ceiling":2250,"words":2482}`, nothing at the base matched

### Remedy klin printed

> An instruction that can be a gate costs no words — encode it as a gate and point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling is a decision to say why in the commit.

## R067

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 9 of 10
- Commit: `3cf500ad4c82`, judged against its first parent `eac13c65be69`, exit 2
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Merge pull request #106 from Open-Dev-Society/feat/openstock-redesign
>
> Redesign, security fixes, multimarket, landing page and sponsors

### Files the change touched

```text
 MARKET_SUPPORT.md                      | 167 ++++++-------------------------------------------
 README.md                              |  65 ++++++++++++++-----
 __tests__/alert-actions.test.ts        |  78 +++++++++++++++++++++++
 __tests__/market-clock.test.ts         |  54 ++++++++++++++++
 __tests__/reset-password-email.test.ts |  12 +++-
 __tests__/utils.test.ts                |  24 ++++++-
 app/(auth)/forgot-password/page.tsx    |   2 +-
 app/(auth)/layout.tsx                  |  25 +++++---
 app/(auth)/sign-in/page.tsx            |  19 +++---
 app/(auth)/sign-up/page.tsx            | 155 ++++++++++++++++++++--------------------------
 app/(marketing)/about/page.tsx         |  64 +++++++++++++++++++
 app/(marketing)/api-docs/page.tsx      | 129 ++++++++++++++++++++++++++++++++++++++
 app/(marketing)/help/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(marketing)/layout.tsx             |  77 +++++++++++++++++++++++
 app/(marketing)/page.tsx               | 226 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/sponsor/page.tsx       | 221 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/terms/page.tsx         |  87 ++++++++++++++++++++++++++
 app/(root)/about/page.tsx              | 133 ---------------------------------------
 app/(root)/api-docs/page.tsx           | 247 -------------------------------------------------------------------------
 app/(root)/dashboard/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(root)/help/page.tsx               | 124 -------------------------------------
 app/(root)/layout.tsx                  |  35 ++++++-----
 app/(root)/loading.tsx                 |  21 +++++++
 app/(root)/page.tsx                    |  64 -------------------
 app/(root)/profile/page.tsx            |  96 ++++++++++++++++++++++++++++
 app/(root)/stocks/[symbol]/page.tsx    | 131 +++++++++++++++++----------------------
 app/(root)/terms/page.tsx              |  98 -----------------------------
 app/(root)/watchlist/page.tsx          | 107 ++++++++++++++++----------------
 app/api/auth/[...all]/route.ts         |   5 ++
 app/api/quotes/route.ts                |  23 +++++++
 ...
 104 files changed, 5999 insertions(+), 3947 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `__tests__/reset-password-email.test.ts:14` new, `const sendMailMock = vi.mocked(transporter!.sendMail);`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   11 | 
   12 | describe('sendPasswordResetEmail', () => {
   13 |     const originalEnv = { ...process.env };
   14 |     const sendMailMock = vi.mocked(transporter!.sendMail);
   15 | 
   16 |     beforeEach(() => {
   17 |         process.env = {
   18 |             ...originalEnv,
   ```

2. `app/(marketing)/sponsor/page.tsx:187` new, `{TIER_ORDER.map((id) => <th key={id} className="text-center">{SPONSOR_TIERS.find((t) => t.id === id)!.name}</th>)}`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   184 |                             <thead>
   185 |                                 <tr>
   186 |                                     <th>Placement</th>
   187 |                                     {TIER_ORDER.map((id) => <th key={id} className="text-center">{SPONSOR_TIERS.find((t) => t.id === id)!.name}</th>)}
   188 |                                 </tr>
   189 |                             </thead>
   190 |                             <tbody>
   191 |                                 {PLACEMENTS.map(({ label, from }) => (
   ```

3. `components/watchlist/CreateAlertModal.tsx:36` new, `if (next) setTargetPrice(hasPrice ? currentPrice!.toFixed(2) : '');`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   33 | 
   34 |     const onOpenChange = (next: boolean) => {
   35 |         setOpen(next);
   36 |         if (next) setTargetPrice(hasPrice ? currentPrice!.toFixed(2) : '');
   37 |     };
   38 | 
   39 |     const handleSubmit = async (e: React.FormEvent) => {
   40 |         e.preventDefault();
   ```


### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R068

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 9 of 10
- Commit: `3cf500ad4c82`, judged against its first parent `eac13c65be69`, exit 2
- Gate: complexity, ERR
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over; unparsed record
- Derived cc: `6`, 95th percentile of 444 functions at eac13c6, floor 5; recorded scope: whole repository
- Derived lines: `75`, 95th percentile of 444 functions at eac13c6, floor 25; recorded scope: whole repository

### Commit message

> Merge pull request #106 from Open-Dev-Society/feat/openstock-redesign
>
> Redesign, security fixes, multimarket, landing page and sponsors

### Files the change touched

```text
 MARKET_SUPPORT.md                      | 167 ++++++-------------------------------------------
 README.md                              |  65 ++++++++++++++-----
 __tests__/alert-actions.test.ts        |  78 +++++++++++++++++++++++
 __tests__/market-clock.test.ts         |  54 ++++++++++++++++
 __tests__/reset-password-email.test.ts |  12 +++-
 __tests__/utils.test.ts                |  24 ++++++-
 app/(auth)/forgot-password/page.tsx    |   2 +-
 app/(auth)/layout.tsx                  |  25 +++++---
 app/(auth)/sign-in/page.tsx            |  19 +++---
 app/(auth)/sign-up/page.tsx            | 155 ++++++++++++++++++++--------------------------
 app/(marketing)/about/page.tsx         |  64 +++++++++++++++++++
 app/(marketing)/api-docs/page.tsx      | 129 ++++++++++++++++++++++++++++++++++++++
 app/(marketing)/help/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(marketing)/layout.tsx             |  77 +++++++++++++++++++++++
 app/(marketing)/page.tsx               | 226 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/sponsor/page.tsx       | 221 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/terms/page.tsx         |  87 ++++++++++++++++++++++++++
 app/(root)/about/page.tsx              | 133 ---------------------------------------
 app/(root)/api-docs/page.tsx           | 247 -------------------------------------------------------------------------
 app/(root)/dashboard/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(root)/help/page.tsx               | 124 -------------------------------------
 app/(root)/layout.tsx                  |  35 ++++++-----
 app/(root)/loading.tsx                 |  21 +++++++
 app/(root)/page.tsx                    |  64 -------------------
 app/(root)/profile/page.tsx            |  96 ++++++++++++++++++++++++++++
 app/(root)/stocks/[symbol]/page.tsx    | 131 +++++++++++++++++----------------------
 app/(root)/terms/page.tsx              |  98 -----------------------------
 app/(root)/watchlist/page.tsx          | 107 ++++++++++++++++----------------
 app/api/auth/[...all]/route.ts         |   5 ++
 app/api/quotes/route.ts                |  23 +++++++
 ...
 104 files changed, 5999 insertions(+), 3947 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 6 or body > 75 lines).

1. `app/(auth)/sign-in/page.tsx:15` new, `const SignIn = () => {`, values `{"cc":2,"lines":81}`, ceiling cc 6, lines 75, nothing at the base matched

   ```text
   12 | import SocialAuthButtons from "@/components/forms/SocialAuthButtons";
   13 | import React from "react";
   14 | 
   15 | const SignIn = () => {
   16 |     const router = useRouter()
   17 |     const {
   18 |         register,
   19 |         handleSubmit,
   20 |         formState: { errors, isSubmitting },
   21 |     } = useForm<SignInFormData>({
   22 |         defaultValues: {
   23 |             email: '',
   24 |             password: '',
   25 |         },
   26 |         mode: 'onBlur',
   27 |     });
   28 | 
   29 |     const onSubmit = async (data: SignInFormData) => {
   30 |         try {
   31 |             const result = await signInWithEmail(data);
   32 |             if (result.success) {
   33 |                 router.push('/dashboard');
   34 |                 return;
   35 |             }
   36 |             toast.error('Sign in failed', {
   37 |                 description: result.error ?? 'Invalid email or password.',
   38 |             });
   39 |         } catch (e) {
   40 |             console.error(e);
   41 |             toast.error('Sign in failed', {
   42 |                 description: e instanceof Error ? e.message : 'Failed to sign in.'
   43 |             })
   44 |         }
   45 |     }
   46 | 
   47 |     return (
   48 |         <>
   49 |             <h1 className="form-title mb-2">Welcome back</h1>
   50 |             <p className="mb-8 text-faint">Sign in to your watchlist and alerts.</p>
   51 | 
   52 |             <SocialAuthButtons />
   53 | 
   54 |             <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
   55 |                 <InputField
   ```

2. `app/(auth)/sign-up/page.tsx:18` new, `const SignUp = () => {`, values `{"cc":3,"lines":115}`, ceiling cc 6, lines 75, nothing at the base matched

   ```text
   15 | import SocialAuthButtons from "@/components/forms/SocialAuthButtons";
   16 | import React from "react";
   17 | 
   18 | const SignUp = () => {
   19 |     const router = useRouter()
   20 |     const {
   21 |         register,
   22 |         handleSubmit,
   23 |         control,
   24 |         watch,
   25 |         formState: { errors, isSubmitting },
   26 |     } = useForm<SignUpFormData>({
   27 |         defaultValues: {
   28 |             fullName: '',
   29 |             email: '',
   30 |             password: '',
   31 |             country: 'IN',
   32 |             investmentGoals: 'Growth',
   33 |             riskTolerance: 'Medium',
   34 |             preferredIndustry: 'Technology'
   35 |         },
   36 |         mode: 'onBlur'
   37 |     },);
   38 | 
   39 |     const passwordValue = watch('password');
   40 | 
   41 |     const onSubmit = async (data: SignUpFormData) => {
   42 |         try {
   43 |             const result = await signUpWithEmail(data);
   44 |             if (result.success) {
   45 |                 router.push('/dashboard');
   46 |                 return;
   47 |             }
   48 |             toast.error('Sign up failed', {
   49 |                 description: result.error ?? 'We could not create your account.',
   50 |             });
   51 |         } catch (e) {
   52 |             console.error(e);
   53 |             toast.error('Sign up failed', {
   54 |                 description: e instanceof Error ? e.message : 'Failed to create an account.'
   55 |             })
   56 |         }
   57 |     }
   58 | 
   ```

3. `app/(marketing)/api-docs/page.tsx:42` new, `export default function ArchitecturePage() {`, values `{"cc":1,"lines":88}`, ceiling cc 6, lines 75, nothing at the base matched

   ```text
   39 |     ['Kit', 'Newsletter broadcasts for the weekly digest.', 'https://kit.com'],
   40 | ];
   41 | 
   42 | export default function ArchitecturePage() {
   43 |     return (
   44 |         <>
   45 |             <PageHero
   46 |                 kicker="Architecture"
   47 |                 title="How OpenStock works."
   48 |                 sub="A transparent look at the event-driven, multi-provider system behind your market data and emails."
   49 |             >
   50 |                 <span className="pill h-8 px-3">v1.0.0</span>
   51 |                 <span className="pill h-8 px-3">Gemini with MiniMax fallback</span>
   52 |                 <span className="pill h-8 px-3">Open source · AGPL-3.0</span>
   53 |             </PageHero>
   54 | 
   55 |             <section className="mx-auto mt-20 max-w-[1200px] px-5">
   56 |                 <SectionHead kicker="Market data" title="Two data modes, one cache." sub="Charts always stream from TradingView. Everything we price ourselves goes through a shared cache with timeouts, so a slow provider never stalls a page." />
   57 |                 <div className="grid gap-3 md:grid-cols-2">
   58 |                     {DATA_MODES.map(({ name, cadence, body }) => (
   59 |                         <div key={name} className="hatch">
   60 |                             <div className="card flex h-full flex-col gap-2 p-5">
   61 |                                 <div className="flex items-center justify-between">
   62 |                                     <h3 className="text-[16px] font-bold">{name}</h3>
   63 |                                     <span className="pill is-brand">{cadence}</span>
   64 |                                 </div>
   65 |                                 <p className="text-[14px] leading-relaxed text-muted-foreground">{body}</p>
   66 |                             </div>
   67 |                         </div>
   68 |                     ))}
   69 |                 </div>
   70 |                 <div className="hatch mt-3">
   71 |                     <div className="card overflow-x-auto">
   72 |                         <table className="data-table">
   73 |                             <thead><tr><th>Market</th><th>Charts</th><th>Our quotes and alerts</th></tr></thead>
   74 |                             <tbody>
   75 |                                 {COVERAGE.map(([market, charts, quotes]) => (
   76 |                                     <tr key={market}>
   77 |                                         <td className="font-semibold text-foreground">{market}</td>
   78 |                                         <td className="text-muted-foreground">{charts}</td>
   79 |                                         <td><span className={quotes === 'Yes' ? 'pill is-up' : 'pill'}>{quotes}</span></td>
   80 |                                     </tr>
   81 |                                 ))}
   82 |                             </tbody>
   ```

4. `app/(marketing)/page.tsx:70` new, `export default async function LandingPage() {`, values `{"cc":5,"lines":157}`, ceiling cc 6, lines 75, nothing at the base matched
5. `app/(marketing)/sponsor/page.tsx:31` new, `export default async function SponsorPage() {`, values `{"cc":5,"lines":191}`, ceiling cc 6, lines 75, nothing at the base matched
6. `app/(marketing)/sponsor/page.tsx:81` new, `{SPONSOR_TIERS.map((tier) => {`, values `{"cc":11,"lines":33}`, ceiling cc 6, lines 75, nothing at the base matched
7. `app/(root)/dashboard/page.tsx:21` new, `export default async function Dashboard({ searchParams }: { searchParams: Promise<{ market?: string }> }) {`, values `{"cc":11,"lines":68}`, ceiling cc 6, lines 75, nothing at the base matched
8. `app/(root)/profile/page.tsx:19` new, `export default async function ProfilePage() {`, values `{"cc":8,"lines":78}`, ceiling cc 6, lines 75, nothing at the base matched
9. `components/PulseTile.tsx:14` new, `export default function PulseTile({ symbol, label, initial }: { symbol: string; label: string; initial: LiveQuote | null }) {`, values `{"cc":8,"lines":30}`, ceiling cc 6, lines 75, nothing at the base matched
10. `components/SearchCommand.tsx:19` new, `export default function SearchCommand({ initialStocks }: { initialStocks: StockWithWatchlistStatus[] }) {`, values `{"cc":9,"lines":95}`, ceiling cc 6, lines 75, nothing at the base matched
11. `components/TradingViewWidget.tsx:17` new, `const TradingViewWidget = ({ scriptUrl, config, height = 600, className, allowExpand = false }: TradingViewWidgetProps) => {`, values `{"cc":9,"lines":41}`, ceiling cc 6, lines 75, nothing at the base matched
12. `components/WatchlistButton.tsx:18` new, `const WatchlistButton = ({ symbol, company, isInWatchlist, variant = "button", onWatchlistChange }: WatchlistButtonProps) => {`, values `{"cc":8,"lines":43}`, ceiling cc 6, lines 75, nothing at the base matched
13. `components/shell/Sidebar.tsx:24` new, `const Sidebar = ({ user, watchlist }: SidebarProps) => {`, values `{"cc":9,"lines":104}`, ceiling cc 6, lines 75, nothing at the base matched
14. `components/shell/TabBar.tsx:34` new, `const TabBar = ({ onMenu }: { onMenu: () => void }) => {`, values `{"cc":3,"lines":91}`, ceiling cc 6, lines 75, nothing at the base matched
15. `components/shell/TabBar.tsx:88` new, `{tabs.map((path) => {`, values `{"cc":8,"lines":29}`, ceiling cc 6, lines 75, nothing at the base matched
16. `components/stocks/StockHeader.tsx:37` new, `export default async function StockHeader({ symbol }: { symbol: string }) {`, values `{"cc":8,"lines":60}`, ceiling cc 6, lines 75, nothing at the base matched
17. `components/watchlist/CreateAlertModal.tsx:23` new, `export default function CreateAlertModal({ symbol, currentPrice, currency = 'USD', children, onAlertCreated }: CreateAlertModalProps) {`, values `{"cc":15,"lines":96}`, ceiling cc 6, lines 75, nothing at the base matched
18. `components/watchlist/WatchlistTable.tsx:19` new, `export default function WatchlistTable({ initialRows }: { initialRows: Row[] }) {`, values `{"cc":2,"lines":92}`, ceiling cc 6, lines 75, nothing at the base matched
19. `components/watchlist/WatchlistTable.tsx:61` new, `{rows.map((row) => (`, values `{"cc":8,"lines":45}`, ceiling cc 6, lines 75, nothing at the base matched
20. `lib/actions/alert.actions.ts:15` new, `export async function createAlert(params: {`, values `{"cc":8,"lines":27}`, ceiling cc 6, lines 75, nothing at the base matched
21. `lib/actions/finnhub.actions.ts:120` new, `async function fetchJSON<T>(url: string, revalidateSeconds = 0): Promise<T> {`, values `{"cc":7,"lines":29}`, ceiling cc 6, lines 75, nothing at the base matched
22. `lib/actions/finnhub.actions.ts:200` new, `const promises = symbols.map(async (sym) => {`, values `{"cc":7,"lines":18}`, ceiling cc 6, lines 75, nothing at the base matched
23. `lib/actions/profile.actions.ts:19` new, `export async function updateProfile(input: ProfileInput) {`, values `{"cc":8,"lines":28}`, ceiling cc 6, lines 75, nothing at the base matched
24. `lib/inngest/functions.ts:275` new, `await step.run('process-triggered-alerts', async () => {`, values `{"cc":8,"lines":48}`, ceiling cc 6, lines 75, nothing at the base matched
25. `lib/market-session.ts:19` new, `export function sessionOf(weekday: number, minutes: number): { phase: Phase; line: string } {`, values `{"cc":10,"lines":13}`, ceiling cc 6, lines 75, nothing at the base matched
26. `lib/utils.ts:122` new, `export function formatNumber(num?: number | null): string {`, values `{"cc":8,"lines":16}`, ceiling cc 6, lines 75, nothing at the base matched
27. `lib/actions/finnhub.actions.ts:298` worsened, `export const searchStocks = cache(async (query?: string): Promise<StockWithWatchlistStatus[]> => {`, values `{"cc":7,"lines":53}`, ceiling cc 6, lines 75, base site `lib/actions/finnhub.actions.ts:193` with `{"cc":6,"lines":76}`
28. `lib/inngest/functions.ts:209` worsened, `async ({ step }) => {`, values `{"cc":11,"lines":121}`, ceiling cc 6, lines 75, base site `lib/inngest/functions.ts:207` with `{"cc":11,"lines":87}`
29. `lib/constants.ts` unparsed, `the TypeScript grammar rejected it`
30. `lib/markets.ts` unparsed, `the TypeScript grammar rejected it`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R069

- Repository: `Open-Dev-Society/OpenStock` (TypeScript), change 9 of 10
- Commit: `3cf500ad4c82`, judged against its first parent `eac13c65be69`, exit 2
- Gate: dead-symbols, ERR
- Decision group: dead-symbols

### Commit message

> Merge pull request #106 from Open-Dev-Society/feat/openstock-redesign
>
> Redesign, security fixes, multimarket, landing page and sponsors

### Files the change touched

```text
 MARKET_SUPPORT.md                      | 167 ++++++-------------------------------------------
 README.md                              |  65 ++++++++++++++-----
 __tests__/alert-actions.test.ts        |  78 +++++++++++++++++++++++
 __tests__/market-clock.test.ts         |  54 ++++++++++++++++
 __tests__/reset-password-email.test.ts |  12 +++-
 __tests__/utils.test.ts                |  24 ++++++-
 app/(auth)/forgot-password/page.tsx    |   2 +-
 app/(auth)/layout.tsx                  |  25 +++++---
 app/(auth)/sign-in/page.tsx            |  19 +++---
 app/(auth)/sign-up/page.tsx            | 155 ++++++++++++++++++++--------------------------
 app/(marketing)/about/page.tsx         |  64 +++++++++++++++++++
 app/(marketing)/api-docs/page.tsx      | 129 ++++++++++++++++++++++++++++++++++++++
 app/(marketing)/help/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(marketing)/layout.tsx             |  77 +++++++++++++++++++++++
 app/(marketing)/page.tsx               | 226 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/sponsor/page.tsx       | 221 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 app/(marketing)/terms/page.tsx         |  87 ++++++++++++++++++++++++++
 app/(root)/about/page.tsx              | 133 ---------------------------------------
 app/(root)/api-docs/page.tsx           | 247 -------------------------------------------------------------------------
 app/(root)/dashboard/page.tsx          |  88 ++++++++++++++++++++++++++
 app/(root)/help/page.tsx               | 124 -------------------------------------
 app/(root)/layout.tsx                  |  35 ++++++-----
 app/(root)/loading.tsx                 |  21 +++++++
 app/(root)/page.tsx                    |  64 -------------------
 app/(root)/profile/page.tsx            |  96 ++++++++++++++++++++++++++++
 app/(root)/stocks/[symbol]/page.tsx    | 131 +++++++++++++++++----------------------
 app/(root)/terms/page.tsx              |  98 -----------------------------
 app/(root)/watchlist/page.tsx          | 107 ++++++++++++++++----------------
 app/api/auth/[...all]/route.ts         |   5 ++
 app/api/quotes/route.ts                |  23 +++++++
 ...
 104 files changed, 5999 insertions(+), 3947 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `types/global.d.ts:42` worsened, `type SelectFieldProps = {`, values `{"dead":1,"lost_reference":"components/forms/SelectField.tsx"}`, base site `types/global.d.ts:42` with `{"dead":0}`

   ```text
   39 |         label: string;
   40 |     };
   41 | 
   42 |     type SelectFieldProps = {
   43 |         name: string;
   44 |         label: string;
   45 |         placeholder: string;
   46 |         options: readonly Option[];
   ```

2. `types/global.d.ts:58` worsened, `type SearchCommandProps = {`, values `{"dead":1,"lost_reference":"components/SearchCommand.tsx"}`, base site `types/global.d.ts:58` with `{"dead":0}`

   ```text
   55 |         href: string;
   56 |     };
   57 | 
   58 |     type SearchCommandProps = {
   59 |         renderAs?: 'button' | 'text';
   60 |         label?: string;
   61 |         initialStocks: StockWithWatchlistStatus[];
   62 |     };
   ```

3. `types/global.d.ts:134` worsened, `type WatchlistTableProps = {`, values `{"dead":1,"lost_reference":"components/watchlist/WatchlistTable.tsx"}`, base site `types/global.d.ts:134` with `{"dead":0}`

   ```text
   131 |         currentPrice?: number;
   132 |     };
   133 | 
   134 |     type WatchlistTableProps = {
   135 |         watchlist: StockWithData[];
   136 |     };
   137 | 
   138 |     type StockWithData = {
   ```

4. `types/global.d.ts:171` worsened, `type SearchCommandProps = {`, values `{"dead":1,"lost_reference":"components/SearchCommand.tsx"}`, base site `types/global.d.ts:171` with `{"dead":0}`
5. `lib/constants.ts` unparsed, `the TypeScript grammar rejected it`
6. `lib/markets.ts` unparsed, `the TypeScript grammar rejected it`

### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R070

- Repository: `mountain-loop/yaak` (TypeScript), change 1 of 10
- Commit: `7f3025361685`, judged against its first parent `6c2a18d506ae`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,495 functions at 6c2a18d, floor 5; recorded scope: whole repository
- Derived lines: `52`, 95th percentile of 8,495 functions at 6c2a18d, floor 25; recorded scope: whole repository

### Commit message

> Select new sub-environments and keep enabled state through bulk edit (#714)

### Files the change touched

```text
 apps/yaak-client/commands/createEnvironment.tsx         |  22 +++++++---
 apps/yaak-client/components/EnvironmentEditDialog.tsx   | 128 +++++++++++++++++++++++++++++++++++++----------------
 apps/yaak-client/components/core/BulkPairEditor.test.ts | 133 +++++++++++++++++++++++++++++++++++++++++++++++++++++++-
 apps/yaak-client/components/core/BulkPairEditor.tsx     |  63 +++++++++++++++++++++------
 4 files changed, 288 insertions(+), 58 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 52 lines).

1. `apps/yaak-client/components/EnvironmentEditDialog.tsx:119` worsened, `function EnvironmentEditDialogSidebar({`, values `{"cc":3,"lines":282}`, ceiling cc 9, lines 52, base site `apps/yaak-client/components/EnvironmentEditDialog.tsx:108` with `{"cc":3,"lines":244}`

   ```text
   116 |   />
   117 | );
   118 | 
   119 | function EnvironmentEditDialogSidebar({
   120 |   selectedEnvironmentId,
   121 |   setSelectedEnvironmentId,
   122 | }: {
   123 |   selectedEnvironmentId: string | null;
   124 |   setSelectedEnvironmentId: (id: string | null) => void;
   125 | }) {
   126 |   const activeWorkspaceId = useAtomValue(activeWorkspaceIdAtom) ?? "";
   127 |   const treeId = `environment.${activeWorkspaceId}.sidebar`;
   128 |   const treeRef = useRef<TreeHandle>(null);
   129 |   const { allEnvironments, baseEnvironment, baseEnvironments } = useEnvironmentsBreakdown();
   130 | 
   131 |   // oxlint-disable-next-line react-hooks/exhaustive-deps -- none
   132 |   useLayoutEffect(() => {
   133 |     if (selectedEnvironmentId == null) return;
   134 |     treeRef.current?.selectItem(selectedEnvironmentId);
   135 |     treeRef.current?.focus();
   136 |   }, []);
   137 | 
   138 |   // A just-created environment to select once it reaches the model store. Selecting it before
   139 |   // then would show a "failed to find" error in the editor, and the tree would drop the
   140 |   // selection of an item it doesn't have yet.
   141 |   const [pendingSelectionId, setPendingSelectionId] = useState<string | null>(null);
   142 |   const tree = useAtomValue(treeAtom);
   143 |   useEffect(() => {
   144 |     if (pendingSelectionId == null) return;
   145 |     if (!allEnvironments.some((e) => e.id === pendingSelectionId)) return;
   146 | 
   147 |     setPendingSelectionId(null);
   148 |     setSelectedEnvironmentId(pendingSelectionId);
   149 | 
   150 |     // The tree leaves out sub-environments while there are multiple base environments
   151 |     const node = tree == null ? null : findTreeNode(tree, pendingSelectionId);
   152 |     if (node == null) return;
   153 |     if (node.parent != null) {
   154 |       // Expand the parent so the new item is visible to be selected
   155 |       const parentId = node.parent.item.id;
   156 |       jotaiStore.set(collapsedFamily(treeId), (prev) => ({ ...prev, [parentId]: false }));
   157 |     }
   158 |     // Wait a frame so the tree's own pending selection fixups, which may have been scheduled
   159 |     // before the new item arrived, don't reset this selection
   ```

2. `apps/yaak-client/components/EnvironmentEditDialog.tsx:237` worsened, `(items: TreeModel[]): ContextMenuProps["items"] => {`, values `{"cc":14,"lines":75}`, ceiling cc 9, lines 52, base site `apps/yaak-client/components/EnvironmentEditDialog.tsx:193` with `{"cc":12,"lines":74}`

   ```text
   234 |   );
   235 | 
   236 |   const getContextMenu = useCallback(
   237 |     (items: TreeModel[]): ContextMenuProps["items"] => {
   238 |       const environment = items[0];
   239 |       const addEnvironmentItem: DropdownItem = {
   240 |         label: "Create Sub Environment",
   241 |         leftSlot: <Icon icon="plus" />,
   242 |         onSelect: handleCreateSubEnvironment,
   243 |       };
   244 |       // Sub-environments aren't shown while there are multiple base environments, same as the
   245 |       // plus button on the base environment row
   246 |       const canCreateSubEnvironment = baseEnvironments.length <= 1;
   247 | 
   248 |       if (environment == null || environment.model !== "environment") {
   249 |         return canCreateSubEnvironment ? [addEnvironmentItem] : [];
   250 |       }
   251 | 
   252 |       const singleEnvironment = items.length === 1;
   253 |       const canDeleteEnvironment =
   254 |         isSubEnvironment(environment) ||
   255 |         (isBaseEnvironment(environment) && baseEnvironments.length > 1);
   256 | 
   257 |       const menuItems: DropdownItem[] = [
   258 |         {
   259 |           label: "Rename",
   260 |           leftSlot: <Icon icon="pencil" />,
   261 |           hidden: isBaseEnvironment(environment) || !singleEnvironment,
   262 |           hotKeyAction: "sidebar.selected.rename",
   263 |           hotKeyLabelOnly: true,
   264 |           onSelect: () => {
   265 |             // Not sure why this is needed, but without it the
   266 |             // edit input blurs immediately after opening.
   267 |             requestAnimationFrame(() => handleRenameSelected());
   268 |           },
   269 |         },
   270 |         {
   271 |           label: "Duplicate",
   272 |           leftSlot: <Icon icon="copy" />,
   273 |           hidden: isBaseEnvironment(environment),
   274 |           hotKeyAction: "sidebar.selected.duplicate",
   275 |           hotKeyLabelOnly: true,
   276 |           onSelect: () => handleDuplicateSelected(items),
   277 |         },
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R071

- Repository: `mountain-loop/yaak` (TypeScript), change 2 of 10
- Commit: `6c2a18d506ae`, judged against its first parent `0a57d8eb6123`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,494 functions at 0a57d8e, floor 5; recorded scope: whole repository
- Derived lines: `52`, 95th percentile of 8,494 functions at 0a57d8e, floor 25; recorded scope: whole repository

### Commit message

> Attach CodeMirror editors after building them to batch layout work (#713)

### Files the change touched

```text
 apps/yaak-client/components/core/Editor/Editor.tsx | 42 +++++++++++++++++++++++++++++++++++-------
 1 file changed, 35 insertions(+), 7 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 52 lines).

1. `apps/yaak-client/components/core/Editor/Editor.tsx:111` worsened, `function EditorInner({`, values `{"cc":25,"lines":488}`, ceiling cc 9, lines 52, base site `apps/yaak-client/components/core/Editor/Editor.tsx:111` with `{"cc":25,"lines":460}`

   ```text
   108 |   return <EditorInner key={props.stateKey} {...props} />;
   109 | }
   110 | 
   111 | function EditorInner({
   112 |   actions,
   113 |   autoFocus,
   114 |   autoSelect,
   115 |   autocomplete,
   116 |   autocompleteFunctions,
   117 |   autocompleteVariables,
   118 |   className,
   119 |   defaultValue,
   120 |   disableTabIndent,
   121 |   disabled,
   122 |   extraExtensions,
   123 |   forcedEnvironmentId,
   124 |   forceUpdateKey,
   125 |   format,
   126 |   heightMode,
   127 |   hideGutter,
   128 |   graphQLSchema,
   129 |   language,
   130 |   lintExtension,
   131 |   onBlur,
   132 |   onChange,
   133 |   onFocus,
   134 |   onKeyDown,
   135 |   onPaste,
   136 |   onPasteOverwrite,
   137 |   placeholder,
   138 |   readOnly,
   139 |   singleLine,
   140 |   containerOnly,
   141 |   stateKey,
   142 |   type,
   143 |   wrapLines,
   144 |   setRef,
   145 | }: EditorProps) {
   146 |   const settings = useAtomValue(settingsAtom);
   147 | 
   148 |   const allEnvironmentVariables = useEnvironmentVariables(forcedEnvironmentId ?? null);
   149 |   const useTemplating = !!(autocompleteFunctions || autocompleteVariables || autocomplete);
   150 |   const environmentVariables = useMemo(() => {
   151 |     if (!autocompleteVariables) return emptyVariables;
   ```

2. `apps/yaak-client/components/core/Editor/Editor.tsx:384` worsened, `function initEditorRef(container: HTMLDivElement | null) {`, values `{"cc":13,"lines":108}`, ceiling cc 9, lines 52, base site `apps/yaak-client/components/core/Editor/Editor.tsx:384` with `{"cc":15,"lines":80}`

   ```text
   381 |   // Initialize the editor when ref mounts
   382 |   // oxlint-disable-next-line react-hooks/exhaustive-deps -- only reinitialize when necessary
   383 |   const initEditorRef = useCallback(
   384 |     function initEditorRef(container: HTMLDivElement | null) {
   385 |       if (container === null) {
   386 |         flushCachedEditorState(stateKey);
   387 |         cm.current?.view.destroy();
   388 |         cm.current = null;
   389 |         return;
   390 |       }
   391 | 
   392 |       try {
   393 |         const languageCompartment = new Compartment();
   394 |         const langExt = getLanguageExtension({
   395 |           useTemplating,
   396 |           language,
   397 |           lintExtension,
   398 |           completionOptions,
   399 |           autocomplete,
   400 |           environmentVariables,
   401 |           onClickVariable,
   402 |           onClickMissingVariable,
   403 |           onClickPathParameter,
   404 |           graphQLSchema: graphQLSchema ?? null,
   405 |         });
   406 |         const extensions = [
   407 |           languageCompartment.of(langExt),
   408 |           placeholderCompartment.current.of(placeholderExt(placeholderElFromText(placeholder))),
   409 |           wrapLinesCompartment.current.of(wrapLines ? EditorView.lineWrapping : emptyExtension),
   410 |           tabIndentCompartment.current.of(
   411 |             !disableTabIndent ? keymap.of([indentWithTab]) : emptyExtension,
   412 |           ),
   413 |           keymapCompartment.current.of(
   414 |             keymapExtensions[settings.editorKeymap] ?? keymapExtensions.default,
   415 |           ),
   416 |           readOnlyCompartment.current.of(readOnly ? readonlyExtensions : editableExtensions),
   417 |           ...getExtensions({
   418 |             container,
   419 |             singleLine,
   420 |             hideGutter,
   421 |             stateKey,
   422 |             onChange: handleChange,
   423 |             onPaste: handlePaste,
   424 |             onPasteOverwrite: handlePasteOverwrite,
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R072

- Repository: `mountain-loop/yaak` (TypeScript), change 3 of 10
- Commit: `0a57d8eb6123`, judged against its first parent `411aa262c7a6`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,466 functions at 411aa26, floor 5; recorded scope: whole repository
- Derived lines: `52`, 95th percentile of 8,466 functions at 411aa26, floor 25; recorded scope: whole repository

### Commit message

> Convert the remaining Insomnia tags to native Yaak functions (#712)

### Files the change touched

```text
 plugins/importer-insomnia/src/templates.ts        | 161 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----
 plugins/importer-insomnia/tests/templates.test.ts | 143 +++++++++++++++++++++++++++++++++++++++++++++++++++++++
 2 files changed, 294 insertions(+), 10 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 52 lines).

1. `plugins/importer-insomnia/src/templates.ts:98` new, `function convertBase64(args: string[]): string | null {`, values `{"cc":10,"lines":15}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   095 |   hex: "base64",
   096 | };
   097 | 
   098 | function convertBase64(args: string[]): string | null {
   099 |   if (args.length < 2 || args.length > 3) return null;
   100 |   // Assumed: exports written before the Kind argument existed pass (action, value), so a
   101 |   // lone second argument that names no kind is the value.
   102 |   const legacy = args.length === 2 && !Object.hasOwn(base64Encodings, args[1]!);
   103 |   const encoding = base64Encodings[legacy ? "normal" : args[1]!];
   104 |   const value = legacy ? args[1]! : (args[2] ?? "");
   105 |   if (encoding == null) return null;
   106 |   if (args[0] === "encode") {
   107 |     return `base64.encode(encoding='${encoding}', value=${argument(value)})`;
   108 |   }
   109 |   // Yaak's decoder reads both alphabets, so the kind doesn't change the call.
   110 |   if (args[0] === "decode") return `base64.decode(value=${argument(value)})`;
   111 |   return null;
   112 | }
   113 | 
   ```

2. `plugins/importer-insomnia/src/templates.ts:148` new, `function convertPrompt(args: string[]): string | null {`, values `{"cc":10,"lines":21}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   145 |   }
   146 | }
   147 | 
   148 | function convertPrompt(args: string[]): string | null {
   149 |   if (args.length > 6) return null;
   150 |   const [title = "", label = "", defaultValue = "", storageKey = "", mask = "false"] = args;
   151 |   // Insomnia requires a title and shows the label above the input, falling back to the title.
   152 |   if (title === "" && label === "") return null;
   153 |   const parts = [`label=${argument(label === "" ? title : label)}`];
   154 |   const masked = mask === "true";
   155 |   if (storageKey !== "" && !masked) {
   156 |     // Insomnia keeps a stored value only until the app closes. Yaak's nearest option is to
   157 |     // store it forever, which is close enough for plain values. Masked values are left
   158 |     // unstored, which is Yaak's default, so a password never outlives the session it was
   159 |     // typed in. Storing needs a namespace, and the workspace is what Yaak's editor defaults to.
   160 |     // oxlint-disable-next-line no-template-curly-in-string -- Yaak template syntax
   161 |     const namespace = argument("${[ctx.workspace()]}");
   162 |     parts.push("store='forever'", `namespace=${namespace}`, `key=${argument(storageKey)}`);
   163 |   }
   164 |   if (title !== "") parts.push(`title=${argument(title)}`);
   165 |   if (defaultValue !== "") parts.push(`defaultValue=${argument(defaultValue)}`);
   166 |   if (masked) parts.push("password=true");
   167 |   return `prompt.text(${parts.join(", ")})`;
   168 | }
   169 | 
   ```

3. `plugins/importer-insomnia/src/templates.ts:170` new, `function convertResponse(args: string[], requestIds: Set<string>): string | null {`, values `{"cc":15,"lines":30}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   167 |   return `prompt.text(${parts.join(", ")})`;
   168 | }
   169 | 
   170 | function convertResponse(args: string[], requestIds: Set<string>): string | null {
   171 |   if (args.length < 2 || args.length > 5) return null;
   172 |   const [attribute, request, filter = "", behavior = "never", maxAge = "0"] = args;
   173 |   if (!request || !requestIds.has(convertId(request))) return null;
   174 |   const behaviors: Record<string, string> = {
   175 |     never: "never",
   176 |     "no-history": "smart",
   177 |     always: "always",
   178 |     "when-expired": "ttl",
   179 |   };
   180 |   const convertedBehavior = Object.hasOwn(behaviors, behavior) ? behaviors[behavior] : null;
   181 |   if (!convertedBehavior || !/^\d+$/.test(maxAge)) return null;
   182 |   // Insomnia treats zero as immediately expired; Yaak uses zero for never expiring.
   183 |   if (convertedBehavior === "ttl" && Number(maxAge) === 0) return null;
   184 |   const common = `request=${argument(convertId(request))}, behavior='${convertedBehavior}', ttl='${maxAge}'`;
   185 |   if (attribute === "raw") {
   186 |     return `response.body.raw(${common})`;
   187 |   }
   188 |   if (attribute === "body") {
   189 |     // Insomnia returns a scalar for one match, but an array for several. Yaak's first/all
   190 |     // modes cannot express that automatically. Only convert definite paths for now.
   191 |     if (!/^\$(?:\.[\p{L}\p{N}_-]+|\[(?:0|[1-9]\d*)\])*$/u.test(filter.trim())) return null;
   192 |     return `response.body.path(${common}, path=${argument(filter.trim())}, result='first')`;
   193 |   }
   194 |   if (attribute === "header" && filter !== "") {
   195 |     return `response.header(${common}, header=${argument(filter.trim())})`;
   196 |   }
   197 |   // Insomnia's url attribute has no counterpart, since Yaak has no response URL function.
   198 |   return null;
   199 | }
   200 | 
   ```

4. `plugins/importer-insomnia/src/templates.ts:24` worsened, `function parseArgs(input: string): string[] | null {`, values `{"cc":11,"lines":24}`, ceiling cc 9, lines 52, base site `plugins/importer-insomnia/src/templates.ts:24` with `{"cc":10,"lines":24}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R073

- Repository: `mountain-loop/yaak` (TypeScript), change 4 of 10
- Commit: `411aa262c7a6`, judged against its first parent `34815f327a73`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> Test the plugin's copy of the template tag scanner (#711)

### Files the change touched

```text
 apps/yaak-client/lib/resolvedModelName.cases.ts                   | 79 +++++++++++++++++++++++++++++++++++++++++++++++
 apps/yaak-client/lib/resolvedModelName.test.ts                    | 60 ++++-------------------------------
 plugins/template-function-request/tests/resolvedModelName.test.ts | 42 +++++++++++++++++++++++++
 3 files changed, 127 insertions(+), 54 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `plugins/template-function-request/tests/resolvedModelName.test.ts:24` new, `const { resolvedModelName } = await import("../src");`, values `{"dead":1}`, nothing at the base matched

   ```text
   21 | vi.mock("@yaakapp-internal/models", () => ({ foldersAtom: {} }));
   22 | vi.mock("../../../apps/yaak-client/lib/jotai", () => ({ jotaiStore: { get: () => [] } }));
   23 | 
   24 | const { resolvedModelName } = await import("../src");
   25 | const { resolvedModelName: clientResolvedModelName } = await import(
   26 |   "../../../apps/yaak-client/lib/resolvedModelName"
   27 | );
   28 | 
   ```

2. `plugins/template-function-request/tests/resolvedModelName.test.ts:25` new, `const { resolvedModelName: clientResolvedModelName } = await import(`, values `{"dead":1}`, nothing at the base matched

   ```text
   22 | vi.mock("../../../apps/yaak-client/lib/jotai", () => ({ jotaiStore: { get: () => [] } }));
   23 | 
   24 | const { resolvedModelName } = await import("../src");
   25 | const { resolvedModelName: clientResolvedModelName } = await import(
   26 |   "../../../apps/yaak-client/lib/resolvedModelName"
   27 | );
   28 | 
   29 | function httpRequest(url: string, name = ""): HttpRequest {
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R074

- Repository: `mountain-loop/yaak` (TypeScript), change 5 of 10
- Commit: `34815f327a73`, judged against its first parent `6fc43a60e23b`, exit 1
- Gate: escapes, FAIL
- Decision group: escapes

### Commit message

> Fix Insomnia template and GraphQL import fidelity (#693)

### Files the change touched

```text
 crates/yaak/src/import.rs                                      | 109 ++++++++++++++++++++++++++++++-
 crates/yaak/src/import_templates.rs                            | 172 +++++++++++++++++++++++++++++++++++++++++++++++++
 crates/yaak/src/lib.rs                                         |   1 +
 plugins/importer-insomnia/src/common.ts                        | 155 ++++++++++++++++++++++++++++++++++++++------
 plugins/importer-insomnia/src/templates.ts                     | 113 ++++++++++++++++++++++++++++++++
 plugins/importer-insomnia/src/v4.ts                            |  11 ++--
 plugins/importer-insomnia/src/v5.ts                            |   7 +-
 plugins/importer-insomnia/tests/body.test.ts                   |  72 +++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/basic.output.json     |   2 +-
 plugins/importer-insomnia/tests/fixtures/chained.input.yaml    |  29 +++++++++
 plugins/importer-insomnia/tests/fixtures/chained.output.json   |  85 ++++++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/version-5.output.json |   6 +-
 plugins/importer-insomnia/tests/migration.test.ts              |  95 +++++++++++++++++++++++++++
 plugins/importer-insomnia/tests/templates.test.ts              |  88 +++++++++++++++++++++++++
 plugins/template-function-response/src/index.ts                |  11 ++--
 15 files changed, 921 insertions(+), 35 deletions(-)
```

### Findings

Condition: where the code opts out of a check.

1. `plugins/importer-insomnia/src/templates.ts:17` new, `const normalized = match[1]!.replace(/=+$/, "").replaceAll("+", "-").replaceAll("/", "_");`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   14 |   const match = /^b64::([A-Za-z0-9+/_-]*={0,2})(?:::[^:]*)?$/.exec(value);
   15 |   if (!match) return null;
   16 |   const bytes = Buffer.from(match[1]!, "base64");
   17 |   const normalized = match[1]!.replace(/=+$/, "").replaceAll("+", "-").replaceAll("/", "_");
   18 |   if (bytes.toString("base64url") !== normalized) return null;
   19 |   const decoded = bytes.toString("utf8");
   20 |   return Buffer.from(decoded).equals(bytes) ? decoded : null;
   21 | }
   ```

2. `plugins/importer-insomnia/tests/migration.test.ts:88` new, `const ref = 'b64'${Buffer.from(login!.id).toString("base64url")}'';`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   85 |       value: "application/json",
   86 |       enabled: true,
   87 |     });
   88 |     const ref = `b64'${Buffer.from(login!.id).toString("base64url")}'`;
   89 |     expect(query?.authentication?.token).toBe(
   90 |       `\${[ response.body.path(request=${ref}, behavior='ttl', ttl='60', path=b64'JC50b2tlbg', result='first') ]}`,
   91 |     );
   92 |     expect(imported.sourceKeys[login!.id]).toBe("req_login");
   ```

3. `plugins/importer-insomnia/tests/migration.test.ts:92` new, `expect(imported.sourceKeys[login!.id]).toBe("req_login");`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

   ```text
   89 |     expect(query?.authentication?.token).toBe(
   90 |       `\${[ response.body.path(request=${ref}, behavior='ttl', ttl='60', path=b64'JC50b2tlbg', result='first') ]}`,
   91 |     );
   92 |     expect(imported.sourceKeys[login!.id]).toBe("req_login");
   93 |     expect(imported.sourceKeys[query!.id]).toBe("req_query");
   94 |   });
   95 | });
   96 | 
   ```

4. `plugins/importer-insomnia/tests/migration.test.ts:93` new, `expect(imported.sourceKeys[query!.id]).toBe("req_query");`, values `{"count":1,"escape":"non-null assertion"}`, nothing at the base matched

### Remedy klin printed

> Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.

## R075

- Repository: `mountain-loop/yaak` (TypeScript), change 5 of 10
- Commit: `34815f327a73`, judged against its first parent `6fc43a60e23b`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,422 functions at 6fc43a6, floor 5; recorded scope: whole repository
- Derived lines: `52`, 95th percentile of 8,422 functions at 6fc43a6, floor 25; recorded scope: whole repository

### Commit message

> Fix Insomnia template and GraphQL import fidelity (#693)

### Files the change touched

```text
 crates/yaak/src/import.rs                                      | 109 ++++++++++++++++++++++++++++++-
 crates/yaak/src/import_templates.rs                            | 172 +++++++++++++++++++++++++++++++++++++++++++++++++
 crates/yaak/src/lib.rs                                         |   1 +
 plugins/importer-insomnia/src/common.ts                        | 155 ++++++++++++++++++++++++++++++++++++++------
 plugins/importer-insomnia/src/templates.ts                     | 113 ++++++++++++++++++++++++++++++++
 plugins/importer-insomnia/src/v4.ts                            |  11 ++--
 plugins/importer-insomnia/src/v5.ts                            |   7 +-
 plugins/importer-insomnia/tests/body.test.ts                   |  72 +++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/basic.output.json     |   2 +-
 plugins/importer-insomnia/tests/fixtures/chained.input.yaml    |  29 +++++++++
 plugins/importer-insomnia/tests/fixtures/chained.output.json   |  85 ++++++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/version-5.output.json |   6 +-
 plugins/importer-insomnia/tests/migration.test.ts              |  95 +++++++++++++++++++++++++++
 plugins/importer-insomnia/tests/templates.test.ts              |  88 +++++++++++++++++++++++++
 plugins/template-function-response/src/index.ts                |  11 ++--
 15 files changed, 921 insertions(+), 35 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 52 lines).

1. `crates/yaak/src/import_templates.rs:55` new, `fn remap_text(text: &str, ids: &BTreeMap<String, String>) -> String {`, values `{"cc":14,"lines":46}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   52 |     changed
   53 | }
   54 | 
   55 | fn remap_text(text: &str, ids: &BTreeMap<String, String>) -> String {
   56 |     // Parse individual unescaped tags. Parsing/reprinting the whole string would unescape raw
   57 |     // template-looking text, and change formatting even when no reference needs updating.
   58 |     let bytes = text.as_bytes();
   59 |     let mut output = String::new();
   60 |     let mut copied = 0;
   61 |     let mut cursor = 0;
   62 |     while let Some(relative) = text[cursor..].find("${[") {
   63 |         let start = cursor + relative;
   64 |         cursor = start + 3;
   65 |         let escapes = bytes[..start].iter().rev().take_while(|&&b| b == b'\\').count();
   66 |         if escapes % 2 != 0 {
   67 |             continue;
   68 |         }
   69 |         let mut end = cursor;
   70 |         let mut quoted = false;
   71 |         while end < bytes.len() {
   72 |             if quoted && bytes[end] == b'\\' {
   73 |                 end += 2;
   74 |                 continue;
   75 |             }
   76 |             if bytes[end] == b'\'' {
   77 |                 quoted = !quoted;
   78 |             }
   79 |             if !quoted && bytes[end..].starts_with(b"]}") {
   80 |                 end += 2;
   81 |                 break;
   82 |             }
   83 |             end += 1;
   84 |         }
   85 |         if end > bytes.len() || !text[start..end].ends_with("]}") {
   86 |             continue;
   87 |         }
   88 |         if let Ok(mut tokens) = Parser::new(&text[start..end]).parse()
   89 |             && let [Token::Tag { val }, Token::Eof] = tokens.tokens.as_mut_slice()
   90 |             && remap_argument(val, ids)
   91 |         {
   92 |             output.push_str(&text[copied..start]);
   93 |             output.push_str(&tokens.to_string());
   94 |             copied = end;
   95 |         }
   ```

2. `plugins/importer-insomnia/src/common.ts:188` new, `function readTopLevelMembers(text: string): Map<string, string> | null {`, values `{"cc":14,"lines":28}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   185 |  * The raw text of each member of the outermost object, without parsing the values.
   186 |  * Returns null unless the whole object is well-formed enough to walk.
   187 |  */
   188 | function readTopLevelMembers(text: string): Map<string, string> | null {
   189 |   const trimmed = text.trim();
   190 |   if (!trimmed.startsWith("{")) return null;
   191 | 
   192 |   const members = new Map<string, string>();
   193 |   let offset = 1;
   194 |   while (offset < trimmed.length) {
   195 |     while (/[\s,]/.test(trimmed[offset] ?? "")) offset++;
   196 |     if (trimmed[offset] === "}") return members;
   197 | 
   198 |     const keyEnd = scanJSONString(trimmed, offset);
   199 |     if (keyEnd == null) return null;
   200 |     const key = parseJSONString(trimmed.slice(offset, keyEnd));
   201 |     if (key == null) return null;
   202 | 
   203 |     offset = keyEnd;
   204 |     while (/\s/.test(trimmed[offset] ?? "")) offset++;
   205 |     if (trimmed[offset] !== ":") return null;
   206 |     offset++;
   207 |     while (/\s/.test(trimmed[offset] ?? "")) offset++;
   208 | 
   209 |     const valueEnd = scanJSONValue(trimmed, offset);
   210 |     if (valueEnd == null) return null;
   211 |     members.set(key, trimmed.slice(offset, valueEnd).trim());
   212 |     offset = valueEnd;
   213 |   }
   214 |   return null;
   215 | }
   216 | 
   ```

3. `plugins/importer-insomnia/src/common.ts:231` new, `function scanJSONValue(text: string, start: number): number | null {`, values `{"cc":12,"lines":20}`, ceiling cc 9, lines 52, nothing at the base matched

   ```text
   228 |  * Offset just past the value starting at `start`. Nesting is tracked so template
   229 |  * tags come back whole, but the value itself is not validated.
   230 |  */
   231 | function scanJSONValue(text: string, start: number): number | null {
   232 |   let depth = 0;
   233 |   for (let i = start; i < text.length; i++) {
   234 |     const char = text[i]!;
   235 |     if (char === '"') {
   236 |       const end = scanJSONString(text, i);
   237 |       if (end == null) return null;
   238 |       i = end - 1;
   239 |     } else if (char === "{" || char === "[") {
   240 |       depth++;
   241 |     } else if (char === "}" || char === "]") {
   242 |       if (depth === 0) return i; // The member ended with the enclosing object
   243 |       depth--;
   244 |       if (depth === 0) return i + 1;
   245 |     } else if (depth === 0 && char === ",") {
   246 |       return i;
   247 |     }
   248 |   }
   249 |   return null;
   250 | }
   251 | 
   ```

4. `plugins/importer-insomnia/src/templates.ts:24` new, `function parseArgs(input: string): string[] | null {`, values `{"cc":10,"lines":24}`, ceiling cc 9, lines 52, nothing at the base matched
5. `plugins/importer-insomnia/src/templates.ts:49` new, `function convertTag(name: string, input: string, requestIds: Set<string>): string | null {`, values `{"cc":23,"lines":37}`, ceiling cc 9, lines 52, nothing at the base matched
6. `plugins/importer-insomnia/tests/migration.test.ts:7` new, `test("imports GraphQL, credentials and forward response references together", () => {`, values `{"cc":2,"lines":88}`, ceiling cc 9, lines 52, nothing at the base matched
7. `crates/yaak/src/import.rs:201` worsened, `pub fn plan_import_resources(`, values `{"cc":30,"lines":258}`, ceiling cc 9, lines 52, base site `crates/yaak/src/import.rs:200` with `{"cc":29,"lines":254}`
8. `crates/yaak/src/import.rs:890` worsened, `fn merge_with_linked_source(`, values `{"cc":69,"lines":331}`, ceiling cc 9, lines 52, base site `crates/yaak/src/import.rs:885` with `{"cc":68,"lines":324}`
9. `plugins/importer-insomnia/src/common.ts:62` worsened, `function importHttpBody(rawBody: any) {`, values `{"cc":25,"lines":86}`, ceiling cc 9, lines 52, base site `plugins/importer-insomnia/src/common.ts:60` with `{"cc":19,"lines":57}`
10. `plugins/importer-insomnia/src/v4.ts:12` worsened, `export function convertInsomniaV4(parsed: any) {`, values `{"cc":6,"lines":64}`, ceiling cc 9, lines 52, base site `plugins/importer-insomnia/src/v4.ts:12` with `{"cc":6,"lines":61}`
11. `plugins/importer-insomnia/src/v5.ts:13` worsened, `export function convertInsomniaV5(parsed: any) {`, values `{"cc":12,"lines":75}`, ceiling cc 9, lines 52, base site `plugins/importer-insomnia/src/v5.ts:13` with `{"cc":12,"lines":72}`
12. `plugins/template-function-response/src/index.ts:282` worsened, `async function getResponse(`, values `{"cc":14,"lines":48}`, ceiling cc 9, lines 52, base site `plugins/template-function-response/src/index.ts:280` with `{"cc":14,"lines":47}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R076

- Repository: `mountain-loop/yaak` (TypeScript), change 5 of 10
- Commit: `34815f327a73`, judged against its first parent `6fc43a60e23b`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> Fix Insomnia template and GraphQL import fidelity (#693)

### Files the change touched

```text
 crates/yaak/src/import.rs                                      | 109 ++++++++++++++++++++++++++++++-
 crates/yaak/src/import_templates.rs                            | 172 +++++++++++++++++++++++++++++++++++++++++++++++++
 crates/yaak/src/lib.rs                                         |   1 +
 plugins/importer-insomnia/src/common.ts                        | 155 ++++++++++++++++++++++++++++++++++++++------
 plugins/importer-insomnia/src/templates.ts                     | 113 ++++++++++++++++++++++++++++++++
 plugins/importer-insomnia/src/v4.ts                            |  11 ++--
 plugins/importer-insomnia/src/v5.ts                            |   7 +-
 plugins/importer-insomnia/tests/body.test.ts                   |  72 +++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/basic.output.json     |   2 +-
 plugins/importer-insomnia/tests/fixtures/chained.input.yaml    |  29 +++++++++
 plugins/importer-insomnia/tests/fixtures/chained.output.json   |  85 ++++++++++++++++++++++++
 plugins/importer-insomnia/tests/fixtures/version-5.output.json |   6 +-
 plugins/importer-insomnia/tests/migration.test.ts              |  95 +++++++++++++++++++++++++++
 plugins/importer-insomnia/tests/templates.test.ts              |  88 +++++++++++++++++++++++++
 plugins/template-function-response/src/index.ts                |  11 ++--
 15 files changed, 921 insertions(+), 35 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `crates/yaak/src/import.rs:2616` new, `async fn insomnia_plugin_output_produces_sendable_graphql_and_linked_requests() {`, values `{"dead":1}`, nothing at the base matched

   ```text
   2613 |     }
   2614 | 
   2615 |     #[tokio::test]
   2616 |     async fn insomnia_plugin_output_produces_sendable_graphql_and_linked_requests() {
   2617 |         // The JS fixture suite asserts this exact payload against the real importer.
   2618 |         let imported: yaak_plugins::events::ImportResponse = serde_json::from_str(include_str!(
   2619 |             "../../../plugins/importer-insomnia/tests/fixtures/chained.output.json"
   2620 |         ))
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R077

- Repository: `mountain-loop/yaak` (TypeScript), change 6 of 10
- Commit: `6fc43a60e23b`, judged against its first parent `23d369d84e26`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile; cc at a derived percentile, the base site already over; lines at a derived percentile; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,345 functions at 23d369d, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 8,345 functions at 23d369d, floor 25; recorded scope: whole repository

### Commit message

> Make template tag scanners quote-aware (#710)

### Files the change touched

```text
 apps/yaak-client/components/core/Editor/json-lint.test.ts  |  46 ++++++++++++++
 apps/yaak-client/components/core/Editor/json-lint.ts       |   5 +-
 apps/yaak-client/components/core/Editor/twig/highlight.ts  |   2 +
 apps/yaak-client/components/core/Editor/twig/twig.grammar  |   6 +-
 apps/yaak-client/components/core/Editor/twig/twig.terms.ts |   5 +-
 apps/yaak-client/components/core/Editor/twig/twig.test.ts  |  48 +++++++++++++++
 apps/yaak-client/components/core/Editor/twig/twig.ts       |  21 ++++---
 apps/yaak-client/lib/resolvedModelName.test.ts             |  18 ++++++
 apps/yaak-client/lib/resolvedModelName.ts                  |  10 +--
 apps/yaak-client/lib/templateTags.test.ts                  | 176 +++++++++++++++++++++++++++++++++++++++++++++++++++++
 apps/yaak-client/lib/templateTags.ts                       | 137 +++++++++++++++++++++++++++++++++++++++++
 apps/yaak-client/lib/validateHttpHeader.test.ts            |  12 ++++
 apps/yaak-client/lib/validateHttpHeader.ts                 |   8 +--
 crates/yaak-templates/src/format_json.rs                   |  43 ++++++++++++-
 crates/yaak-templates/src/lib.rs                           |   1 +
 crates/yaak-templates/src/parser.rs                        |  15 +++--
 crates/yaak-templates/src/strip_json_comments.rs           |  38 ++++++++++++
 crates/yaak-templates/src/tag_scan.rs                      | 114 ++++++++++++++++++++++++++++++++++
 plugins/template-function-request/src/index.ts             |  81 +++++++++++++++++++++++-
 19 files changed, 747 insertions(+), 39 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 53 lines).

1. `apps/yaak-client/lib/templateTags.ts:59` new, `export function findTemplateTags(text: string): TemplateTagMatch[] {`, values `{"cc":12,"lines":61}`, ceiling cc 9, lines 53, nothing at the base matched

   ```text
   56 |  * A `${[` with no `]}` after it isn't a tag, and neither is a `${[` nested inside a tag body —
   57 |  * both are ordinary content, same as the regex this replaced treated them.
   58 |  */
   59 | export function findTemplateTags(text: string): TemplateTagMatch[] {
   60 |   const matches: TemplateTagMatch[] = [];
   61 | 
   62 |   // Both built only once a scan actually needs them, so short values allocate nothing
   63 |   let closingQuotes: Int32Array | null = null;
   64 | 
   65 |   /*
   66 |    * Body positions a previous scan already ran off the end of the text from. Where a scan ends
   67 |    * up depends only on where it starts, so a scan that walks into one of these would repeat
   68 |    * that same walk and fail the same way, and can stop right there.
   69 |    *
   70 |    * That bound is what keeps the whole pass linear: every position enters this set at most
   71 |    * once, and every step of every scan either visits a position for the first time or ends
   72 |    * that scan.
   73 |    */
   74 |   let deadEnds: Set<number> | null = null;
   75 | 
   76 |   let searchFrom = 0;
   77 |   while (searchFrom < text.length) {
   78 |     const start = text.indexOf("${[", searchFrom);
   79 |     if (start < 0) break;
   80 | 
   81 |     const visited: number[] = [];
   82 |     let end = -1;
   83 |     let i = start + 3;
   84 |     while (i < text.length) {
   85 |       if (deadEnds?.has(i)) break;
   86 |       visited.push(i);
   87 | 
   88 |       const c = text.charCodeAt(i);
   89 |       if (c === QUOTE) {
   90 |         closingQuotes ??= closingQuoteTable(text);
   91 |         const close = closingQuotes[i + 1] ?? -1;
   92 |         // An unterminated quote is an ordinary character, so only step over a closed string
   93 |         i = close < 0 ? i + 1 : close + 1;
   94 |         continue;
   95 |       }
   96 | 
   97 |       if (c === CLOSE_BRACKET && text.charCodeAt(i + 1) === CLOSE_BRACE) {
   98 |         end = i + 2;
   99 |         break;
   ```

2. `plugins/template-function-request/src/index.ts:284` new, `function replaceTemplateTags(text: string, replace: (inner: string) => string): string {`, values `{"cc":12,"lines":49}`, ceiling cc 9, lines 53, nothing at the base matched

   ```text
   281 |  * `apps/yaak-client/lib/templateTags.ts`, which carries the full explanation — including why
   282 |  * this is a linear hand-written scan rather than a regex.
   283 |  */
   284 | function replaceTemplateTags(text: string, replace: (inner: string) => string): string {
   285 |   let closingQuotes: Int32Array | null = null;
   286 |   let deadEnds: Set<number> | null = null;
   287 | 
   288 |   let result = "";
   289 |   let copiedTo = 0;
   290 |   let searchFrom = 0;
   291 |   while (searchFrom < text.length) {
   292 |     const start = text.indexOf("${[", searchFrom);
   293 |     if (start < 0) break;
   294 | 
   295 |     const visited: number[] = [];
   296 |     let end = -1;
   297 |     let i = start + 3;
   298 |     while (i < text.length) {
   299 |       if (deadEnds?.has(i)) break;
   300 |       visited.push(i);
   301 | 
   302 |       const c = text.charCodeAt(i);
   303 |       if (c === QUOTE) {
   304 |         closingQuotes ??= closingQuoteTable(text);
   305 |         const close = closingQuotes[i + 1] ?? -1;
   306 |         i = close < 0 ? i + 1 : close + 1;
   307 |         continue;
   308 |       }
   309 | 
   310 |       if (c === CLOSE_BRACKET && text.charCodeAt(i + 1) === CLOSE_BRACE) {
   311 |         end = i + 2;
   312 |         break;
   313 |       }
   314 | 
   315 |       i += 1;
   316 |     }
   317 | 
   318 |     if (end < 0) {
   319 |       // Not a tag, but a real one may start inside a string this scan stepped over
   320 |       deadEnds ??= new Set();
   321 |       for (const position of visited) deadEnds.add(position);
   322 |       searchFrom = start + 3;
   323 |       continue;
   324 |     }
   ```

3. `crates/yaak-templates/src/format_json.rs:10` worsened, `pub fn format_json(text: &str, tab: &str) -> String {`, values `{"cc":54,"lines":217}`, ceiling cc 9, lines 53, base site `crates/yaak-templates/src/format_json.rs:8` with `{"cc":52,"lines":209}`

   ```text
   07 | }
   08 | 
   09 | /// Formats JSON that might contain template tags (skipped entirely)
   10 | pub fn format_json(text: &str, tab: &str) -> String {
   11 |     let mut chars = text.chars().peekable();
   12 |     let mut tag_strings = TagStrings::default();
   13 | 
   14 |     let mut new_json = "".to_string();
   15 |     let mut depth = 0;
   16 |     let mut state = FormatState::None;
   17 |     let mut saw_newline_in_whitespace = false;
   18 | 
   19 |     loop {
   20 |         let rest_of_chars = chars.clone();
   21 |         let current_char = match chars.next() {
   22 |             None => break,
   23 |             Some(c) => c,
   24 |         };
   25 | 
   26 |         // Handle JSON string states
   27 |         if let FormatState::String = state {
   28 |             match current_char {
   29 |                 '"' => {
   30 |                     state = FormatState::None;
   31 |                     new_json.push(current_char);
   32 |                     continue;
   33 |                 }
   34 |                 '\\' => {
   35 |                     new_json.push(current_char);
   36 |                     if let Some(c) = chars.next() {
   37 |                         new_json.push(c);
   38 |                     }
   39 |                     continue;
   40 |                 }
   41 |                 _ => {
   42 |                     new_json.push(current_char);
   43 |                     continue;
   44 |                 }
   45 |             }
   46 |         }
   47 |         // Close Template tag states
   48 |         if let FormatState::TemplateTag = state {
   49 |             // A quoted argument is allowed to contain `]}`, so skip over strings whole
   50 |             if current_char == '\'' {
   ```

4. `crates/yaak-templates/src/strip_json_comments.rs:20` worsened, `pub fn strip_json_comments(text: &str) -> String {`, values `{"cc":30,"lines":112}`, ceiling cc 9, lines 53, base site `crates/yaak-templates/src/strip_json_comments.rs:18` with `{"cc":28,"lines":104}`

### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## R078

- Repository: `mountain-loop/yaak` (TypeScript), change 7 of 10
- Commit: `23d369d84e26`, judged against its first parent `6e7f23a778e1`, exit 1
- Gate: dead-symbols, FAIL
- Decision group: dead-symbols

### Commit message

> Print template strings in plain quotes unless they can't round-trip (#708)

### Files the change touched

```text
 apps/yaak-client/components/HeadersEditor.tsx   | 11 +-------
 apps/yaak-client/lib/resolvedModelName.test.ts  | 51 ++++++++++++++++++++++++++++++++++++
 apps/yaak-client/lib/resolvedModelName.ts       |  9 ++++++-
 apps/yaak-client/lib/validateHttpHeader.test.ts | 32 +++++++++++++++++++++++
 apps/yaak-client/lib/validateHttpHeader.ts      | 15 +++++++++++
 crates/yaak-templates/src/parser.rs             | 92 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++------
 6 files changed, 191 insertions(+), 19 deletions(-)
```

### Findings

Condition: where no reference named the declaration exists outside its own declaration.

1. `apps/yaak-client/lib/resolvedModelName.test.ts:9` new, `const { resolvedModelName } = await import("./resolvedModelName");`, values `{"dead":1}`, nothing at the base matched

   ```text
   06 | vi.mock("@yaakapp-internal/models", () => ({ foldersAtom: {} }));
   07 | vi.mock("./jotai", () => ({ jotaiStore: { get: () => [] } }));
   08 | 
   09 | const { resolvedModelName } = await import("./resolvedModelName");
   10 | 
   11 | function httpRequest(url: string): HttpRequest {
   12 |   return { id: "rq_test", model: "http_request", name: "", url } as HttpRequest;
   13 | }
   ```


### Remedy klin printed

> Delete the declaration if the refactor made it obsolete, or restore a real reference to it.

## R079

- Repository: `mountain-loop/yaak` (TypeScript), change 9 of 10
- Commit: `b03c41b767ca`, judged against its first parent `a0d71eb486ee`, exit 1
- Gate: complexity, FAIL
- Decision group: cc at a derived percentile, the base site already over; lines at a derived percentile, the base site already over
- Derived cc: `9`, 95th percentile of 8,322 functions at a0d71eb, floor 5; recorded scope: whole repository
- Derived lines: `53`, 95th percentile of 8,322 functions at a0d71eb, floor 25; recorded scope: whole repository

### Commit message

> chore(deps): bump git2 from 0.20.4 to 0.21.0 (#703)
>
> Signed-off-by: dependabot[bot] <support@github.com>
> Co-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>
> Co-authored-by: Gregory Schier <gschier1990@gmail.com>

### Files the change touched

```text
 Cargo.lock                     | 35 ++++++-----------------------------
 crates/yaak-git/Cargo.toml     |  2 +-
 crates/yaak-git/src/log.rs     |  6 +++---
 crates/yaak-git/src/pull.rs    |  9 +++++----
 crates/yaak-git/src/push.rs    |  9 +++++----
 crates/yaak-git/src/remotes.rs |  7 ++++---
 crates/yaak-git/src/status.rs  | 10 +++++-----
 crates/yaak-git/src/util.rs    |  3 ++-
 8 files changed, 31 insertions(+), 50 deletions(-)
```

### Findings

Condition: over the complexity gate (cyclomatic > 9 or body > 53 lines).

1. `crates/yaak-git/src/pull.rs:33` worsened, `pub async fn git_pull(dir: &Path) -> Result<PullResult> {`, values `{"cc":21,"lines":82}`, ceiling cc 9, lines 53, base site `crates/yaak-git/src/pull.rs:33` with `{"cc":20,"lines":81}`

   ```text
   30 |     Ok(statuses.iter().any(|e| e.status() != git2::Status::CURRENT))
   31 | }
   32 | 
   33 | pub async fn git_pull(dir: &Path) -> Result<PullResult> {
   34 |     if has_uncommitted_changes(dir)? {
   35 |         return Ok(PullResult::UncommittedChanges);
   36 |     }
   37 | 
   38 |     // Extract all git2 data before any await points (git2 types are not Send)
   39 |     let (branch_name, remote_name, remote_url) = {
   40 |         let repo = open_repo(dir)?;
   41 |         let branch_name = get_current_branch_name(&repo)?;
   42 |         let remote = get_default_remote_in_repo(&repo)?;
   43 |         let remote_name = remote
   44 |             .name()?
   45 |             .ok_or(GenericError("Failed to get remote name".to_string()))?
   46 |             .to_string();
   47 |         let remote_url = remote.url()?.to_string();
   48 |         (branch_name, remote_name, remote_url)
   49 |     };
   50 | 
   51 |     // Step 1: fetch the specific branch
   52 |     // NOTE: We use fetch + merge instead of `git pull` to avoid conflicts with
   53 |     // global git config (e.g. pull.ff=only) and the background fetch --all.
   54 |     let fetch_out = new_binary_command(dir)
   55 |         .await?
   56 |         .args(["fetch", &remote_name, &branch_name])
   57 |         .env("GIT_TERMINAL_PROMPT", "0")
   58 |         .output()
   59 |         .await
   60 |         .map_err(|e| GenericError(format!("failed to run git fetch: {e}")))?;
   61 | 
   62 |     let fetch_stdout = String::from_utf8_lossy(&fetch_out.stdout);
   63 |     let fetch_stderr = String::from_utf8_lossy(&fetch_out.stderr);
   64 |     let fetch_combined = format!("{fetch_stdout}{fetch_stderr}");
   65 | 
   66 |     info!("Fetched status={} {fetch_combined}", fetch_out.status);
   67 | 
   68 |     if fetch_combined.to_lowercase().contains("could not read") {
   69 |         return Ok(PullResult::NeedsCredentials { url: remote_url.to_string(), error: None });
   70 |     }
   71 | 
   72 |     if fetch_combined.to_lowercase().contains("unable to access") {
   73 |         return Ok(PullResult::NeedsCredentials {
   ```

2. `crates/yaak-git/src/push.rs:20` worsened, `pub async fn git_push(dir: &Path) -> Result<PushResult> {`, values `{"cc":19,"lines":71}`, ceiling cc 9, lines 53, base site `crates/yaak-git/src/push.rs:20` with `{"cc":18,"lines":70}`

   ```text
   17 |     NeedsCredentials { url: String, error: Option<String> },
   18 | }
   19 | 
   20 | pub async fn git_push(dir: &Path) -> Result<PushResult> {
   21 |     // Extract all git2 data before any await points (git2 types are not Send)
   22 |     let (branch_name, remote_name, remote_url) = {
   23 |         let repo = open_repo(dir)?;
   24 |         let branch_name = get_current_branch_name(&repo)?;
   25 |         let remote = get_default_remote_for_push_in_repo(&repo)?;
   26 |         let remote_name = remote
   27 |             .name()?
   28 |             .ok_or(GenericError("Failed to get remote name".to_string()))?
   29 |             .to_string();
   30 |         let remote_url = remote.url()?.to_string();
   31 |         (branch_name, remote_name, remote_url)
   32 |     };
   33 | 
   34 |     let out = new_binary_command(dir)
   35 |         .await?
   36 |         .args(["push", &remote_name, &branch_name])
   37 |         .env("GIT_TERMINAL_PROMPT", "0")
   38 |         .output()
   39 |         .await
   40 |         .map_err(|e| GenericError(format!("failed to run git push: {e}")))?;
   41 | 
   42 |     let stdout = String::from_utf8_lossy(&out.stdout);
   43 |     let stderr = String::from_utf8_lossy(&out.stderr);
   44 |     let combined = stdout + stderr;
   45 |     let combined_lower = combined.to_lowercase();
   46 | 
   47 |     info!("Pushed to repo status={} {combined}", out.status);
   48 | 
   49 |     // Helper to check if this is a credentials error
   50 |     let is_credentials_error = || {
   51 |         combined_lower.contains("could not read")
   52 |             || combined_lower.contains("unable to access")
   53 |             || combined_lower.contains("authentication failed")
   54 |     };
   55 | 
   56 |     // Check for explicit rejection indicators first (e.g., protected branch rejections)
   57 |     // These can occur even if some git servers don't properly set exit codes
   58 |     if combined_lower.contains("rejected") || combined_lower.contains("failed to push") {
   59 |         if is_credentials_error() {
   60 |             return Ok(PushResult::NeedsCredentials {
   ```

3. `crates/yaak-git/src/status.rs:172` worsened, `fn git_branch_info_for_repo(`, values `{"cc":11,"lines":32}`, ceiling cc 9, lines 53, base site `crates/yaak-git/src/status.rs:172` with `{"cc":10,"lines":32}`

   ```text
   169 |     })
   170 | }
   171 | 
   172 | fn git_branch_info_for_repo(
   173 |     repo: &git2::Repository,
   174 |     dir: &Path,
   175 | ) -> crate::error::Result<GitBranchInfo> {
   176 |     let (head_ref, head_ref_shorthand) = git_head_refs(repo);
   177 |     let origins = repo.remotes()?.into_iter().filter_map(|o| Some(o.ok()??.to_string())).collect();
   178 |     let local_branches = local_branch_names(repo)?;
   179 |     let remote_branches = remote_branch_names(repo)?;
   180 | 
   181 |     // Compute ahead/behind relative to remote tracking branch
   182 |     let (ahead, behind) = (|| -> Option<(usize, usize)> {
   183 |         let head = repo.head().ok()?;
   184 |         let local_oid = head.target()?;
   185 |         let branch_name = head.shorthand().ok()?;
   186 |         let upstream_ref =
   187 |             repo.find_branch(&format!("origin/{branch_name}"), git2::BranchType::Remote).ok()?;
   188 |         let upstream_oid = upstream_ref.get().target()?;
   189 |         repo.graph_ahead_behind(local_oid, upstream_oid).ok()
   190 |     })()
   191 |     .unwrap_or((0, 0));
   192 | 
   193 |     Ok(GitBranchInfo {
   194 |         path: dir.to_string_lossy().to_string(),
   195 |         head_ref,
   196 |         head_ref_shorthand,
   197 |         origins,
   198 |         local_branches,
   199 |         remote_branches,
   200 |         ahead: ahead as u32,
   201 |         behind: behind as u32,
   202 |     })
   203 | }
   204 | 
   ```


### Remedy klin printed

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.
