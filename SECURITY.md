# 🔒 Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | ✅ Yes |
| < 1.0   | ❌ No |

---

## Reporting a Vulnerability

**DO NOT OPEN A PUBLIC ISSUE** for security vulnerabilities.

Email: **[Your Security Email]**

Response time: Within 48 hours

---

## Security Features

T-PYTREXT has security built-in at every layer:

| Layer | Feature |
|-------|---------|
| **Rust Core** | AES-256-GCM encryption, SHA-256 hashing, Memory zeroization |
| **Database** | SQLite + AES-256 encryption at rest (PRAGMA key) |
| **Blockchain** | SHA-256 chain with tamper detection |
| **Network** | SSL/TLS via Nginx, rate limiting, security headers |
| **Auth** | bcrypt password hashing, JWT-like tokens, RBAC |
| **Deploy** | Non-root Docker user, systemd hardening, firewall config |
| **Logging** | Automatic redaction of passwords, keys, tokens |

---

## Best Practices for Users

1. **Change `PYTREX_AUTH_SECRET`** in production
2. **Use strong encryption keys** (32+ characters)
3. **Enable SSL/TLS** in production (Let's Encrypt)
4. **Review `deploy/security.yaml`** before deployment
5. **Keep dependencies updated**
6. **Run `pytrex test` before every deploy**

---

## Acknowledgements

We follow OWASP guidelines and industry best practices.
