# Security Policy

## Reporting a Vulnerability

Do **not** open a public GitHub issue for security vulnerabilities that could be exploited, or that contain sensitive details.

### Temporary reporting mechanism

TX4 does not yet have a dedicated private security email address published.

Until a dedicated security contact is established:

1. Prefer a **private** maintainer contact path once the project publishes one (for example, GitHub Security Advisories on the public repository when enabled).
2. If GitHub Security Advisories are available for this repository, use **Report a vulnerability** on the repository Security tab.
3. If neither a dedicated email nor Security Advisories is available yet, contact a repository maintainer privately through an already-established private channel. Do not invent or guess email addresses.

Do not fabricate a security@ address for this project.

## What to Include

When reporting, include as much of the following as is safe:

* description of the issue
* affected component or version / commit
* reproduction steps
* impact assessment
* whether a proof of concept exists (describe carefully; do not publish exploits publicly)
* suggested remediation, if any

## Responsible Disclosure Expectations

* Give maintainers reasonable time to investigate and remediate before public disclosure.
* Do not intentionally access data that is not yours.
* Do not degrade service or attempt destructive testing against production systems you do not operate.
* Avoid including secrets, payment credentials, or personal data in reports unless strictly necessary; redact where possible.

## Public Issues

Public GitHub issues should **not** contain:

* exploit details that enable abuse
* live credentials or secrets
* private keys
* customer or payment data
* other sensitive vulnerability details

Use public issues for non-sensitive bugs and documentation problems only.

## Supported Versions

Supported-version policy is a **placeholder** until the first public release exists.

| Version | Supported |
| --- | --- |
| Unreleased / main | Best-effort during early development |
| Released versions | Policy TBD after first release |

## Security Response Expectations

Maintainers will aim to:

1. acknowledge receipt when a private report is received
2. assess severity and scope
3. remediate or document risk
4. coordinate disclosure where appropriate
5. credit reporters if they wish to be credited

Exact SLA timelines are not guaranteed during early development.

## Secrets in This Repository

Never commit:

* API keys
* cloud credentials
* payment provider secrets
* private keys
* database passwords
* JWT signing secrets
* personal access tokens

Use placeholders and local environment configuration only.

If you accidentally commit a secret, rotate it immediately and notify maintainers.

## Related

* [GOVERNANCE.md](GOVERNANCE.md)
* [CONTRIBUTING.md](CONTRIBUTING.md)
* [AGENTS.md](AGENTS.md)
