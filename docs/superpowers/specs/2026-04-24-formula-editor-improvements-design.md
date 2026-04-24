# Formula editor improvements

Three additions to the column-formula editor surface:

1. Syntax-help tooltip on the `fx` label.
2. Autocomplete popover for `{ColumnName}` placeholders.
3. Error dot (debounced) showing the evalexpr error on hover.

A narrow change to `evaluate_column_formula` is bundled: unknown placeholder
names become errors instead of silently coercing to `0`, so the error dot
catches typos.

## Current state

- Editor lives in `src/ui/formula_bar.rs` and is shown when
  `TableApp::editing_formula_col` is `Some(col)`.
- Bar layout today: `fx` label · column name · `TextInput` · Apply · Cancel.
- Formula evaluation: `data::formula::evaluate_column_formula(sheet, expr, row_idx)`
  substitutes `{ColumnName}` with the numeric cell value (or `0.0` if the
  column name is unknown, the cell is empty, or the cell is non-numeric), then
  evaluates the result with `evalexpr::eval`.
- Errors returned today: evalexpr parse/eval failures. Unknown column names
  are silently replaced by `0` and never surface.

## 1. Syntax-help tooltip

Wrap the existing `fx` label in `iced_longbridge::components::hover_card`.
The hover card contains a small column of text rows listing placeholder
syntax, supported operators, and two example formulas. Width is capped by
the existing `hover_card` style (`max_width(320)`).

No state. No messages. Pure view code in `formula_bar.rs`.

Copy (authoritative):

```
Formula syntax

  {ColumnName}   value of that column in this row
  + - * /        arithmetic
  ( )            grouping

Examples
  {Price} * {Quantity}
  ({Value} + 10) / 2
```

## 2. Autocomplete popover

### Trigger

iced's `TextInput` does not expose cursor position, so the trigger is
string-based: scan the formula for the last unclosed `{` (i.e. a `{`
followed by characters that contain no `}`). If found *and*
`formula_autocomplete_suppressed == false`, the substring after that `{`
is the active *partial* and the popover is open. Otherwise it is closed.

The column being edited (`editing_formula_col`) is excluded from the
suggestion set — a formula cannot reference itself.

Matching is case-insensitive `starts_with` against the partial, preserving
sheet column order. No fuzzy matching.

### Widget

`popover::popover_dismissable` wraps the formula `TextInput`. The panel is
`Some(list)` when the suggestion set is non-empty and the popover is not
suppressed; otherwise `None`. `FloatingPanel` hides itself when its panel
is `None`, so no open/closed flag is stored in app state.

The `on_dismiss` message (fired when the user clicks outside the panel) is
`FormulaEscape` — same handler as the Escape key.

Each suggestion row is a clickable button spanning the panel width,
rendering the column name. The currently-highlighted row is drawn with the
`muted` background; others are transparent.

### Keyboard

Key routing is split across three surfaces because iced's `TextInput`
intercepts some keys before the app-level `event::listen_with` handler
sees a useful status:

- **Arrows and Tab** — handled in `handle_key_event` via a new early
  branch that fires whenever `editing_formula_col.is_some()` and the
  suggestion set is non-empty, *regardless of* `event::Status`. Arrow
  keys normally come through as `Status::Captured` (text input moves its
  caret), so the existing `Status::Ignored` gate has to be bypassed for
  this case.
  - `ArrowDown` / `ArrowUp` → `FormulaSuggestionMove(±1)`
  - `Tab` → `FormulaSuggestionAccept`
- **Enter** — already routed to `FormulaEditCommit` via the text input's
  `on_submit`. Reinterpret inside the `FormulaEditCommit` arm of
  `update`: if the suggestion set is non-empty, perform accept-suggestion
  instead of committing the formula. No new message needed.
- **Escape** — add a `FormulaEscape` message. In `handle_key_event`,
  route `Escape` to `FormulaEscape` when `editing_formula_col.is_some()`
  (taking precedence over the existing `CellEditCancel` mapping). In
  `update`, `FormulaEscape` sets `formula_autocomplete_suppressed = true`
  when suggestions are showing; otherwise it is a no-op (closing the
  editor stays on the Cancel button, matching current behavior).

This means arrow-key caret movement inside the text input is overridden
while the popover is open. Standard IDE behavior.

### Accept behavior

Accepting `{Val` against suggestion `Value`:

