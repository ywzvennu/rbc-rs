# Security Policy

## Supported versions

Only the latest released version on `main` receives security fixes
during pre-1.0 development.

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✓         |
| < 0.1   | ✗         |

## Reporting a vulnerability

Please report suspected security issues privately rather than through a
public issue. Two options:

1. Open a [private security advisory][advisory] on GitHub. This is the
   preferred channel — it creates a confidential thread that the
   maintainer is notified about immediately.
2. Email the address listed on the repository owner's GitHub profile.

Please include:

- A description of the issue and the impact you observed.
- A minimal reproduction (FEN, move sequence, or test case) if
  applicable.
- The crate version or commit you tested against.

You can expect an acknowledgement within a few business days. A fix
or mitigation timeline will be discussed in the private thread. Once
a fix lands, the advisory will be published with credit (or kept
anonymous, your choice).

## Scope

This crate implements RBC game logic only; it does not handle network
input, authentication, or persistent storage. Vulnerabilities in this
crate would typically take the form of panics, denial-of-service via
input parsing (e.g. crafted FEN strings), or divergence from the
specified rules in a way that affects downstream consumers. Reports in
those areas are in scope.

[advisory]: https://github.com/ywzvennu/rbc-rs/security/advisories/new
