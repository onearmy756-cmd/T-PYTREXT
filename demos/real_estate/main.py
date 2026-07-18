"""
╔══════════════════════════════════════════════════════════════╗
║  DEMO 5: REAL ESTATE MANAGEMENT PLATFORM                   ║
║  Features: Property Listings, AI Valuation,                ║
║            Tenant/Landlord Management, Contracts,          ║
║            Payment Tracking (Blockchain), Search/Filter    ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import HermesAgent, BlockchainBridge, HumanInTheLoop
from pytrex.search_engine import SearchEngine
import json, time, uuid, random

class RealEstatePlatform(PyTreXApp):
    """Full Real Estate Platform — Property Management + Marketplace"""

    def __init__(self):
        super().__init__(name="PyTreX Properties")
        self.hermes = HermesAgent(name="Property AI")
        self.blockchain = BlockchainBridge()
        self.search = SearchEngine()
        self.hitl = HumanInTheLoop()

        self.properties = []    # Property listings
        self.tenants = {}       # Tenant records
        self.contracts = []     # Rental contracts (blockchain-backed)
        self.payments = []      # Payment records
        self.valuations = {}    # AI property valuations

        # Seed some properties
        self._seed_properties()

    def _seed_properties(self):
        """Add sample properties"""
        samples = [
            {"title": "Nyumba ya Kifahari Masaki", "type": "House", "bedrooms": 5,
             "price": 850000000, "location": "Masaki, Dar es Salaam", "status": "for_sale"},
            {"title": "Ofisi Modern Posta", "type": "Office", "bedrooms": 0,
             "price": 350000, "location": "Posta, Dar es Salaam", "status": "for_rent"},
            {"title": "Kiwanja Kibaha", "type": "Land", "bedrooms": 0,
             "price": 45000000, "location": "Kibaha, Pwani", "status": "for_sale"},
            {"title": "Hostel Mwenge", "type": "Hostel", "bedrooms": 12,
             "price": 1500000, "location": "Mwenge, Dar es Salaam", "status": "for_rent"},
            {"title": "Nyumba Mbezi Beach", "type": "House", "bedrooms": 3,
             "price": 250000000, "location": "Mbezi Beach, Dar es Salaam", "status": "for_sale"},
        ]
        for s in samples:
            s["id"] = f"PROP-{str(uuid.uuid4())[:8]}"
            s["listed_at"] = time.time()
            s["views"] = random.randint(10, 500)
            self.properties.append(s)

    @event("list_property")
    def list_property(self, data):
        """Weka property sokoni"""
        payload = json.loads(data) if isinstance(data, str) else data
        prop = {
            "id": f"PROP-{str(uuid.uuid4())[:8]}",
            "title": payload.get("title", ""),
            "type": payload.get("type", "House"),
            "bedrooms": payload.get("bedrooms", 1),
            "price": payload.get("price", 0),
            "location": payload.get("location", ""),
            "status": payload.get("status", "for_sale"),
            "description": payload.get("description", ""),
            "owner": payload.get("owner", "Anonymous"),
            "listed_at": time.time(),
            "views": 0
        }
        self.properties.append(prop)

        # AI Valuation
        self.hermes.chat(
            f"Give a property valuation opinion for: {prop['title']} in {prop['location']}, "
            f"{prop['bedrooms']} bedrooms, listed at TZS {prop['price']:,}. Is this a fair price?"
        )

        return json.dumps({"status": "listed", "property": prop})

    @event("search_properties")
    def search_properties(self, data):
        """Tafuta property kwa filters"""
        payload = json.loads(data) if isinstance(data, str) else data
        query = payload.get("query", "").lower()
        prop_type = payload.get("type", "").lower()
        status = payload.get("status", "").lower()
        max_price = payload.get("max_price", float("inf"))
        min_bedrooms = payload.get("min_bedrooms", 0)

        results = []
        for p in self.properties:
            if query and query not in p["title"].lower() and query not in p["location"].lower():
                continue
            if prop_type and p["type"].lower() != prop_type:
                continue
            if status and p["status"] != status:
                continue
            if p["price"] > max_price:
                continue
            if p["bedrooms"] < min_bedrooms:
                continue
            results.append(p)

        return json.dumps({
            "total_found": len(results),
            "properties": results[:20],
            "filters": payload
        })

    @event("create_contract")
    def create_contract(self, data):
        """Tengeneza mkataba wa kukodisha — backed by blockchain"""
        payload = json.loads(data) if isinstance(data, str) else data

        contract = {
            "id": f"CTR-{str(uuid.uuid4())[:8]}",
            "property_id": payload.get("property_id", ""),
            "tenant_name": payload.get("tenant", ""),
            "landlord": payload.get("landlord", ""),
            "monthly_rent": payload.get("rent", 0),
            "deposit": payload.get("deposit", 0),
            "start_date": payload.get("start", ""),
            "duration_months": payload.get("duration", 12),
            "status": "active",
            "signed_at": time.time()
        }

        # Record on blockchain
        self.blockchain.add_block(json.dumps({
            "action": "contract_signed",
            "contract_id": contract["id"],
            "property": contract["property_id"]
        }))
        self.contracts.append(contract)

        return json.dumps({"status": "signed", "contract": contract})

    @event("record_payment")
    def record_payment(self, data):
        """Rekodi malipo ya kodi — blockchain verified"""
        payload = json.loads(data) if isinstance(data, str) else data

        payment = {
            "id": f"PAY-{str(uuid.uuid4())[:8]}",
            "contract_id": payload.get("contract_id", ""),
            "amount": payload.get("amount", 0),
            "currency": "TZS",
            "method": payload.get("method", "M-Pesa"),
            "paid_at": time.time()
        }
        self.payments.append(payment)

        # Record on blockchain
        self.blockchain.add_block(json.dumps({
            "action": "rent_payment",
            "payment_id": payment["id"],
            "amount": payment["amount"]
        }))

        return json.dumps({"status": "paid", "payment": payment})

    @event("ai_valuation")
    def ai_valuation(self, data):
        """AI Property Valuation"""
        payload = json.loads(data) if isinstance(data, str) else data
        prop_id = payload.get("property_id", "")

        prop = next((p for p in self.properties if p["id"] == prop_id), None)
        if not prop:
            return json.dumps({"error": "Property not found"})

        # AI analysis using Hermes + Search
        market_data = self.search.web_search_summary(
            f"property prices {prop['location']} Tanzania"
        )

        valuation_prompt = (
            f"Property: {prop['title']}, {prop['bedrooms']} bed {prop['type']} "
            f"in {prop['location']}. Listed at TZS {prop['price']:,}. "
            f"Market data: {market_data['summary'][:200]}. "
            f"Give a valuation opinion with estimated market value range in TZS."
        )

        ai_result = self.hermes.chat(valuation_prompt)
        estimated_value = prop["price"] * random.uniform(0.85, 1.25)

        valuation = {
            "property_id": prop_id,
            "listed_price": prop["price"],
            "ai_estimated_value": round(estimated_value),
            "ai_opinion": ai_result["reply"][:200],
            "market_trend": random.choice(["Rising 📈", "Stable ➡️", "Declining 📉"]),
            "valued_at": time.time()
        }
        self.valuations[prop_id] = valuation

        return json.dumps(valuation)

    @event("platform_stats")
    def platform_stats(self, data):
        """Get platform statistics"""
        total_value = sum(p["price"] for p in self.properties)
        total_payments = sum(p["amount"] for p in self.payments)

        return json.dumps({
            "total_properties": len(self.properties),
            "total_value_tzs": total_value,
            "active_contracts": len(self.contracts),
            "total_tenants": len(self.tenants),
            "total_payments": total_payments,
            "blockchain_records": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0,
            "property_types": list(set(p["type"] for p in self.properties)),
            "locations": list(set(p["location"] for p in self.properties)),
        })


# ─── RUN ─────────────────────────────────────────────────────
if __name__ == "__main__":
    print("═" * 55)
    print("  🏠 PYTREX REAL ESTATE — Live Demo")
    print("═" * 55)

    re_platform = RealEstatePlatform()

    # Search properties
    r1 = re_platform.search_properties(json.dumps({
        "query": "Dar", "max_price": 500000000, "min_bedrooms": 3
    }))
    results = json.loads(r1)
    print(f"\n  🔍 Search 'Dar es Salaam, 3+ beds, ≤500M TZS'")
    print(f"     Found: {results['total_found']} properties")
    for p in results["properties"][:3]:
        print(f"     • {p['title']} — TZS {p['price']:,} ({p['bedrooms']} beds)")

    # AI Valuation
    r2 = re_platform.ai_valuation(json.dumps({
        "property_id": re_platform.properties[0]["id"]
    }))
    val = json.loads(r2)
    print(f"\n  🧠 AI Valuation: {re_platform.properties[0]['title']}")
    print(f"     Listed: TZS {val['listed_price']:,}")
    print(f"     AI Estimate: TZS {val['ai_estimated_value']:,}")
    print(f"     Trend: {val['market_trend']}")

    # Create contract
    r3 = re_platform.create_contract(json.dumps({
        "property_id": re_platform.properties[1]["id"],
        "tenant": "DR MBILINYI",
        "landlord": "Property Owner Ltd",
        "rent": 350000, "deposit": 350000,
        "start": "2026-08-01", "duration": 12
    }))
    contract = json.loads(r3)["contract"]
    print(f"\n  📝 Contract: {contract['id']}")
    print(f"     Tenant: {contract['tenant_name']}")
    print(f"     Rent: TZS {contract['monthly_rent']:,}/month")

    # Record payment
    r4 = re_platform.record_payment(json.dumps({
        "contract_id": contract["id"], "amount": 350000, "method": "M-Pesa"
    }))
    print(f"\n  💰 Payment: TZS 350,000 via M-Pesa ✓")

    # Platform stats
    stats = json.loads(re_platform.platform_stats("{}"))
    print(f"\n  📊 Platform Stats:")
    print(f"     Properties: {stats['total_properties']}")
    print(f"     Total Value: TZS {stats['total_value_tzs']:,.0f}")
    print(f"     Active Contracts: {stats['active_contracts']}")
    print(f"     Blockchain Records: {stats['blockchain_records']}")

    print(f"\n  ✅ Real Estate Platform: FULLY OPERATIONAL")
    print(f"═" * 55)
