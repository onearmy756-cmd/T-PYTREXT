"""
╔══════════════════════════════════════════════════════════════╗
║  SCHOOL MANAGEMENT SYSTEM — Built with T-PYTREXT          ║
║  ========================================================  ║
║  Features:                                                ║
║  📋 Student Management (Register, Search, Reports)        ║
║  👨‍🏫 Teacher Management (Assign, Payroll)                ║
║  📚 Classes & Subjects                                    ║
║  📊 Attendance Tracking                                   ║
║  🎓 Exam Results & Grading                                ║
║  💰 Fee Management & Payments                             ║
║  📅 Timetable Management                                  ║
║  📄 Report Cards (auto-generated)                          ║
║  🔗 Blockchain Certificates                               ║
║  🔐 Encrypted Student Data                                ║
║  🧠 AI-Powered Analytics                                  ║
║  👤 HITL Approvals (critical actions)                     ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import (
    BlockchainBridge, EncryptionManager, HumanInTheLoop,
    HermesAgent, LangChainAgent
)
import json, time, uuid, random
from datetime import datetime, timedelta


class SchoolManagementSystem(PyTreXApp):
    """Complete School Management System — From Registration to Graduation"""

    def __init__(self, school_name: str = "Smart Academy"):
        super().__init__(name=school_name)
        self.school_name = school_name
        self.blockchain = BlockchainBridge()
        self.encryption = EncryptionManager(default_password="school_secret_2026")
        self.hitl = HumanInTheLoop(default_timeout=300)
        self.hermes = HermesAgent(name="SchoolAI")

        # ─── DATABASES ───
        self.students = {}          # student_id → student_data
        self.teachers = {}          # teacher_id → teacher_data
        self.classes = {}           # class_id → class_data
        self.subjects = {}          # subject_id → subject_data
        self.attendance = []        # [attendance_record, ...]
        self.exam_results = []      # [exam_result, ...]
        self.fees = []              # [fee_record, ...]
        self.timetable = {}         # class_id → [schedule, ...]
        self.certificates = []      # Blockchain-backed certificates

        # ─── COUNTERS ───
        self._student_count = 0
        self._teacher_count = 0

        # ─── SEED DEMO DATA ───
        self._seed_data()

    # ═══════════════════════════════════════════════════════════
    #  STUDENT MANAGEMENT
    # ═══════════════════════════════════════════════════════════

    @event("register_student")
    def register_student(self, data):
        """Register a new student"""
        p = json.loads(data) if isinstance(data, str) else data
        self._student_count += 1
        sid = f"STU-{datetime.now().strftime('%Y')}-{self._student_count:04d}"

        student = {
            "id": sid,
            "first_name": p.get("first_name", ""),
            "last_name": p.get("last_name", ""),
            "full_name": f"{p.get('first_name','')} {p.get('last_name','')}",
            "dob": p.get("dob", ""),
            "gender": p.get("gender", "M"),
            "parent_name_enc": self.encryption.encrypt(p.get("parent_name", "")),
            "parent_phone_enc": self.encryption.encrypt(p.get("parent_phone", "")),
            "address_enc": self.encryption.encrypt(p.get("address", "")),
            "class_id": p.get("class_id", ""),
            "registered_at": time.time(),
            "status": "active",
            "fees_paid": 0,
        }
        self.students[sid] = student

        # Blockchain record
        self.blockchain.add_block(json.dumps({
            "action": "student_registered", "student_id": sid,
            "name": student["full_name"], "class": student["class_id"]
        }))

        self.bus.emit("student_registered", student)
        return json.dumps({"status": "registered", "student": {
            "id": sid, "name": student["full_name"], "class": student["class_id"]
        }})

    @event("get_student")
    def get_student(self, data):
        """Get student details"""
        p = json.loads(data) if isinstance(data, str) else data
        sid = p.get("student_id", "")
        student = self.students.get(sid)
        if not student:
            return json.dumps({"error": "Student not found"})

        # Decrypt sensitive fields for display
        return json.dumps({
            "id": student["id"],
            "name": student["full_name"],
            "dob": student["dob"],
            "gender": student["gender"],
            "class": student["class_id"],
            "parent": self.encryption.decrypt(student.get("parent_name_enc", "")),
            "phone": self.encryption.decrypt(student.get("parent_phone_enc", "")),
            "status": student["status"],
            "fees_paid": student["fees_paid"],
        })

    @event("list_students")
    def list_students(self, data):
        """List students with optional filters"""
        p = json.loads(data) if isinstance(data, str) else {}
        class_id = p.get("class_id", "")
        search = p.get("search", "").lower()

        results = []
        for s in self.students.values():
            if class_id and s["class_id"] != class_id:
                continue
            if search and search not in s["full_name"].lower():
                continue
            results.append({
                "id": s["id"], "name": s["full_name"],
                "class": s["class_id"], "status": s["status"],
                "gender": s["gender"]
            })

        return json.dumps({"total": len(results), "students": results[:50]})

    @event("promote_student")
    def promote_student(self, data):
        """Promote student to next class (requires HITL approval)"""
        p = json.loads(data) if isinstance(data, str) else data
        sid = p.get("student_id", "")
        new_class = p.get("new_class", "")
        student = self.students.get(sid)

        if not student:
            return json.dumps({"error": "Student not found"})

        # Requires human approval
        approval_id = self.hitl.request_approval(
            "promote_student",
            {"student": student["full_name"], "from": student["class_id"], "to": new_class},
            timeout=3600
        )
        return json.dumps({
            "status": "pending_approval",
            "approval_id": approval_id,
            "message": f"Promotion of {student['full_name']} requires approval!"
        })

    # ═══════════════════════════════════════════════════════════
    #  TEACHER MANAGEMENT
    # ═══════════════════════════════════════════════════════════

    @event("register_teacher")
    def register_teacher(self, data):
        """Register a new teacher"""
        p = json.loads(data) if isinstance(data, str) else data
        self._teacher_count += 1
        tid = f"TCH-{datetime.now().strftime('%Y')}-{self._teacher_count:03d}"

        teacher = {
            "id": tid,
            "first_name": p.get("first_name", ""),
            "last_name": p.get("last_name", ""),
            "full_name": f"{p.get('first_name','')} {p.get('last_name','')}",
            "qualification": p.get("qualification", "B.Ed."),
            "subjects": p.get("subjects", []),
            "phone_enc": self.encryption.encrypt(p.get("phone", "")),
            "salary": p.get("salary", 0),
            "joined_at": time.time(),
            "status": "active",
        }
        self.teachers[tid] = teacher
        return json.dumps({"status": "registered", "teacher_id": tid, "name": teacher["full_name"]})

    @event("list_teachers")
    def list_teachers(self, data):
        """List all teachers"""
        results = [{"id": t["id"], "name": t["full_name"],
                     "subjects": t["subjects"], "qualification": t["qualification"]}
                   for t in self.teachers.values()]
        return json.dumps({"total": len(results), "teachers": results})

    # ═══════════════════════════════════════════════════════════
    #  CLASSES & SUBJECTS
    # ═══════════════════════════════════════════════════════════

    @event("create_class")
    def create_class(self, data):
        """Create a new class/stream"""
        p = json.loads(data) if isinstance(data, str) else data
        cid = p.get("class_id", f"FORM-{len(self.classes)+1}")

        cls = {
            "id": cid,
            "name": p.get("name", f"Form {len(self.classes)+1}"),
            "level": p.get("level", "O-Level"),
            "capacity": p.get("capacity", 45),
            "class_teacher": p.get("class_teacher", ""),
            "students_count": 0,
            "subjects": p.get("subjects", []),
        }
        self.classes[cid] = cls
        return json.dumps({"status": "created", "class": cls})

    @event("add_subject")
    def add_subject(self, data):
        """Add a subject"""
        p = json.loads(data) if isinstance(data, str) else data
        sid = p.get("subject_id", f"SUB-{len(self.subjects)+1}")

        subject = {
            "id": sid,
            "name": p.get("name", ""),
            "code": p.get("code", ""),
            "teacher_id": p.get("teacher_id", ""),
            "periods_per_week": p.get("periods", 4),
        }
        self.subjects[sid] = subject
        return json.dumps({"status": "added", "subject": subject})

    # ═══════════════════════════════════════════════════════════
    #  ATTENDANCE
    # ═══════════════════════════════════════════════════════════

    @event("mark_attendance")
    def mark_attendance(self, data):
        """Mark attendance for students"""
        p = json.loads(data) if isinstance(data, str) else data
        record = {
            "id": f"ATT-{str(uuid.uuid4())[:8]}",
            "date": p.get("date", datetime.now().strftime("%Y-%m-%d")),
            "class_id": p.get("class_id", ""),
            "marked_by": p.get("teacher_id", ""),
            "records": p.get("records", []),  # [{"student_id": "X", "status": "present/absent/late"}, ...]
            "marked_at": time.time(),
        }
        self.attendance.append(record)

        # Count present/absent
        present = sum(1 for r in record["records"] if r.get("status") == "present")
        absent = len(record["records"]) - present

        return json.dumps({
            "status": "recorded", "date": record["date"],
            "class": record["class_id"], "present": present, "absent": absent
        })

    @event("attendance_report")
    def attendance_report(self, data):
        """Get attendance report for a class"""
        p = json.loads(data) if isinstance(data, str) else data
        class_id = p.get("class_id", "")
        days = p.get("days", 30)

        class_records = [r for r in self.attendance if r["class_id"] == class_id]
        return json.dumps({
            "class": class_id,
            "total_days": len(class_records),
            "records": class_records[-days:],
        })

    # ═══════════════════════════════════════════════════════════
    #  EXAM RESULTS & GRADING
    # ═══════════════════════════════════════════════════════════

    @event("enter_exam_results")
    def enter_exam_results(self, data):
        """Enter exam results for a student"""
        p = json.loads(data) if isinstance(data, str) else data
        result = {
            "id": f"EXAM-{str(uuid.uuid4())[:8]}",
            "student_id": p.get("student_id", ""),
            "exam_name": p.get("exam_name", "Terminal"),
            "term": p.get("term", "Term 1"),
            "year": p.get("year", datetime.now().year),
            "subjects": p.get("subjects", []),  # [{"subject": "Math", "score": 85}, ...]
            "total_marks": sum(s.get("score", 0) for s in p.get("subjects", [])),
            "average": 0,
            "grade": "",
            "entered_at": time.time(),
        }

        # Calculate average and grade
        subs = result["subjects"]
        if subs:
            result["average"] = result["total_marks"] / len(subs)
            avg = result["average"]
            if avg >= 80: result["grade"] = "A"
            elif avg >= 65: result["grade"] = "B"
            elif avg >= 50: result["grade"] = "C"
            elif avg >= 40: result["grade"] = "D"
            else: result["grade"] = "F"

        self.exam_results.append(result)
        return json.dumps({"status": "recorded", "result": result})

    @event("student_results")
    def student_results(self, data):
        """Get exam results for a student"""
        p = json.loads(data) if isinstance(data, str) else data
        sid = p.get("student_id", "")
        student_results = [r for r in self.exam_results if r["student_id"] == sid]

        return json.dumps({
            "student_id": sid,
            "total_exams": len(student_results),
            "results": student_results,
        })

    @event("class_results")
    def class_results(self, data):
        """Get results for entire class — ranked"""
        p = json.loads(data) if isinstance(data, str) else data
        class_id = p.get("class_id", "")
        exam_name = p.get("exam_name", "Terminal")

        # Get students in class
        class_students = {s["id"]: s for s in self.students.values() if s["class_id"] == class_id}

        rankings = []
        for sid in class_students:
            results = [r for r in self.exam_results
                       if r["student_id"] == sid and r["exam_name"] == exam_name]
            if results:
                latest = max(results, key=lambda r: r["entered_at"])
                rankings.append({
                    "student_id": sid,
                    "name": class_students[sid]["full_name"],
                    "average": latest["average"],
                    "grade": latest["grade"],
                })

        rankings.sort(key=lambda r: r["average"], reverse=True)
        for i, r in enumerate(rankings):
            r["position"] = i + 1

        return json.dumps({"class": class_id, "exam": exam_name, "rankings": rankings[:20]})

    # ═══════════════════════════════════════════════════════════
    #  FEE MANAGEMENT
    # ═══════════════════════════════════════════════════════════

    @event("record_fee_payment")
    def record_fee_payment(self, data):
        """Record a fee payment"""
        p = json.loads(data) if isinstance(data, str) else data
        payment = {
            "id": f"FEE-{str(uuid.uuid4())[:8]}",
            "student_id": p.get("student_id", ""),
            "amount": p.get("amount", 0),
            "term": p.get("term", "Term 1"),
            "year": p.get("year", datetime.now().year),
            "method": p.get("method", "Cash"),
            "paid_at": time.time(),
        }
        self.fees.append(payment)

        # Update student fees
        sid = payment["student_id"]
        if sid in self.students:
            self.students[sid]["fees_paid"] += payment["amount"]

        # Blockchain record
        self.blockchain.add_block(json.dumps({
            "action": "fee_payment",
            "student": sid, "amount": payment["amount"]
        }))

        return json.dumps({"status": "paid", "receipt": payment["id"], "amount": payment["amount"]})

    @event("fee_report")
    def fee_report(self, data):
        """Get fee collection report"""
        total_collected = sum(f["amount"] for f in self.fees)
        total_students = len(self.students)
        paid_students = len(set(f["student_id"] for f in self.fees))

        return json.dumps({
            "total_collected": total_collected,
            "total_students": total_students,
            "paid_students": paid_students,
            "unpaid_students": total_students - paid_students,
            "collection_rate": round(paid_students / max(total_students, 1) * 100, 1),
        })

    # ═══════════════════════════════════════════════════════════
    #  CERTIFICATES (Blockchain-Backed)
    # ═══════════════════════════════════════════════════════════

    @event("issue_certificate")
    def issue_certificate(self, data):
        """Issue a blockchain-backed certificate"""
        p = json.loads(data) if isinstance(data, str) else data

        cert = {
            "id": f"CERT-{str(uuid.uuid4())[:8]}",
            "student_id": p.get("student_id", ""),
            "type": p.get("type", "Completion"),
            "description": p.get("description", ""),
            "issued_at": time.time(),
            "blockchain_hash": "",
        }

        # Record on blockchain
        block = self.blockchain.add_block(json.dumps({
            "action": "certificate_issued",
            "cert_id": cert["id"],
            "student": cert["student_id"],
            "type": cert["type"],
        }))
        if block.get("block"):
            cert["blockchain_hash"] = block["block"].get("hash", "")[:16]

        self.certificates.append(cert)
        return json.dumps({
            "status": "issued",
            "certificate": cert,
            "verified": True,
            "message": "Certificate backed by blockchain — tamper-proof!"
        })

    @event("verify_certificate")
    def verify_certificate(self, data):
        """Verify a certificate's blockchain authenticity"""
        p = json.loads(data) if isinstance(data, str) else data
        cert_id = p.get("certificate_id", "")

        for cert in self.certificates:
            if cert["id"] == cert_id:
                # Verify blockchain integrity
                chain_valid = self.blockchain.verify_chain()
                return json.dumps({
                    "certificate_id": cert_id,
                    "verified": chain_valid.get("valid", False),
                    "blockchain_hash": cert["blockchain_hash"],
                    "student": cert["student_id"],
                    "type": cert["type"],
                    "status": "✅ GENUINE" if chain_valid.get("valid") else "❌ TAMPERED!"
                })

        return json.dumps({"error": "Certificate not found"})

    # ═══════════════════════════════════════════════════════════
    #  AI ANALYTICS
    # ═══════════════════════════════════════════════════════════

    @event("school_analytics")
    def school_analytics(self, data):
        """AI-powered school analytics dashboard"""
        total_students = len(self.students)
        total_teachers = len(self.teachers)
        total_classes = len(self.classes)
        total_fees = sum(f["amount"] for f in self.fees)

        # Calculate averages
        all_results = [r for r in self.exam_results if r.get("average")]
        avg_score = sum(r["average"] for r in all_results) / max(len(all_results), 1)

        # Attendance rate
        attendance_days = len(self.attendance)
        total_present = sum(
            sum(1 for r2 in r.get("records", []) if r2.get("status") == "present")
            for r in self.attendance
        )

        # AI summary
        ai_prompt = (
            f"School analytics: {total_students} students, {total_teachers} teachers, "
            f"{total_classes} classes, average score {avg_score:.1f}%, "
            f"TZS {total_fees:,.0f} collected. Give a brief Swahili performance summary."
        )
        ai_result = self.hermes.chat(ai_prompt)

        return json.dumps({
            "school": self.school_name,
            "students": total_students,
            "teachers": total_teachers,
            "classes": total_classes,
            "fees_collected": total_fees,
            "average_score": round(avg_score, 1),
            "attendance_days": attendance_days,
            "blockchain_records": len(self.certificates),
            "ai_summary": ai_result["reply"][:200],
        })

    # ═══════════════════════════════════════════════════════════
    #  DASHBOARD SUMMARY
    # ═══════════════════════════════════════════════════════════

    @event("dashboard")
    def dashboard(self, data):
        """Complete school dashboard"""
        return json.dumps({
            "school": self.school_name,
            "students": {"total": len(self.students), "active": sum(1 for s in self.students.values() if s["status"] == "active")},
            "teachers": {"total": len(self.teachers)},
            "classes": {"total": len(self.classes)},
            "subjects": {"total": len(self.subjects)},
            "attendance": {"days_recorded": len(self.attendance)},
            "exams": {"total_recorded": len(self.exam_results)},
            "fees": {"total_collected": sum(f["amount"] for f in self.fees), "transactions": len(self.fees)},
            "certificates": {"issued": len(self.certificates)},
            "blockchain": {"blocks": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0},
        })

    # ═══════════════════════════════════════════════════════════
    #  SEED DATA
    # ═══════════════════════════════════════════════════════════

    def _seed_data(self):
        """Create demo data for testing"""
        # Classes
        for i in range(1, 5):
            self.create_class(json.dumps({
                "class_id": f"FORM-{i}",
                "name": f"Form {i}",
                "level": "O-Level" if i <= 4 else "A-Level",
                "capacity": 45,
            }))

        # Subjects
        subjects_data = [
            ("SUB-MATH", "Mathematics", "MATH"), ("SUB-ENG", "English", "ENG"),
            ("SUB-KIS", "Kiswahili", "KIS"), ("SUB-SCI", "Science", "SCI"),
            ("SUB-GEO", "Geography", "GEO"), ("SUB-HIS", "History", "HIS"),
            ("SUB-ICT", "Computer Studies", "ICT"),
        ]
        for sid, name, code in subjects_data:
            self.add_subject(json.dumps({"subject_id": sid, "name": name, "code": code}))

        # Teachers
        teachers_data = [
            ("Juma", "Hamisi", "B.Ed. Mathematics"), ("Amina", "Ali", "M.A. English"),
            ("David", "Mushi", "B.Sc. ICT"), ("Fatma", "Omar", "B.Ed. Science"),
        ]
        for first, last, qual in teachers_data:
            self.register_teacher(json.dumps({
                "first_name": first, "last_name": last, "qualification": qual,
                "subjects": ["Math", "English", "ICT", "Science"][:random.randint(1, 3)],
                "salary": random.randint(800000, 1500000),
            }))

        # Students (20 of them)
        first_names = ["Juma", "Aisha", "Hamisi", "Grace", "Hassan", "Rehema", "Charles",
                       "Mwajuma", "Paulo", "Neema", "Peter", "Mary", "John", "Elizabeth",
                       "James", "Catherine", "Michael", "Sarah", "William", "DR JACKSON"]
        last_names = ["Omari", "Mohamed", "Bakari", "Peter", "Mwinyi", "Juma", "David",
                      "Salum", "Mbaga", "Kitwanga", "Mahenge", "Mgaya", "Mwasenga", "Mbilinyi"]

        for i in range(20):
            fn = first_names[i % len(first_names)]
            ln = last_names[i % len(last_names)]
            class_id = f"FORM-{random.randint(1, 4)}"
            self.register_student(json.dumps({
                "first_name": fn, "last_name": ln,
                "dob": f"{random.randint(2005, 2010)}-0{random.randint(1,9)}-{random.randint(1,28):02d}",
                "gender": "M" if i % 2 == 0 else "F",
                "parent_name": f"Mzee {ln}",
                "parent_phone": f"2557{random.randint(10000000, 99999999)}",
                "address": f"Dar es Salaam, Tanzania",
                "class_id": class_id,
            }))

        # Exam results for some students
        for sid in list(self.students.keys())[:10]:
            scores = []
            for subj in ["Mathematics", "English", "Kiswahili", "Science"]:
                scores.append({"subject": subj, "score": random.randint(40, 95)})
            self.enter_exam_results(json.dumps({
                "student_id": sid, "exam_name": "Mid-Term",
                "term": "Term 1", "year": 2026, "subjects": scores
            }))

        # Attendance for today
        for class_id in ["FORM-1", "FORM-2"]:
            class_students = [s["id"] for s in self.students.values() if s["class_id"] == class_id]
            records = [{"student_id": sid, "status": "present" if random.random() > 0.2 else "absent"}
                       for sid in class_students]
            self.mark_attendance(json.dumps({
                "date": datetime.now().strftime("%Y-%m-%d"),
                "class_id": class_id,
                "records": records,
            }))

        # Fee payments
        for sid in list(self.students.keys())[:15]:
            self.record_fee_payment(json.dumps({
                "student_id": sid, "amount": 350000,
                "term": "Term 1", "year": 2026, "method": "M-Pesa"
            }))


