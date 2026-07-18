"""
╔══════════════════════════════════════════════════════════════╗
║  DEMO 3: HOSPITAL MANAGEMENT SYSTEM                        ║
║  Features: Encrypted Patient DB, AI Diagnosis,             ║
║            Drug Tracking (Blockchain), Lab Results,        ║
║            Doctor Approval Workflows (HITL), Billing       ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import HermesAgent, HumanInTheLoop, BlockchainBridge, EncryptionManager
import json, time, uuid

class HospitalSystem(PyTreXApp):
    """Full Hospital Management — Electronic Health Records (EHR) System"""

    def __init__(self):
        super().__init__(name="Afya Smart Hospital")
        self.hermes = HermesAgent(name="Dr. AI")
        self.hitl = HumanInTheLoop()
        self.blockchain = BlockchainBridge()
        self.encryption = EncryptionManager(default_password="hospital_secret")

        # Encrypted databases
        self.patients = {}      # Patient records (encrypted)
        self.lab_results = {}   # Lab results
        self.prescriptions = [] # Drug prescriptions (blockchain-tracked)
        self.appointments = []  # Doctor appointments
        self.billing = []       # Billing records

    @event("register_patient")
    def register_patient(self, data):
        """Sajili mgonjwa mpya — data encrypted"""
        payload = json.loads(data) if isinstance(data, str) else data
        patient_id = f"PT-{str(uuid.uuid4())[:8]}"

        # Encrypt sensitive data
        patient = {
            "id": patient_id,
            "name": payload.get("name", ""),
            "age": payload.get("age", 0),
            "gender": payload.get("gender", ""),
            "phone_encrypted": self.encryption.encrypt(payload.get("phone", "")),
            "medical_history_encrypted": self.encryption.encrypt(payload.get("history", "")),
            "registered_at": time.time()
        }
        self.patients[patient_id] = patient

        # Record on blockchain for audit
        self.blockchain.add_block(json.dumps({
            "action": "patient_registered",
            "patient_id": patient_id, "timestamp": time.time()
        }))

        return json.dumps({"status": "registered", "patient_id": patient_id})

    @event("ai_diagnose")
    def ai_diagnose(self, data):
        """AI Diagnosis kwa kutumia Hermes Agent"""
        payload = json.loads(data) if isinstance(data, str) else data
        symptoms = payload.get("symptoms", "")
        patient_id = payload.get("patient_id", "")

        # Get patient history
        patient = self.patients.get(patient_id, {})
        history = self.encryption.decrypt(
            patient.get("medical_history_encrypted", "")
        ) if patient else "Unknown"

        # Hermes AI analysis
        diagnosis_prompt = (
            f"Patient age {patient.get('age', '?')}, gender {patient.get('gender', '?')}. "
            f"Medical history: {history}. "
            f"Current symptoms: {symptoms}. "
            f"Give a diagnosis and treatment recommendation in Swahili."
        )

        ai_result = self.hermes.chat(diagnosis_prompt)

        # Critical diagnosis → require HUMAN DOCTOR APPROVAL
        diagnosis_data = {
            "patient_id": patient_id,
            "symptoms": symptoms,
            "ai_diagnosis": ai_result["reply"],
            "timestamp": time.time()
        }

        # For serious cases: HITL approval
        if "serious" in str(ai_result).lower() or "critical" in str(ai_result).lower():
            approval_id = self.hitl.request_approval(
                "critical_diagnosis", diagnosis_data, timeout=600
            )
            diagnosis_data["requires_approval"] = True
            diagnosis_data["approval_id"] = approval_id

        return json.dumps(diagnosis_data)

    @event("prescribe_medicine")
    def prescribe_medicine(self, data):
        """Agiza dawa — tracked on blockchain"""
        payload = json.loads(data) if isinstance(data, str) else data

        prescription = {
            "id": f"RX-{str(uuid.uuid4())[:8]}",
            "patient_id": payload.get("patient_id", ""),
            "doctor": payload.get("doctor", "Dr. AI"),
            "medicine": payload.get("medicine", ""),
            "dosage": payload.get("dosage", ""),
            "duration": payload.get("duration", ""),
            "prescribed_at": time.time()
        }

        # Track on blockchain (anti-counterfeit)
        self.blockchain.add_block(json.dumps({
            "action": "prescription",
            "rx_id": prescription["id"],
            "medicine": prescription["medicine"]
        }))
        self.prescriptions.append(prescription)

        return json.dumps({"status": "prescribed", "rx": prescription})

    @event("lab_result")
    def lab_result(self, data):
        """Ingiza matokeo ya maabara — encrypted"""
        payload = json.loads(data) if isinstance(data, str) else data
        result_id = f"LAB-{str(uuid.uuid4())[:8]}"

        lab = {
            "id": result_id,
            "patient_id": payload.get("patient_id", ""),
            "test_type": payload.get("test", ""),
            "result_encrypted": self.encryption.encrypt(json.dumps(payload.get("results", {}))),
            "tested_at": time.time()
        }
        self.lab_results[result_id] = lab

        return json.dumps({"status": "recorded", "lab_id": result_id})

    @event("generate_bill")
    def generate_bill(self, data):
        """Tengeneza bili ya mgonjwa"""
        payload = json.loads(data) if isinstance(data, str) else data
        bill_id = f"BILL-{str(uuid.uuid4())[:8]}"

        bill = {
            "id": bill_id,
            "patient_id": payload.get("patient_id", ""),
            "items": payload.get("items", []),
            "total": sum(i.get("amount", 0) for i in payload.get("items", [])),
            "currency": "TZS",
            "status": "pending",
            "created_at": time.time()
        }
        self.billing.append(bill)

        return json.dumps({"status": "generated", "bill": bill})

    @event("hospital_status")
    def hospital_status(self, data):
        """Get full hospital statistics"""
        return json.dumps({
            "total_patients": len(self.patients),
            "total_prescriptions": len(self.prescriptions),
            "total_lab_results": len(self.lab_results),
            "pending_approvals": self.hitl.pending_count(),
            "total_billing": sum(b["total"] for b in self.billing),
            "blockchain_records": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0,
            "ai_available": True,
            "encryption": "AES-256 Active"
        })


# ─── RUN ─────────────────────────────────────────────────────
if __name__ == "__main__":
    print("═" * 55)
    print("  🏥 AFYA SMART HOSPITAL — Live Demo")
    print("═" * 55)

    hospital = HospitalSystem()

    # Register patients
    r1 = hospital.register_patient(json.dumps({
        "name": "Fatma Juma", "age": 34, "gender": "Female",
        "phone": "255712345678", "history": "Malaria 2023, BP normal"
    }))
    r2 = hospital.register_patient(json.dumps({
        "name": "Hassan Ali", "age": 45, "gender": "Male",
        "phone": "255798765432", "history": "Diabetes type 2, Hypertension"
    }))
    print(f"\n  ✅ Patient 1: {json.loads(r1)['patient_id']}")
    print(f"  ✅ Patient 2: {json.loads(r2)['patient_id']}")

    # AI Diagnosis
    diag = hospital.ai_diagnose(json.dumps({
        "patient_id": json.loads(r1)['patient_id'],
        "symptoms": "Homa, kichwa, kuhara, kutetemeka"
    }))
    diag_data = json.loads(diag)
    print(f"\n  🧠 AI Diagnosis: {diag_data.get('ai_diagnosis', 'Pending')[:100]}...")
    print(f"  ⚠️  Needs Doctor Approval: {diag_data.get('requires_approval', False)}")

    # Prescribe medicine
    rx = hospital.prescribe_medicine(json.dumps({
        "patient_id": json.loads(r1)['patient_id'],
        "medicine": "Artesunate 100mg", "dosage": "2x daily",
        "duration": "3 days", "doctor": "Dr. AI (pending human review)"
    }))
    print(f"\n  💊 Prescription: {json.loads(rx)['rx']['id']}")

    # Lab results
    lab = hospital.lab_result(json.dumps({
        "patient_id": json.loads(r1)['patient_id'],
        "test": "Malaria Rapid Test",
        "results": {"malaria": "Positive +++", "hb": "11.2 g/dL"}
    }))

    # Generate bill
    bill = hospital.generate_bill(json.dumps({
        "patient_id": json.loads(r1)['patient_id'],
        "items": [
            {"item": "Consultation", "amount": 30000},
            {"item": "Lab Test", "amount": 15000},
            {"item": "Medicine", "amount": 25000},
        ]
    }))

    # Status
    status = json.loads(hospital.hospital_status("{}"))
    print(f"\n  📊 Hospital Stats:")
    print(f"     Patients: {status['total_patients']}")
    print(f"     Prescriptions: {status['total_prescriptions']}")
    print(f"     Total Billing: TZS {status['total_billing']:,.0f}")
    print(f"     Blockchain Records: {status['blockchain_records']}")

    print(f"\n  ✅ Hospital System: FULLY OPERATIONAL")
    print(f"═" * 55)
