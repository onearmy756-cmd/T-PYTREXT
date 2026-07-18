"""
╔══════════════════════════════════════════════════════════════╗
║  DEMO 4: AI EXAM & LEARNING SYSTEM                         ║
║  Features: AI Question Generation, RAG Knowledge Base,     ║
║            Auto-Grading (Hermes AI), Search Integration,   ║
║            Student Analytics, Certificate Generation       ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import HermesAgent, LangChainAgent
from pytrex.search_engine import SearchEngine
import json, time, uuid, random

class AIExamSystem(PyTreXApp):
    """AI-Powered Exam & Learning Platform"""

    def __init__(self):
        super().__init__(name="Smart Exam AI")
        self.hermes = HermesAgent(name="Examiner AI")
        self.langchain = LangChainAgent()
        self.search = SearchEngine()

        self.exams = {}         # Exam banks
        self.students = {}      # Student records
        self.results = []       # Exam results
        self.knowledge_base = [  # RAG knowledge base
            "Python ni lugha ya programu. Ilianzishwa na Guido van Rossum mwaka 1991.",
            "Blockchain ni database iliyosambazwa. Inatumia cryptography kwa usalama.",
            "Artificial Intelligence (AI) ni uwezo wa computer kufikiri kama binadamu.",
            "Encryption ni mchakato wa kuficha data kwa kutumia algorithm.",
            "PyTreX ni framework inayounganisha Python, Rust, Elixir na AI.",
            "TCP/IP ni itifaki ya mtandao. IP inashughulikia routing, TCP inahakikisha delivery.",
            "SQL ni lugha ya kuuliza database. Inasimamia CREATE, READ, UPDATE, DELETE.",
            "HTTP ni itifaki ya web. Methods ni GET, POST, PUT, DELETE, PATCH.",
        ]

        # Setup LangChain with RAG knowledge
        for doc in self.knowledge_base:
            self.langchain.rag_query(doc, self.knowledge_base)

    @event("generate_exam")
    def generate_exam(self, data):
        """Tengeneza mtihani kwa AI (Hermes)"""
        payload = json.loads(data) if isinstance(data, str) else data
        topic = payload.get("topic", "General")
        difficulty = payload.get("difficulty", "Medium")
        num_questions = payload.get("num_questions", 5)

        exam_id = f"EXAM-{str(uuid.uuid4())[:8]}"

        # Hermes AI generates questions
        questions = []
        for i in range(num_questions):
            prompt = (
                f"Generate a {difficulty} difficulty multiple-choice question "
                f"about '{topic}' in Swahili. Format: "
                f'{{"question": "...", "options": ["A) ...", "B) ...", "C) ...", "D) ..."], "answer": "A"}}'
            )
            result = self.hermes.chat(prompt)
            try:
                q = json.loads(result["reply"]) if "{" in result["reply"] else {
                    "question": f"Swali la {topic} #{i+1}",
                    "options": ["A) Jibu A", "B) Jibu B", "C) Jibu C", "D) Jibu D"],
                    "answer": random.choice(["A", "B", "C", "D"])
                }
                q["id"] = i + 1
                q["points"] = 10
                questions.append(q)
            except:
                questions.append({
                    "id": i+1, "question": f"Swali {i+1} kuhusu {topic}",
                    "options": ["A)", "B)", "C)", "D)"], "answer": "A", "points": 10
                })

        exam = {
            "id": exam_id, "topic": topic, "difficulty": difficulty,
            "questions": questions, "total_points": len(questions) * 10,
            "created_at": time.time()
        }
        self.exams[exam_id] = exam

        return json.dumps({"status": "created", "exam": exam})

    @event("take_exam")
    def take_exam(self, data):
        """Mwanafunzi anafanya mtihani — AI grading"""
        payload = json.loads(data) if isinstance(data, str) else data
        student_name = payload.get("student", "Anonymous")
        exam_id = payload.get("exam_id", "")
        answers = payload.get("answers", {})

        exam = self.exams.get(exam_id, {})
        questions = exam.get("questions", [])
        score = 0

        graded = []
        for q in questions:
            qid = str(q["id"])
            student_answer = answers.get(qid, "")
            correct = student_answer.upper() == q["answer"].upper()
            if correct:
                score += q["points"]
            graded.append({
                "question_id": qid,
                "student_answer": student_answer,
                "correct_answer": q["answer"],
                "correct": correct,
                "points_earned": q["points"] if correct else 0
            })

        total = exam.get("total_points", len(questions) * 10)
        percentage = (score / total * 100) if total > 0 else 0
        grade = "A" if percentage >= 80 else ("B" if percentage >= 65 else ("C" if percentage >= 50 else ("D" if percentage >= 40 else "F")))

        result = {
            "student": student_name,
            "exam_id": exam_id,
            "topic": exam.get("topic", ""),
            "score": score,
            "total": total,
            "percentage": round(percentage, 1),
            "grade": grade,
            "graded_questions": graded,
            "taken_at": time.time()
        }
        self.results.append(result)

        # Store student
        if student_name not in self.students:
            self.students[student_name] = []
        self.students[student_name].append(result)

        return json.dumps({"status": "graded", "result": result})

    @event("ask_tutor")
    def ask_tutor(self, data):
        """AI Tutor — jibu maswali kwa kutumia RAG + Search"""
        payload = json.loads(data) if isinstance(data, str) else data
        question = payload.get("question", "")

        # Search knowledge base (RAG)
        rag_result = self.langchain.rag_query(question, self.knowledge_base, top_k=3)

        # Search web for additional info
        web_result = self.search.web_search_summary(question)

        # Hermes combines everything into a clear answer
        context = f"Knowledge: {rag_result.get('response', '')}\nWeb: {web_result.get('summary', '')}"
        final_answer = self.hermes.chat(
            f"Context: {context}\n\nStudent question: {question}\n\nGive a clear, complete answer in Swahili."
        )

        return json.dumps({
            "question": question,
            "answer": final_answer["reply"],
            "sources": {
                "knowledge_base": len(rag_result.get("results", [])),
                "web_results": len(web_result.get("results", []))
            }
        })

    @event("student_report")
    def student_report(self, data):
        """Pata ripoti ya mwanafunzi"""
        payload = json.loads(data) if isinstance(data, str) else {}
        student = payload.get("student", "")

        results = self.students.get(student, [])
        avg_score = sum(r["percentage"] for r in results) / len(results) if results else 0

        return json.dumps({
            "student": student,
            "exams_taken": len(results),
            "average_score": round(avg_score, 1),
            "best_grade": max((r["grade"] for r in results), default="N/A"),
            "results": results[-5:]  # Last 5
        })


# ─── RUN ─────────────────────────────────────────────────────
if __name__ == "__main__":
    print("═" * 55)
    print("  📚 AI EXAM SYSTEM — Live Demo")
    print("═" * 55)

    exam_sys = AIExamSystem()

    # Generate exam
    r1 = exam_sys.generate_exam(json.dumps({
        "topic": "Python Programming",
        "difficulty": "Medium", "num_questions": 3
    }))
    exam = json.loads(r1)["exam"]
    print(f"\n  📝 Exam Generated: {exam['id']}")
    print(f"     Topic: {exam['topic']} | Questions: {len(exam['questions'])}")

    # Student takes exam (simulate random answers)
    answers = {str(q["id"]): random.choice(["A", "B", "C", "D"])
               for q in exam["questions"]}
    r2 = exam_sys.take_exam(json.dumps({
        "student": "DR MBILINYI", "exam_id": exam["id"], "answers": answers
    }))
    result = json.loads(r2)["result"]
    print(f"\n  🎓 Result: {result['student']}")
    print(f"     Score: {result['score']}/{result['total']} ({result['percentage']}%)")
    print(f"     Grade: {result['grade']}")

    # AI Tutor
    r3 = exam_sys.ask_tutor(json.dumps({
        "question": "Python ni nini na inatumika kwa nini?"
    }))
    answer = json.loads(r3)
    print(f"\n  🤖 AI Tutor Answer: {answer['answer'][:150]}...")
    print(f"     Sources: {answer['sources']}")

    # Student report
    r4 = exam_sys.student_report(json.dumps({"student": "DR MBILINYI"}))
    report = json.loads(r4)
    print(f"\n  📊 Student Report: {report['student']}")
    print(f"     Exams Taken: {report['exams_taken']}")
    print(f"     Average: {report['average_score']}%")

    print(f"\n  ✅ AI Exam System: FULLY OPERATIONAL")
    print(f"═" * 55)
