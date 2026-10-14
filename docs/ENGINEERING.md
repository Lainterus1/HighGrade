<a id="инженерные-соглашения-high-grade"></a>

# High Grade engineering conventions

Scope: Delivery development in this Project. [ARCHITECTURE](ARCHITECTURE.md) owns structure; [DEVELOPMENT](DEVELOPMENT.md) owns commands. [kit](../kit/rules.md) owns common rules; the [local instruction](../.highgrade/project/INSTRUCTIONS.md) owns local recording/publication authority.

<a id="документы-и-сопровождение"></a>

## Documents and maintenance

Apply [CONTEXT](workflow/CONTEXT.md). New behavior uses one native JSON change; do not duplicate its tasks/requirements in a technical plan. Dependent plans contain only route, links and Next. The user owns material new decisions. Current kit/ procedures own common document/evidence/archive rules. Historical P0–P5 contracts retain checked CLI guarantees.

<a id="компоненты-и-доказательства"></a>

## Components and evidence

Do not merge plugin runtimes/dependencies. SVG Vectorizer/Code Health Audit are checked within their scope. Preserve tests, pins and contracts. [QUALITY](workflow/QUALITY.md) separates structure, transfer accuracy, functionality, installation and acceptance. Core workflow/preservation changes need independent review.

For Rust, keep Cargo.lock and use cargo fmt plus affected behavior tests. For activation, the only mandatory release build is an exact-SHA candidate. Without a candidate, a separate build needs a specific risk. Automatic tests use temp directories. Product projects need an explicitly authorized pilot. Error/unknown is not a successful fallback.

<a id="сохранность"></a>

## Preservation

Preserve unrelated changes. Before destructive conversion, keep recoverable originals unless the user explicitly requested discarding specified changes. Always check operation scope. Without Git, preserve sources/manifests separately. Archiving does not authorize removing functionality/installed plugins.

Never publish secrets, environments or temporary local logs. Selected original evidence follows [storage](EVIDENCE-STORAGE.md). Before publication, check origin/licenses. Commit, push, activation and human acceptance have separate authority. Feedback remains local without automatic sending.

Edit stage instructions in kit/procedures/. python scripts/render-skills.py produces full SKILL.md and refreshes manifest; --check detects drift. Never edit generated bodies manually. global-update replaces owned skills from the active journal, preserves user changes and leaves interrupted-transaction journals for explicit global-recover. Historical bundle/ is not the current workflow source.