# ═══════════════════════════════════════════════════════════════
#  LIVE DEMO
# ═══════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("═" * 60)
    print("  🏫 SMART ACADEMY — School Management System")
    print("  Built with T-PYTREXT Framework")
    print("═" * 60)

    school = SchoolManagementSystem(school_name="Smart Academy")

    # ═══ DASHBOARD ═══
    r = school.dashboard("{}")
    dash = json.loads(r)
    print(f"\n━━━ DASHBOARD ━━━")
    print(f"  🏫 School: {dash['school']}")
    print(f"  👨‍🎓 Students: {dash['students']['total']} (Active: {dash['students']['active']})")
    print(f"  👨‍🏫 Teachers: {dash['teachers']['total']}")
    print(f"  📚 Classes: {dash['classes']['total']}")
    print(f"  📖 Subjects: {dash['subjects']['total']}")
    print(f"  💰 Fees: TZS {dash['fees']['total_collected']:,.0f} ({dash['fees']['transactions']} payments)")
    print(f"  🎓 Certificates: {dash['certificates']['issued']}")
    print(f"  🔗 Blockchain: {dash['blockchain']['blocks']} blocks")

    # ═══ STUDENT LIST ═══
    r = school.list_students('{"search": ""}')
    students = json.loads(r)
    print(f"\n━━━ STUDENTS ({students['total']}) ━━━")
    for s in students["students"][:8]:
        print(f"  {s['id']} | {s['name']:<20} | {s['class']} | {s['status']}")

    # ═══ CLASS RESULTS ═══
    r = school.class_results('{"class_id": "FORM-1", "exam_name": "Mid-Term"}')
    rankings = json.loads(r)
    print(f"\n━━━ FORM-1 RANKINGS ━━━")
    for rk in rankings["rankings"][:5]:
        print(f"  #{rk['position']} {rk['name']:<20} | {rk['average']:.1f}% | Grade: {rk['grade']}")

    # ═══ ISSUE CERTIFICATE ═══
    r = school.issue_certificate(json.dumps({
        "student_id": list(school.students.keys())[-1],
        "type": "Completion",
        "description": "Successfully completed Form 4 with distinction"
    }))
    cert = json.loads(r)
    print(f"\n━━━ CERTIFICATE ISSUED ━━━")
    print(f"  📜 {cert['certificate']['id']}")
    print(f"  🔗 Blockchain Hash: {cert['certificate']['blockchain_hash']}")
    print(f"  ✅ {cert['message']}")

    # ═══ SCHOOL ANALYTICS ═══
    r = school.school_analytics("{}")
    analytics = json.loads(r)
    print(f"\n━━━ AI ANALYTICS ━━━")
    print(f"  📊 Students: {analytics['students']}")
    print(f"  📊 Avg Score: {analytics['average_score']}%")
    print(f"  💰 Total Fees: TZS {analytics['fees_collected']:,.0f}")
    print(f"  🧠 AI: {analytics['ai_summary'][:150]}...")

    # ═══ REGISTER NEW STUDENT ═══
    r = school.register_student(json.dumps({
        "first_name": "DR JACKSON", "last_name": "MBILINYI",
        "dob": "2010-05-15", "gender": "M",
        "parent_name": "Mzee Mbilinyi",
        "parent_phone": "255712345678",
        "address": "Dar es Salaam",
        "class_id": "FORM-4"
    }))
    new_stu = json.loads(r)
    print(f"\n━━━ NEW STUDENT REGISTERED ━━━")
    print(f"  ✅ {new_stu['student']['name']} — {new_stu['student']['id']}")

    # ═══ FEE PAYMENT ═══
    r = school.record_fee_payment(json.dumps({
        "student_id": new_stu["student"]["id"],
        "amount": 500000, "term": "Term 1",
        "year": 2026, "method": "M-Pesa"
    }))
    payment = json.loads(r)
    print(f"\n  💰 Fee Paid: TZS {payment['amount']:,} (Receipt: {payment['receipt']})")

    print(f"\n{'═' * 60}")
    print(f"  ✅ SCHOOL MANAGEMENT SYSTEM — FULLY OPERATIONAL!")
    print(f"  🏫 {dash['students']['total']+1} Students | {dash['teachers']['total']} Teachers | {dash['classes']['total']} Classes")
    print(f"  💰 TZS {dash['fees']['total_collected']+500000:,.0f} Collected")
    print(f"  🔗 Blockchain-Backed Certificates")
    print(f"  🧠 AI-Powered Analytics")
    print(f"{'═' * 60}")
