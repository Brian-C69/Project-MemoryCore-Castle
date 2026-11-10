# Provider switching and image jobs

The selector below the companion's name switches the main conversation between Claude Code and Codex. Refresh providers after installing or signing in. The selector uses the installed CLI's own authentication-status command; it does not equate a credential file with a usable session. Model choice remains each CLI's configured default.

## Switching

A live switch queues until an explicit native end-of-turn event. Silence is not an idle signal. Unknown record formats leave the request pending; Cancel switch returns control without terminating the session. Settings uses the same handoff flow.

Castle preserves normalized messages and attachment references in its own application-data directory, partitioned by MemoryCore folder and room. Native CLI records are never rewritten. A bounded handoff carries recent dialogue and the canonical master-memory.md route to a fresh target CLI session. This transfers conversation context, not private model state or an entire context window.

The running session retains its own provider identity. Existing Story sessions continue with their original provider; future sessions use the selected default. Settings cannot move the memory folder while sessions or image work are active.

Main-chat messages submitted during switching/image work are saved in a private queue. Queued messages can be cancelled. Interrupted messages are held after restart and require a deliberate retry after checking the visible history, avoiding blind replay of an action that might already have run. Drafts remain in the composer when acceptance fails.

## Images

Appearance and Studio route image requests to a separate Codex process. Requests are serialized and wait for a safe main-chat boundary. The process receives the image brief and selected image attachments rather than the Castle conversation handoff. It uses normal Codex permissions, strips inherited API keys, and requires ChatGPT authentication. It does not retry through the stored OpenAI API key. Capability, sign-in, permission, or output failures retain a prompt and provide Retry with Codex. Model-generated instructions are not a security sandbox; normal CLI filesystem permissions still apply.

Image-job status and prompts survive restart. Interrupted jobs become saved prompts rather than running automatically. A completed job must produce a changed, decodable image. Existing output is backed up before replacement. No Windows startup entry, shared memory identity, or provider configuration file is changed by these features.

## Verification boundaries

Automated coverage includes explicit end-of-turn detection, bounded handoffs, tool-output exclusion, journal deduplication and separation, corruption preservation, multiline message packaging, and fake-CLI image success/unchanged-output/failure behaviour. Frontend tests verify image routing does not write to the main terminal or fall back to an API.

Browser checks use the actual served header/Appearance components with a mocked Tauri bridge: provider selection/cancellation, output and reference routing, saved-prompt retry, and no main-terminal image writes. This is component verification, not a real model-session smoke test.

Real Claude-to-Codex and Codex-to-Claude handoffs, actual image generation, permissions/sign-in failure UX in a packaged app, and installed-app replacement remain explicit live validation gates. Do not call these end-to-end verified based only on unit tests or a successful build. No paid API request or model inference was used for automated verification.

Reference: [Codex CLI session commands](https://learn.chatgpt.com/docs/cli/reference). Castle uses its own handoff rather than converting another provider's native session to a Codex resume record.
