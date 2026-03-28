## Collaboration

At the start of every session:
1. Check your current phase and task from the project context (ROADMAP.md, active PLAN.md, or recent git log)
2. Run `collab list` and READ the output before doing anything else — treat pending messages as blocking
3. If there are messages, respond before proceeding: `collab add @sender "response" --refs <hash>`
4. Run `collab watch --role "<project>: <your current task>"` with real context, not a leftover or generic description
   Example: `collab watch --role "yubitui: phase 09 OathScreen widget implementation"`
5. Run `collab roster` to see who else is online and what they're working on

When your focus changes, restart watch with an updated --role.

When to message other workers (keep it signal, not noise):
- A public API changed: trait signature, method rename, new required field
  Example: `collab add @yubitui "renamed Widget::render to Widget::draw in widget/mod.rs — update any impl blocks"`
- A new widget or utility they might want to use
- Something that was working changed behavior

Do NOT message for: general progress updates, phase completions, or anything they don't need to act on.
Never message yourself.
