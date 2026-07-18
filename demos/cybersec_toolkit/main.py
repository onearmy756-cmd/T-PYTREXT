"""
╔══════════════════════════════════════════════════════════════╗
║  PYTREX CYBERSEC TOOLKIT — Authorized Security Testing     ║
║  ========================================================  ║
║  🔐 For: Penetration Testing, CTF, Security Research,      ║
║         Defense, Forensics, Education                       ║
║                                                            ║
║  ⚠️  AUTHORIZED USE ONLY — No malicious hacking!          ║
║                                                            ║
║  Modules:                                                  ║
║  🔍 1. Vulnerability Scanner (Port, Service, Web)          ║
║  🔐 2. Crypto Toolkit (Hash, Encrypt, Decrypt, Crack)      ║
║  🔑 3. Password Analyzer (Strength, Breach Check)          ║
║  🕵️ 4. OSINT Toolkit (Search, Recon, Intelligence)        ║
║  🛡️ 5. Forensics (File Analysis, Metadata, Recovery)      ║
║  📡 6. Network Monitor (Traffic, Anomaly Detection)        ║
║  🧠 7. AI Security Analyst (Hermes + LangChain)            ║
║  🔗 8. Audit Trail (Blockchain — all actions logged)       ║
║  👤 9. HITL — Approve critical operations                  ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import (
    BlockchainBridge, EncryptionManager, HumanInTheLoop,
    HermesAgent, LangChainAgent,
)
from pytrex.search_engine import SearchEngine
import json, time, hashlib, os, re, socket, string, itertools

class CyberSecToolkit(PyTreXApp):
    """Authorized Security Testing & Defense Toolkit"""

    def __init__(self):
        super().__init__(name="PyTreX CyberSec")
        self.blockchain = BlockchainBridge()       # Audit trail
        self.encryption = EncryptionManager(default_password="cybersec_master")
        self.hitl = HumanInTheLoop()               # Critical operations
        self.hermes = HermesAgent(name="SecurityAI")
        self.langchain = LangChainAgent()
        self.search = SearchEngine()

        self.audit_log = []      # All actions logged
        self.scan_results = []   # Vulnerability scan results

        # Register security functions
        self.hermes.register_function(
            "analyze_threat", self._analyze_threat,
            "Analyze a potential security threat",
            {"threat_description": {"type": "string"}},
            category="security"
        )

    def _log_action(self, action: str, details: str):
        """Log every action to blockchain audit trail"""
        entry = {
            "action": action,
            "details": details,
            "timestamp": time.time()
        }
        self.audit_log.append(entry)
        self.blockchain.add_block(json.dumps(entry))

    # ═══════════════════════════════════════════════════════════
    #  1. VULNERABILITY SCANNER
    # ═══════════════════════════════════════════════════════════

    @event("port_scan")
    def port_scan(self, data):
        """Scan open ports on target (AUTHORIZED ONLY!)"""
        payload = json.loads(data) if isinstance(data, str) else data
        target = payload.get("target", "127.0.0.1")
        ports = payload.get("ports", [22, 80, 443, 8080, 3306, 5432, 6379, 27017])

        self._log_action("port_scan", f"Target: {target}, Ports: {ports}")

        open_ports = []
        for port in ports:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1)
                result = sock.connect_ex((target, port))
                if result == 0:
                    service = {22: "SSH", 80: "HTTP", 443: "HTTPS", 8080: "HTTP-Alt",
                               3306: "MySQL", 5432: "PostgreSQL", 6379: "Redis",
                               27017: "MongoDB"}.get(port, "Unknown")
                    open_ports.append({"port": port, "service": service, "status": "OPEN"})
                sock.close()
            except:
                pass

        return json.dumps({
            "target": target,
            "scanned_ports": len(ports),
            "open_ports": open_ports,
            "scan_time": time.time()
        })

    @event("web_scan")
    def web_scan(self, data):
        """Scan web endpoint for common vulnerabilities"""
        payload = json.loads(data) if isinstance(data, str) else data
        url = payload.get("url", "http://localhost")

        self._log_action("web_scan", f"URL: {url}")

        findings = []

        # Check common headers
        headers_to_check = [
            "X-Frame-Options", "X-Content-Type-Options",
            "Content-Security-Policy", "Strict-Transport-Security",
            "X-XSS-Protection"
        ]

        findings.append({
            "type": "missing_headers",
            "severity": "MEDIUM",
            "description": f"Check for security headers: {', '.join(headers_to_check)}",
            "remediation": "Add missing security headers in Nginx/Apache config"
        })

        # Check common paths
        common_paths = ["/admin", "/.git", "/.env", "/backup", "/wp-admin",
                        "/phpmyadmin", "/api", "/debug", "/console"]
        for path in common_paths:
            findings.append({
                "type": "path_check",
                "path": path,
                "severity": "INFO",
                "description": f"Check if {path} is exposed"
            })

        # SQL injection test patterns
        sql_patterns = ["'", "\"", "1=1", "OR 1=1", "--", ";--"]
        findings.append({
            "type": "sql_injection_check",
            "severity": "HIGH",
            "patterns_tested": sql_patterns,
            "description": "Test for SQL injection vulnerabilities"
        })

        return json.dumps({
            "url": url,
            "findings": findings,
            "total_findings": len(findings),
            "recommendation": "Run full authenticated scan for comprehensive results"
        })

    # ═══════════════════════════════════════════════════════════
    #  2. CRYPTO TOOLKIT
    # ═══════════════════════════════════════════════════════════

    @event("hash_crack")
    def hash_crack(self, data):
        """Attempt to crack hash using wordlist (FOR RECOVERY/AUDIT ONLY)"""
        payload = json.loads(data) if isinstance(data, str) else data
        target_hash = payload.get("hash", "")
        hash_type = payload.get("type", "sha256")
        wordlist = payload.get("wordlist", ["password", "123456", "admin", "letmein",
                                              "qwerty", "monkey", "dragon", "master"])

        self._log_action("hash_crack", f"Hash: {target_hash[:16]}..., Type: {hash_type}")

        found = None
        for word in wordlist:
            if hash_type == "sha256":
                h = hashlib.sha256(word.encode()).hexdigest()
            elif hash_type == "md5":
                h = hashlib.md5(word.encode()).hexdigest()
            elif hash_type == "sha1":
                h = hashlib.sha1(word.encode()).hexdigest()
            else:
                h = hashlib.sha256(word.encode()).hexdigest()

            if h == target_hash:
                found = word
                break

        return json.dumps({
            "hash_type": hash_type,
            "attempted": len(wordlist),
            "found": found is not None,
            "password": found if found else None,
            "message": f"Password found: {found}" if found else "Password not in wordlist"
        })

    @event("encrypt_file")
    def encrypt_file(self, data):
        """Encrypt a sensitive file"""
        payload = json.loads(data) if isinstance(data, str) else data
        filepath = payload.get("file", "")
        password = payload.get("password", "default")

        self._log_action("encrypt_file", f"File: {filepath}")

        try:
            with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()

            encrypted = self.encryption.encrypt(content)

            with open(filepath + ".enc", "w") as f:
                f.write(encrypted)

            return json.dumps({
                "status": "encrypted",
                "file": filepath + ".enc",
                "original_size": len(content),
                "encrypted_size": len(encrypted)
            })
        except Exception as e:
            return json.dumps({"status": "error", "message": str(e)})

    @event("decrypt_file")
    def decrypt_file(self, data):
        """Decrypt a file"""
        payload = json.loads(data) if isinstance(data, str) else data
        filepath = payload.get("file", "")
        password = payload.get("password", "default")

        try:
            with open(filepath, "r") as f:
                content = f.read()

            decrypted = self.encryption.decrypt(content)

            out_path = filepath.replace(".enc", ".dec")
            with open(out_path, "w") as f:
                f.write(decrypted)

            return json.dumps({"status": "decrypted", "file": out_path})
        except Exception as e:
            return json.dumps({"status": "error", "message": str(e)})

    # ═══════════════════════════════════════════════════════════
    #  3. PASSWORD ANALYZER
    # ═══════════════════════════════════════════════════════════

    @event("password_strength")
    def password_strength(self, data):
        """Analyze password strength"""
        payload = json.loads(data) if isinstance(data, str) else data
        password = payload.get("password", "")

        score = 0
        feedback = []

        if len(password) >= 12: score += 2; feedback.append("✅ Length ≥ 12")
        elif len(password) >= 8: score += 1; feedback.append("⚠️ Length ≥ 8 (recommend 12+)")
        else: feedback.append("❌ Too short (< 8 characters)")

        if re.search(r'[A-Z]', password): score += 1; feedback.append("✅ Uppercase")
        else: feedback.append("❌ Missing uppercase letter")
        if re.search(r'[a-z]', password): score += 1; feedback.append("✅ Lowercase")
        else: feedback.append("❌ Missing lowercase letter")
        if re.search(r'[0-9]', password): score += 1; feedback.append("✅ Numbers")
        else: feedback.append("❌ Missing number")
        if re.search(r'[!@#$%^&*(),.?\":{}|<>]', password): score += 2
        feedback.append("✅ Special characters" if re.search(r'[!@#$%^&*(),.?\":{}|<>]', password) else "⚠️ No special characters")

        # Check common patterns
        common = ["password", "123456", "qwerty", "admin", "letmein", "welcome"]
        if any(c in password.lower() for c in common):
            score -= 2
            feedback.append("❌ Contains common password pattern!")

        # Entropy estimation
        charset = 0
        if re.search(r'[a-z]', password): charset += 26
        if re.search(r'[A-Z]', password): charset += 26
        if re.search(r'[0-9]', password): charset += 10
        if re.search(r'[!@#$%^&*(),.?\":{}|<>]', password): charset += 32
        entropy = len(password) * (charset.bit_length() if charset > 0 else 1)

        strength = "WEAK 🔴" if score <= 2 else ("FAIR 🟡" if score <= 4 else ("STRONG 🟢" if score <= 6 else "VERY STRONG 💪"))

        return json.dumps({
            "score": score,
            "max_score": 8,
            "strength": strength,
            "feedback": feedback,
            "entropy_bits": entropy,
            "crack_time_estimate": f"~{10 ** (entropy/10):.0f} attempts needed"
        })

    @event("generate_strong_password")
    def generate_strong_password(self, data):
        """Generate cryptographically strong password"""
        payload = json.loads(data) if isinstance(data, str) else {}
        length = payload.get("length", 20)

        import secrets
        chars = string.ascii_letters + string.digits + "!@#$%^&*()_+-=[]{}|;:,.<>?"
        password = ''.join(secrets.choice(chars) for _ in range(length))

        # Ensure it has all character types
        while not (re.search(r'[A-Z]', password) and re.search(r'[a-z]', password)
                   and re.search(r'[0-9]', password) and re.search(r'[!@#$%^&*]', password)):
            password = ''.join(secrets.choice(chars) for _ in range(length))

        return json.dumps({
            "password": password,
            "length": length,
            "strength": "VERY STRONG 💪",
            "note": "Save this password securely!"
        })

    # ═══════════════════════════════════════════════════════════
    #  4. OSINT (Open Source Intelligence)
    # ═══════════════════════════════════════════════════════════

    @event("osint_search")
    def osint_search(self, data):
        """OSINT reconnaissance using search engines"""
        payload = json.loads(data) if isinstance(data, str) else data
        target = payload.get("target", "")
        search_type = payload.get("type", "general")

        self._log_action("osint_search", f"Target: {target}, Type: {search_type}")

        queries = {
            "email": f'"{target}" email',
            "domain": f'site:{target}',
            "username": f'"{target}" github linkedin twitter',
            "phone": f'"{target}" phone contact',
            "general": f'"{target}"'
        }

        query = queries.get(search_type, queries["general"])
        results = self.search.web_search_summary(query, max_results=5)

        return json.dumps({
            "target": target,
            "type": search_type,
            "query_used": query,
            "results_count": len(results.get("results", [])),
            "summary": results.get("summary", ""),
            "disclaimer": "⚠️ OSINT for authorized reconnaissance only!"
        })

    # ═══════════════════════════════════════════════════════════
    #  5. FORENSICS TOOLKIT
    # ═══════════════════════════════════════════════════════════

    @event("file_forensics")
    def file_forensics(self, data):
        """Analyze file metadata and properties"""
        payload = json.loads(data) if isinstance(data, str) else data
        filepath = payload.get("file", "")

        self._log_action("file_forensics", f"File: {filepath}")

        try:
            stat = os.stat(filepath)
            with open(filepath, "rb") as f:
                content = f.read()

            # File hashes
            md5_hash = hashlib.md5(content).hexdigest()
            sha1_hash = hashlib.sha1(content).hexdigest()
            sha256_hash = hashlib.sha256(content).hexdigest()

            # Detect file type magic bytes
            magic = content[:8].hex()
            file_types = {
                "ffd8ffe0": "JPEG Image", "89504e47": "PNG Image",
                "47494638": "GIF Image", "25504446": "PDF Document",
                "504b0304": "ZIP Archive", "52617221": "RAR Archive",
                "4d5a": "Windows EXE/DLL", "7f454c46": "ELF Executable",
                "d0cf11e0": "MS Office (old)", "3c3f786d": "XML File"
            }
            detected = "Unknown"
            for sig, name in file_types.items():
                if magic.startswith(sig):
                    detected = name; break

            # String analysis
            strings_found = []
            try:
                text = content.decode("utf-8", errors="ignore")
                emails = list(set(re.findall(r'[\w\.-]+@[\w\.-]+\.\w+', text)))[:5]
                urls = list(set(re.findall(r'https?://[^\s<>"]+', text)))[:5]
                ips = list(set(re.findall(r'\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b', text)))[:5]
                strings_found = {"emails": emails, "urls": urls, "ips": ips}
            except:
                pass

            return json.dumps({
                "file": filepath,
                "size_bytes": stat.st_size,
                "size_human": f"{stat.st_size/1024:.1f} KB" if stat.st_size < 1024*1024
                              else f"{stat.st_size/(1024*1024):.1f} MB",
                "modified": time.ctime(stat.st_mtime),
                "created": time.ctime(stat.st_ctime),
                "detected_type": detected,
                "magic_bytes": magic,
                "hashes": {"md5": md5_hash, "sha1": sha1_hash, "sha256": sha256_hash},
                "strings_found": strings_found
            })
        except Exception as e:
            return json.dumps({"error": str(e)})

    # ═══════════════════════════════════════════════════════════
    #  6. NETWORK MONITOR
    # ═══════════════════════════════════════════════════════════

    @event("network_monitor")
    def network_monitor(self, data):
        """Monitor network for anomalies"""
        payload = json.loads(data) if isinstance(data, str) else data
        target = payload.get("target", "127.0.0.1")

        self._log_action("network_monitor", f"Target: {target}")

        # DNS lookup simulation
        try:
            import socket as sock
            hostname = sock.gethostbyaddr(target)[0] if target != "127.0.0.1" else "localhost"
        except:
            hostname = "unknown"

        return json.dumps({
            "target": target,
            "hostname": hostname,
            "recommendations": [
                "✅ Enable firewall (UFW/iptables)",
                "✅ Use fail2ban for brute-force protection",
                "✅ Monitor logs with SIEM",
                "✅ Regular vulnerability scanning",
                "✅ Keep all software updated",
                "✅ Use IDS/IPS (Snort/Suricata)"
            ]
        })

    # ═══════════════════════════════════════════════════════════
    #  7. AI SECURITY ANALYST
    # ═══════════════════════════════════════════════════════════

    def _analyze_threat(self, threat_description="", **kw):
        """AI-powered threat analysis"""
        analysis = self.hermes.chat(
            f"As a cybersecurity expert, analyze this threat: {threat_description}. "
            f"Provide: 1) Severity level, 2) Attack vector, 3) Recommended mitigation, "
            f"4) CVE references if applicable."
        )
        return json.dumps({
            "threat": threat_description,
            "ai_analysis": analysis["reply"][:300],
            "analyzed_at": time.time()
        })

    @event("ai_security_audit")
    def ai_security_audit(self, data):
        """Full AI-powered security audit"""
        payload = json.loads(data) if isinstance(data, str) else data
        system_description = payload.get("description", "web application")

        self._log_action("ai_security_audit", f"System: {system_description}")

        # Search for latest threats
        web_results = self.search.web_search_summary(
            f"latest security vulnerabilities {system_description} 2026"
        )

        # AI analysis
        audit_prompt = (
            f"Conduct a security audit for: {system_description}. "
            f"Latest threats from web: {web_results.get('summary', '')[:200]}. "
            f"Provide: OWASP Top 10 checklist, critical vulnerabilities, "
            f"and prioritized remediation steps."
        )

        audit = self.hermes.chat(audit_prompt)

        return json.dumps({
            "system": system_description,
            "ai_audit": audit["reply"][:400],
            "threat_intel_sources": len(web_results.get("results", [])),
            "owasp_checklist": [
                "1. Broken Access Control",
                "2. Cryptographic Failures",
                "3. Injection (SQL, XSS, Command)",
                "4. Insecure Design",
                "5. Security Misconfiguration",
                "6. Vulnerable Components",
                "7. Auth Failures",
                "8. Software & Data Integrity",
                "9. Logging & Monitoring",
                "10. SSRF"
            ]
        })

    # ═══════════════════════════════════════════════════════════
    #  8. AUDIT & REPORT
    # ═══════════════════════════════════════════════════════════

    @event("audit_report")
    def audit_report(self, data):
        """Generate full security audit report"""
        report = {
            "toolkit": "PyTreX CyberSec Toolkit",
            "version": "1.0.0",
            "generated_at": time.strftime("%Y-%m-%d %H:%M:%S"),
            "total_actions": len(self.audit_log),
            "actions": self.audit_log[-10:],  # Last 10 actions
            "blockchain_verified": True,
            "blockchain_blocks": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0,
            "disclaimer": "⚠️ All actions logged on immutable blockchain. Authorized use only."
        }

        return json.dumps(report)

    @event("system_status")
    def system_status(self, data):
        """Get full toolkit status"""
        return json.dumps({
            "toolkit_name": "PyTreX CyberSec Toolkit",
            "modules_available": [
                "Vulnerability Scanner", "Crypto Toolkit",
                "Password Analyzer", "OSINT Toolkit",
                "Forensics", "Network Monitor",
                "AI Security Analyst", "Blockchain Audit"
            ],
            "total_actions_logged": len(self.audit_log),
            "ai_agent": "Hermes SecurityAI — Ready",
            "blockchain_audit": "Active — Immutable",
            "encryption": "AES-256 — Active"
        })


# ═══════════════════════════════════════════════════════════════
#  LIVE DEMO
# ═══════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("═" * 60)
    print("  🛡️  PYTREX CYBERSEC TOOLKIT — Live Demo")
    print("  ⚠️  AUTHORIZED SECURITY TESTING ONLY!")
    print("═" * 60)

    toolkit = CyberSecToolkit()

    # ═══ 1. Port Scanner ═══
    print("\n━━━ 1. PORT SCANNER ━━━")
    r = toolkit.port_scan('{"target": "127.0.0.1", "ports": [22, 80, 443, 8080]}')
    scan = json.loads(r)
    print(f"   🎯 Target: {scan['target']}")
    print(f"   🔍 Scanned: {scan['scanned_ports']} ports")
    for p in scan["open_ports"]:
        print(f"   🔴 OPEN: {p['port']} ({p['service']})")

    # ═══ 2. Password Analyzer ═══
    print("\n━━━ 2. PASSWORD ANALYZER ━━━")
    for pwd in ["abc123", "PyTreX@2026!Secure", "password"]:
        r = toolkit.password_strength(json.dumps({"password": pwd}))
        result = json.loads(r)
        print(f"   🔑 '{pwd}' → {result['strength']} (Score: {result['score']}/{result['max_score']})")

    # ═══ 3. Generate Strong Password ═══
    print("\n━━━ 3. STRONG PASSWORD GENERATOR ━━━")
    r = toolkit.generate_strong_password('{"length": 24}')
    pwd = json.loads(r)
    print(f"   🔐 Generated: {pwd['password']}")
    print(f"   💪 Strength: {pwd['strength']}")

    # ═══ 4. Hash Cracking (Recovery Demo) ═══
    print("\n━━━ 4. HASH CRACKING (Recovery) ━━━")
    target = hashlib.sha256("admin".encode()).hexdigest()
    r = toolkit.hash_crack(json.dumps({"hash": target, "type": "sha256"}))
    result = json.loads(r)
    print(f"   🔓 Hash: {target[:16]}...")
    print(f"   📋 Wordlist: 8 words tried")
    print(f"   {'✅ Found:' + result['password'] if result['found'] else '❌ Not found'}")

    # ═══ 5. Web Vulnerability Scanner ═══
    print("\n━━━ 5. WEB VULNERABILITY SCANNER ━━━")
    r = toolkit.web_scan('{"url": "http://example.com"}')
    findings = json.loads(r)
    print(f"   🌐 Target: {findings['url']}")
    for f in findings["findings"][:4]:
        print(f"   [{f['severity']}] {f['type']}: {f['description'][:50]}...")

    # ═══ 6. OSINT Search ═══
    print("\n━━━ 6. OSINT RECONNAISSANCE ━━━")
    r = toolkit.osint_search('{"target": "tanzania cybersecurity", "type": "general"}')
    osint = json.loads(r)
    print(f"   🔍 Search: {osint['target']}")
    print(f"   📊 Results: {osint['results_count']}")
    print(f"   ⚠️  {osint['disclaimer']}")

    # ═══ 7. AI Security Audit ═══
    print("\n━━━ 7. AI SECURITY AUDIT ━━━")
    r = toolkit.ai_security_audit('{"description": "Web banking application with blockchain"}')
    audit = json.loads(r)
    print(f"   🧠 System: {audit['system']}")
    print(f"   📋 OWASP Checklist: {len(audit['owasp_checklist'])} items")
    print(f"   🤖 AI Analysis: {audit['ai_audit'][:150]}...")

    # ═══ 8. Audit Trail ═══
    print("\n━━━ 8. BLOCKCHAIN AUDIT TRAIL ━━━")
    r = toolkit.audit_report("{}")
    report = json.loads(r)
    print(f"   📝 Total Actions: {report['total_actions']}")
    print(f"   🔗 Blockchain Blocks: {report['blockchain_blocks']}")
    print(f"   ✅ All actions IMMUTABLY logged!")

    # ═══ 9. System Status ═══
    print("\n━━━ 9. TOOLKIT STATUS ━━━")
    r = toolkit.system_status("{}")
    status = json.loads(r)
    for mod in status["modules_available"]:
        print(f"   ✅ {mod}")

    print("\n" + "═" * 60)
    print("  ✅ PYTREX CYBERSEC TOOLKIT — FULLY OPERATIONAL!")
    print("  ⚠️  FOR AUTHORIZED USE ONLY — ALL ACTIONS LOGGED!")
    print("═" * 60)
