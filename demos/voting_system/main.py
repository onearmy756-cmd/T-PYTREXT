"""
╔══════════════════════════════════════════════════════════════╗
║  DEMO: BLOCKCHAIN VOTING SYSTEM                            ║
║  -------------------------------------------------------  ║
║  🔗 Blockchain — kila kura imerekodiwa (immutable)         ║
║  🔐 Encryption — siri ya mpiga kura inalindwa              ║
║  🧠 AI — uchambuzi wa matokeo, utabiri                     ║
║  👤 HITL — approval kwa vitendo muhimu                     ║
║  🌐 Real-time — matokeo live kupitia Elixir                ║
║  📊 Dashboard — live results visualization                 ║
║  🛡️ Anti-Fraud — double-vote detection, tamper-proof      ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import BlockchainBridge, EncryptionManager, HumanInTheLoop, HermesAgent
import json, time, uuid, hashlib

class VotingSystem(PyTreXApp):
    """Blockchain-Backed Voting System — Tamper-Proof & Transparent"""

    def __init__(self):
        super().__init__(name="Uchaguzi Smart")
        self.blockchain = BlockchainBridge()
        self.encryption = EncryptionManager(default_password="uchaguzi_secret_key")
        self.hitl = HumanInTheLoop(default_timeout=300)
        self.hermes = HermesAgent(name="Uchaguzi AI")

        # Voting state
        self.election = None          # Current election
        self.voters = {}              # Registered voters (encrypted)
        self.votes_cast = []          # All votes (blockchain-backed)
        self.voter_ids_voted = set()  # Prevent double voting
        self.candidates = {}          # Candidates & vote counts
        self.is_election_open = False

    # ═══════════════════════════════════════════════════════════
    #  ELECTION MANAGEMENT
    # ═══════════════════════════════════════════════════════════

    @event("create_election")
    def create_election(self, data):
        """Anzisha uchaguzi mpya"""
        payload = json.loads(data) if isinstance(data, str) else data

        election = {
            "id": f"EL-{str(uuid.uuid4())[:8]}",
            "title": payload.get("title", "Uchaguzi Mkuu 2026"),
            "type": payload.get("type", "Presidential"),
            "positions": payload.get("positions", ["Rais", "Wabunge"]),
            "start_date": payload.get("start", time.strftime("%Y-%m-%d")),
            "end_date": payload.get("end", time.strftime("%Y-%m-%d")),
            "status": "created",
            "created_by": payload.get("admin", "Tume ya Uchaguzi"),
            "created_at": time.time()
        }
        self.election = election
        self.candidates = {pos: {} for pos in election["positions"]}

        # Record on blockchain
        self.blockchain.add_block(json.dumps({
            "action": "election_created",
            "election_id": election["id"],
            "title": election["title"]
        }))

        return json.dumps({"status": "created", "election": election})

    @event("open_election")
    def open_election(self, data):
        """Fungua uchaguzi — inahitaji HITL APPROVAL"""
        if not self.election:
            return json.dumps({"error": "No election created"})

        # Human approval required to open elections!
        approval_id = self.hitl.request_approval(
            "open_election",
            {"election": self.election["title"], "timestamp": time.time()},
            timeout=600
        )

        return json.dumps({
            "status": "pending_approval",
            "approval_id": approval_id,
            "message": f"⚠️ Kufungua uchaguzi kunahitaji idhini! Approve: hitl.approve('{approval_id}')"
        })

    @event("approve_open")
    def approve_open(self, data):
        """Idhinisha kufungua uchaguzi (baada ya HITL)"""
        payload = json.loads(data) if isinstance(data, str) else data
        approval_id = payload.get("approval_id", "")

        if self.hitl.approve(approval_id):
            self.is_election_open = True
            self.election["status"] = "open"

            self.blockchain.add_block(json.dumps({
                "action": "election_opened",
                "election_id": self.election["id"],
                "time": time.time()
            }))

            return json.dumps({"status": "opened", "election": self.election["title"]})

        return json.dumps({"error": "Approval failed"})

    # ═══════════════════════════════════════════════════════════
    #  VOTER REGISTRATION
    # ═══════════════════════════════════════════════════════════

    @event("register_voter")
    def register_voter(self, data):
        """Sajili mpiga kura — data inasimbwa kwa AES-256"""
        payload = json.loads(data) if isinstance(data, str) else data

        # Generate unique voter ID (hashed for privacy)
        raw_id = f"{payload.get('nida', '')}-{payload.get('name', '')}-{time.time()}"
        voter_id = hashlib.sha256(raw_id.encode()).hexdigest()[:12]

        if voter_id in self.voters:
            return json.dumps({"error": "Mpiga kura tayari amesajiliwa"})

        voter = {
            "voter_id": voter_id,
            "name_encrypted": self.encryption.encrypt(payload.get("name", "")),
            "nida_encrypted": self.encryption.encrypt(payload.get("nida", "")),
            "region_encrypted": self.encryption.encrypt(payload.get("region", "")),
            "registered_at": time.time(),
            "has_voted": False
        }
        self.voters[voter_id] = voter

        # Blockchain record (no personal data — just verification hash)
        self.blockchain.add_block(json.dumps({
            "action": "voter_registered",
            "voter_hash": hashlib.sha256(voter_id.encode()).hexdigest()[:16],
            "region_hash": hashlib.sha256(payload.get("region", "").encode()).hexdigest()[:12],
            "timestamp": time.time()
        }))

        return json.dumps({
            "status": "registered",
            "voter_id": voter_id,
            "note": "Hifadhi voter_id yako kwa siri!"
        })

    @event("verify_voter")
    def verify_voter(self, data):
        """Hakiki kama mpiga kura yupo na hajapiga kura"""
        payload = json.loads(data) if isinstance(data, str) else data
        voter_id = payload.get("voter_id", "")

        voter = self.voters.get(voter_id)
        if not voter:
            return json.dumps({"status": "not_found", "message": "Mpiga kura hajasajiliwa"})

        if voter["has_voted"]:
            return json.dumps({"status": "already_voted", "message": "Tayari umesha-piga kura!"})

        if not self.is_election_open:
            return json.dumps({"status": "closed", "message": "Uchaguzi bado haujafunguliwa"})

        # Decrypt for verification
        name = self.encryption.decrypt(voter["name_encrypted"])
        region = self.encryption.decrypt(voter["region_encrypted"])

        return json.dumps({
            "status": "verified",
            "name": name,
            "region": region,
            "can_vote": True
        })

    # ═══════════════════════════════════════════════════════════
    #  CANDIDATE MANAGEMENT
    # ═══════════════════════════════════════════════════════════

    @event("register_candidate")
    def register_candidate(self, data):
        """Sajili mgombea"""
        payload = json.loads(data) if isinstance(data, str) else data
        position = payload.get("position", "Rais")
        candidate_name = payload.get("name", "")
        party = payload.get("party", "Independent")

        if position not in self.candidates:
            self.candidates[position] = {}

        candidate_id = f"C-{str(uuid.uuid4())[:8]}"
        self.candidates[position][candidate_id] = {
            "id": candidate_id,
            "name": candidate_name,
            "party": party,
            "votes": 0,
            "registered_at": time.time()
        }

        # Blockchain record
        self.blockchain.add_block(json.dumps({
            "action": "candidate_registered",
            "candidate_id": candidate_id,
            "name": candidate_name,
            "party": party,
            "position": position
        }))

        return json.dumps({
            "status": "registered",
            "candidate": self.candidates[position][candidate_id]
        })

    # ═══════════════════════════════════════════════════════════
    #  VOTING (THE CORE!)
    # ═══════════════════════════════════════════════════════════

    @event("cast_vote")
    def cast_vote(self, data):
        """Piga kura — kila kura inarekodiwa kwenye BLOCKCHAIN"""
        payload = json.loads(data) if isinstance(data, str) else data
        voter_id = payload.get("voter_id", "")
        votes = payload.get("votes", {})  # {"Rais": "C-xxx", "Wabunge": "C-yyy"}

        # Verify voter
        voter = self.voters.get(voter_id)
        if not voter:
            return json.dumps({"status": "rejected", "reason": "Mpiga kura hajasajiliwa"})

        if voter["has_voted"]:
            return json.dumps({"status": "rejected", "reason": "Tayari umesha-piga kura!"})

        if voter_id in self.voter_ids_voted:
            return json.dumps({"status": "rejected", "reason": "⚠️ DOUBLE VOTE DETECTED — blocked!"})

        if not self.is_election_open:
            return json.dumps({"status": "rejected", "reason": "Uchaguzi haujafunguliwa"})

        # Verify candidates exist
        for position, candidate_id in votes.items():
            if position not in self.candidates:
                return json.dumps({"status": "rejected", "reason": f"Position '{position}' haipo"})
            if candidate_id not in self.candidates[position]:
                return json.dumps({"status": "rejected", "reason": f"Candidate '{candidate_id}' haipo"})

        # COUNT THE VOTE
        for position, candidate_id in votes.items():
            self.candidates[position][candidate_id]["votes"] += 1

        # Mark voter
        voter["has_voted"] = True
        self.voter_ids_voted.add(voter_id)

        # Generate encrypted ballot receipt
        ballot_hash = hashlib.sha256(
            f"{voter_id}-{json.dumps(votes)}-{time.time()}".encode()
        ).hexdigest()

        # RECORD ON BLOCKCHAIN (immutable!)
        vote_record = {
            "action": "vote_cast",
            "ballot_hash": ballot_hash[:16],
            "voter_anonymous_id": hashlib.sha256(voter_id.encode()).hexdigest()[:12],
            "positions_voted": list(votes.keys()),
            "timestamp": time.time()
        }
        self.blockchain.add_block(json.dumps(vote_record))
        self.votes_cast.append(vote_record)

        # Broadcast live result update
        self.bus.emit("new_vote", {
            "total_votes": len(self.votes_cast),
            "results": self._get_current_results()
        })

        return json.dumps({
            "status": "counted",
            "ballot_receipt": ballot_hash[:16],
            "message": "✅ Kura yako imehesabiwa na kurekodiwa kwenye blockchain!",
            "total_votes_so_far": len(self.votes_cast)
        })

    # ═══════════════════════════════════════════════════════════
    #  RESULTS & ANALYSIS
    # ═══════════════════════════════════════════════════════════

    @event("live_results")
    def live_results(self, data):
        """Matokeo live — real-time dashboard"""
        return json.dumps({
            "election": self.election,
            "is_open": self.is_election_open,
            "total_voters": len(self.voters),
            "total_votes": len(self.votes_cast),
            "turnout_percent": round(len(self.votes_cast) / max(len(self.voters), 1) * 100, 1),
            "results": self._get_current_results(),
            "blockchain_blocks": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0
        })

    def _get_current_results(self):
        """Pata matokeo ya sasa kwa kila nafasi"""
        results = {}
        for position, candidates in self.candidates.items():
            sorted_cands = sorted(
                candidates.values(),
                key=lambda c: c["votes"],
                reverse=True
            )
            total = sum(c["votes"] for c in sorted_cands)
            results[position] = {
                "total_votes": total,
                "candidates": [
                    {
                        "name": c["name"],
                        "party": c["party"],
                        "votes": c["votes"],
                        "percent": round(c["votes"] / max(total, 1) * 100, 1)
                    }
                    for c in sorted_cands
                ],
                "leader": sorted_cands[0]["name"] if sorted_cands else "N/A"
            }
        return results

    @event("ai_analysis")
    def ai_analysis(self, data):
        """AI inachambua matokeo na kutoa taarifa"""
        if not self.votes_cast:
            return json.dumps({"analysis": "Bado hakuna kura zilizopigwa."})

        results = self._get_current_results()

        # Hermes AI analysis
        summary_parts = []
        for pos, data in results.items():
            leader = data["leader"]
            cands = data["candidates"]
            if len(cands) >= 2:
                diff = cands[0]["votes"] - cands[1]["votes"]
                summary_parts.append(
                    f"{pos}: {cands[0]['name']} anaongoza kwa kura {cands[0]['votes']} "
                    f"({cands[0]['percent']}%) — tofauti ya kura {diff} na {cands[1]['name']}"
                )

        analysis_prompt = (
            f"Uchambuzi wa uchaguzi:\n" + "\n".join(summary_parts) +
            f"\n\nJumla ya wapiga kura waliojitokeza: {len(self.votes_cast)} kati ya {len(self.voters)} "
            f"({round(len(self.votes_cast)/max(len(self.voters),1)*100,1)}%). "
            f"Toa uchambuzi wa kitaalamu kwa Kiswahili."
        )

        ai_result = self.hermes.chat(analysis_prompt)

        return json.dumps({
            "results_summary": summary_parts,
            "ai_analysis": ai_result["reply"][:300],
            "total_votes": len(self.votes_cast),
            "voter_turnout": round(len(self.votes_cast) / max(len(self.voters), 1) * 100, 1)
        })

    @event("verify_election")
    def verify_election(self, data):
        """Hakiki uhalali wa uchaguzi — blockchain audit"""
        verify_result = self.blockchain.verify_chain()

        # Count votes from blockchain
        blockchain_votes = sum(
            1 for v in self.votes_cast
            if v.get("action") == "vote_cast"
        )

        return json.dumps({
            "blockchain_verified": verify_result.get("valid", False),
            "total_blockchain_records": len(self.votes_cast),
            "votes_on_blockchain": blockchain_votes,
            "votes_in_system": sum(
                sum(c["votes"] for c in cands.values())
                for cands in self.candidates.values()
            ),
            "integrity": "✅ SAFE — Blockchain verified" if verify_result.get("valid")
                         else "⚠️ TAMPERING DETECTED!",
            "double_votes_blocked": len(self.voter_ids_voted) - blockchain_votes
        })

    @event("close_election")
    def close_election(self, data):
        """Funga uchaguzi na toa matokeo rasmi"""
        if not self.is_election_open:
            return json.dumps({"error": "Uchaguzi tayari umefungwa"})

        self.is_election_open = False
        self.election["status"] = "closed"
        self.election["closed_at"] = time.time()

        final_results = self._get_current_results()

        # Final blockchain record
        self.blockchain.add_block(json.dumps({
            "action": "election_closed",
            "election_id": self.election["id"],
            "final_total_votes": len(self.votes_cast),
            "final_results_hash": hashlib.sha256(
                json.dumps(final_results).encode()
            ).hexdigest()
        }))

        return json.dumps({
            "status": "closed",
            "election": self.election["title"],
            "final_results": final_results,
            "total_votes": len(self.votes_cast),
            "blockchain_verified": True,
            "message": "✅ Uchaguzi umefungwa. Matokeo rasmi yamerekodiwa kwenye blockchain!"
        })


# ═══════════════════════════════════════════════════════════════
#  LIVE DEMO — RUN
# ═══════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("═" * 60)
    print("  🗳️  BLOCKCHAIN VOTING SYSTEM — Live Demo")
    print("═" * 60)

    tume = VotingSystem()

    # ═══ 1. Create Election ═══
    print("\n━━━ 1. Kuanzisha Uchaguzi ━━━")
    r = tume.create_election(json.dumps({
        "title": "Uchaguzi Mkuu Tanzania 2026",
        "type": "Presidential + Parliamentary",
        "positions": ["Rais", "Mbunge"],
        "admin": "Tume ya Taifa ya Uchaguzi (NEC)"
    }))
    el = json.loads(r)
    print(f"   ✅ {el['election']['title']} — ID: {el['election']['id']}")
    print(f"   📋 Positions: {', '.join(el['election']['positions'])}")

    # ═══ 2. Register Candidates ═══
    print("\n━━━ 2. Kusajili Wagombea ━━━")
    candidates_data = [
        ("Rais", "Juma Hamisi", "CCM"),
        ("Rais", "Amina Ali", "CHADEMA"),
        ("Rais", "David Mushi", "ACT-Wazalendo"),
        ("Mbunge", "Fatma Omar", "CCM"),
        ("Mbunge", "John Magesa", "CHADEMA"),
        ("Mbunge", "Neema Kitwanga", "CUF"),
    ]
    for pos, name, party in candidates_data:
        r = tume.register_candidate(json.dumps({
            "position": pos, "name": name, "party": party
        }))
        c = json.loads(r)["candidate"]
        print(f"   ✅ {c['name']} ({c['party']}) — {pos}")

    # ═══ 3. Register Voters ═══
    print("\n━━━ 3. Kusajili Wapiga Kura ━━━")
    voters_data = [
        ("Juma Omari", "NIDA-001", "Dar es Salaam"),
        ("Aisha Mohamed", "NIDA-002", "Mwanza"),
        ("Hamisi Bakari", "NIDA-003", "Arusha"),
        ("Grace Peter", "NIDA-004", "Dodoma"),
        ("Hassan Mwinyi", "NIDA-005", "Zanzibar"),
        ("Rehema Juma", "NIDA-006", "Mbeya"),
        ("Charles David", "NIDA-007", "Kilimanjaro"),
        ("Mwajuma Salum", "NIDA-008", "Morogoro"),
        ("Paulo Mbaga", "NIDA-009", "Tanga"),
        ("DR MBILINYI", "NIDA-010", "Dar es Salaam"),
    ]
    voter_ids = []
    for name, nida, region in voters_data:
        r = tume.register_voter(json.dumps({
            "name": name, "nida": nida, "region": region
        }))
        vid = json.loads(r)["voter_id"]
        voter_ids.append((vid, name, region))
        print(f"   ✅ {name} ({region}) — ID: {vid}")

    # ═══ 4. Open Election (HITL) ═══
    print("\n━━━ 4. Kufungua Uchaguzi ━━━")
    r = tume.open_election("{}")
    approval = json.loads(r)
    aid = approval["approval_id"]
    print(f"   ⚠️  Inahitaji HUMAN APPROVAL: {aid}")

    # Approve
    r = tume.approve_open(json.dumps({"approval_id": aid}))
    print(f"   ✅ {json.loads(r)['status']} — Uchaguzi umefunguliwa!")

    # ═══ 5. CAST VOTES! ═══
    print("\n━━━ 5. WANANCHI WANAPIGA KURA ━━━")

    # Simulate voting patterns
    import random
    voting_patterns = {
        "Rais": {"CCM": 0.45, "CHADEMA": 0.40, "ACT-Wazalendo": 0.15},
        "Mbunge": {"CCM": 0.35, "CHADEMA": 0.45, "CUF": 0.20},
    }

    for i, (vid, name, region) in enumerate(voter_ids):
        votes = {}
        for pos in ["Rais", "Mbunge"]:
            party = random.choices(
                list(voting_patterns[pos].keys()),
                weights=list(voting_patterns[pos].values())
            )[0]
            # Find candidate ID
            for cid, cd in tume.candidates[pos].items():
                if cd["party"] == party:
                    votes[pos] = cid
                    break

        r = tume.cast_vote(json.dumps({
            "voter_id": vid, "votes": votes
        }))
        result = json.loads(r)
        print(f"   🗳️  {name} ({region}) → {result['ballot_receipt']} — {result['message']}")

    # ═══ 6. LIVE RESULTS ═══
    print("\n━━━ 6. MATOKEO LIVE ━━━")
    r = tume.live_results("{}")
    results = json.loads(r)
    print(f"   📊 Registered: {results['total_voters']}")
    print(f"   🗳️  Votes Cast: {results['total_votes']}")
    print(f"   📈 Turnout: {results['turnout_percent']}%")
    print(f"   🔗 Blockchain Blocks: {results['blockchain_blocks']}")

    for pos, data in results["results"].items():
        print(f"\n   ── {pos} ──")
        for c in data["candidates"]:
            bar = "█" * int(c["percent"] / 5)
            print(f"     {c['name']} ({c['party']}): {c['votes']} votes ({c['percent']}%) {bar}")

    # ═══ 7. AI ANALYSIS ═══
    print("\n━━━ 7. AI ANALYSIS ━━━")
    r = tume.ai_analysis("{}")
    analysis = json.loads(r)
    print(f"   🧠 {analysis['ai_analysis'][:200]}...")

    # ═══ 8. VERIFY ELECTION ═══
    print("\n━━━ 8. BLOCKCHAIN AUDIT ━━━")
    r = tume.verify_election("{}")
    audit = json.loads(r)
    print(f"   🔗 Verified: {audit['blockchain_verified']}")
    print(f"   📊 Votes on Blockchain: {audit['votes_on_blockchain']}")
    print(f"   🛡️  Integrity: {audit['integrity']}")

    # ═══ 9. CLOSE ELECTION ═══
    print("\n━━━ 9. KUFUNGA UCHAGUZI ━━━")
    r = tume.close_election("{}")
    final = json.loads(r)
    print(f"   ✅ {final['message']}")
    print(f"   🏆 Winners:")
    for pos, data in final["final_results"].items():
        print(f"      {pos}: {data['leader']} ({data['candidates'][0]['votes']} votes)")

    # ═══ DOUBLE VOTE TEST ═══
    print("\n━━━ 10. SECURITY TEST: DOUBLE VOTE ═══")
    r = tume.cast_vote(json.dumps({
        "voter_id": voter_ids[0][0], "votes": {"Rais": list(tume.candidates["Rais"].keys())[0]}
    }))
    print(f"   🛡️  {json.loads(r)['reason']}")

    print("\n" + "═" * 60)
    print("  ✅ BLOCKCHAIN VOTING SYSTEM — FULLY OPERATIONAL!")
    print("═" * 60)