- Find the last unclosed `{` in the formula string — call its index `i`.
- Replace the substring from `i` to end with `{Value}`.
- Clear the "suppress" flag.

The trailing `}` is inserted automatically. Cursor position is not
controlled (iced limitation) — the caret lands at the end of the string,
which is where users want it for typical append-composition.

### State additions (`TableApp`)

- `formula_suggestions_selected: usize` — highlighted index; clamped to
  `0..suggestions.len()` on `FormulaSuggestionMove` (no wrap-around); reset
  to `0` in `FormulaChanged` whenever the set of suggestion names changes.
- `formula_autocomplete_suppressed: bool` — set by `FormulaEscape`, cleared
  by any `FormulaChanged` (so the next keystroke re-opens the popover).

Suggestion set and `is_open` are computed on the fly in `formula_bar::view`
from the current `editing_formula_value` + `sheet.columns` — no cached
copy.

### New `Message` variants

- `FormulaSuggestionMove(i32)`
- `FormulaSuggestionAccept`
- `FormulaSuggestionClick(usize)` — mouse click on row `n`; sets selected
  index then performs accept
- `FormulaEscape` — Escape-key handler while the formula editor is open

## 3. Error dot

### Placement

Between the input and the Apply button, in a fixed-width
(`Length::Fixed(14.0)`) container so the Apply button does not jitter
when the dot appears and disappears. The dot itself is an 8px circle drawn
via a `container` with equal radius, filled with `theme.destructive`.

Wrapped in `tooltip::wrap` whose label is:

```
Formula didn't evaluate: <raw error message>
```

The raw message comes from `evaluate_column_formula`'s `Err(String)` arm.

The dot is only rendered when `formula_error.is_some()`; when it is `None`
an empty fixed-width spacer takes its place.

### Debounce

Re-use the existing 250 ms `NotifyTick` subscription rather than adding a
new timer. State:

- `formula_last_edit: Option<Instant>` — set in `FormulaChanged` to
  `Some(Instant::now())`.
- `formula_error: Option<String>` — set by the tick handler.
- `formula_error_checked_for: Option<String>` — the formula string we last
  ran; prevents re-evaluation on every tick after it settles.

In the `NotifyTick` handler, if `editing_formula_col.is_some()` and
`formula_last_edit.elapsed() >= 300ms` and the current
`editing_formula_value` differs from `formula_error_checked_for`: evaluate
and update. Empty formulas are treated as "no error" (clear
`formula_error`).

This does mean the first evaluation happens between 300 ms and 550 ms
after the user stops typing (one full tick interval of slack). That is
fine for a correctness indicator.

### Evaluation target

`evaluate_column_formula` is row-indexed. Evaluate against `row 0` if the
sheet has any rows; skip (and clear error) if the sheet is empty. Users
see structural errors (unknown column name, unbalanced parens, unknown
operator) which are what the dot is meant to signal.

On `FormulaEditCommit` and `FormulaEditCancel`, clear `formula_error`,
`formula_last_edit`, and `formula_error_checked_for` — the editor is
closing.

## 4. Evaluation change — unknown placeholders as errors

In `data::formula::evaluate_column_formula`, change:

```rust
let value = sheet
    .columns
    .iter()
    .position(|c| c.name == col_name)
    .and_then(|ci| row.get(ci))
    .and_then(|cell| cell.as_f64())
    .unwrap_or(0.0);
```

to resolve the column position first, and return `Err(format!("Unknown
column: {col_name}"))` when no column of that name exists. When the
column exists but the cell is non-numeric or empty, keep the silent-zero
behavior — that is a legitimate in-progress state (partially-filled
sheets) and not a typo signal.

Touches only the inside of the `for caps in placeholder_re.captures_iter`
loop. No signature change.

## 5. Files changed

- `src/ui/formula_bar.rs` — hover card, popover wrap, error dot, suggestion
  list view.
- `src/main.rs` — new `Message` variants, new `TableApp` fields, keyboard
  interception for popover navigation, tick-driven error evaluation,
  accept-suggestion logic.
- `src/data/formula.rs` — unknown-placeholder error path.

## 6. Out of scope

- Cursor-aware autocomplete (requires forking iced's `TextInput`).
- Fuzzy matching of suggestions.
- Function-name autocomplete (`SUM`, `AVG`, …) — only `{ColumnName}` is
  completed in this pass.
- Per-row error indication in the data grid itself (the existing `#EVAL`
  token stays as-is).
- Syntax highlighting inside the formula field.
