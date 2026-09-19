# Security policy

This project is an experimental compiler foundation. It has not completed a security audit or the full authorization design. No production-support or security-response service level is offered yet.

## Current boundary

The compiler reads source and emits a plan; it does not execute plan effects. Source files cannot supply the trusted engine host identity. The current source grammar creates public nodes and edges and cannot express private visibility policies. The engine is responsible for independent authorization, storage, adapter effects and restrictions propagated through derived results. A locally trusted CLI flag is not a deployed authentication mechanism.

Parser and expression limits reduce some accidental resource use; they are not a complete service sandbox. Do not run untrusted compilation as an unbounded privileged service. Literal source text, properties and identifiers remain data, not executable instructions.

## Reporting

Private vulnerability reporting and a dedicated security contact have not yet been configured for the intended GitHub organization. Do not assume a private advisory channel exists. If GitHub private reporting becomes available on the repository, use that channel; otherwise use an already-established private maintainer channel. Public issues should contain only a sanitized description and synthetic reproduction, never secrets or exploitable private deployment details.

Include the affected commit/protocol version, reproduction, expected versus observed behavior, and potential disclosure or integrity impact. Coordinated reports involving both language and engine should identify both revisions. Confirmed fixes need regression tests and release notes; no unsupported response deadline is promised.
