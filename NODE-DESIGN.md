# Native graph node design

## Visual reference

Inspired by the hierarchy and selection language of RenderMax's `qrenderdoc/Windows/RenderGraphViewer.cpp` (PassItem and dependency paths), implemented with GPUI rather than embedding Qt or depending on RenderMax.

- Quiet rounded surfaces, restrained shadows and a narrow category accent rail.
- Category/icon header, wrapped title, source filename, details, separated source actions.
- Main Skill: violet; subflow: teal; phase/step: blue. References retain their file-role colors.
- Flow cards are 320 world units wide and 214–290 high depending on semantic fields; file cards are 264 × 188. Automatic spacing accounts for card height; existing saved file positions are not rewritten.
- At distant Flow zoom (<38%), only category and title remain, avoiding illegible metadata.

## Semantic information

- Action title removes Phase/Step numbering, retaining the source wording. An English Skill title may use an explicitly stated Chinese mission as an extractive title; this is not model paraphrasing or translation.
- Prefer explicit mission/purpose sections. Ordinary paragraph fallback is labelled 原文, not presented as an authored purpose.
- Show condition and user-confirmation tags only when supported by the heading/body. Negated confirmation statements such as 无需用户确认 do not become confirmation gates.
- Input/output appear only when explicitly labelled or in matching sections; example code is not treated as a real contract.
- Every field retains source line evidence. A call inheriting its stage's condition navigates to the caller, while its purpose and contracts navigate to the target document.
- Entry-level semantics stop before explicit numbered stages to avoid attributing a later step's output to the whole Skill.
- Dynamic heights are used by layout, ports, arrows and fit-to-view; metadata moves below the purpose instead of dominating the card.

## Selection

- Clicked Flow cards are keyed by their graph-instance ID, not document path. Repeated calls to one document can therefore be selected independently.
- Selected Flow card: warm amber 2px border, raised surface and SELECTED indicator.
- Incoming edges: amber (`#ffb800`); outgoing edges: green (`#1fe482`). Connected strokes and ports become heavier with a subtle halo.
- Non-incident edges become quieter rather than competing with the selected path.
- References use the same direction-dependent edge styling and a warm card border; the Overview navigator continues to reflect selected files.
- Flow background click clears selection. Returning via breadcrumbs restores the previous card selection along with the camera.

Selection colors indicate focus and direction, NOT execution status, success/failure or confirmed control flow. Candidate Flow edges remain dashed.

## Preserved actions

- Single-click main/sub Skill opens its Markdown; single-click stage locates the source line.
- Double-click sub Skill enters its graph. No embedded collapse/expand.
- Source and call-site footer buttons both select the associated card and open the relevant Markdown location.
- Right-drag camera panning does not select cards, mutate file contents or save node positions.

## Code

- `gpui/src/ui/graph/flow.rs`: flow card hierarchy, per-instance selection, camera navigation.
- `gpui/src/ui/graph.rs`: file cards and reference links.
- `gpui/src/ui/graph/visual.rs`: shared directional selection styles and strokes/ports.
