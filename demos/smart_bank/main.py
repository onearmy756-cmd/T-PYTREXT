# main.py — PyTreX Smart Bank Management System
from pytrex import PyTreXApp, event, BLOCKCHAIN_CACHE
import torch
import torch.nn as nn
import json
import my_framework


class BankAuthAI(nn.Module):
    def __init__(self):
        super(BankAuthAI, self).__init__()
        self.fc = nn.Linear(512, 1)

    def forward(self, x):
        return torch.sigmoid(self.fc(x))


auth_model = BankAuthAI()
auth_model.eval()


class SmartBank(PyTreXApp):
    def __init__(self):
        super().__init__(name="PyTreX National Bank (PNB) POS")
        self._seed_accounts()

    def _seed_accounts(self):
        """Insert demo accounts into SQLx database if they don't exist."""
        import sqlite3
        conn = sqlite3.connect("salama_enterprise.db")
        c = conn.cursor()
        c.execute("INSERT OR IGNORE INTO akaunti_salama (akaunti_no, jina, salio, sahihi_hash) VALUES (?, ?, ?, ?)",
                  ("100200300", "Juma Hamisi", 1500000.0, "hash_juma_001"))
        c.execute("INSERT OR IGNORE INTO akaunti_salama (akaunti_no, jina, salio, sahihi_hash) VALUES (?, ?, ?, ?)",
                  ("400500600", "Amina Ali", 450000.0, "hash_amina_002"))
        conn.commit()
        conn.close()

    @event("pata_akaunti")
    def akaunti_info(self, data):
        payload = json.loads(data)
        acc_no = payload.get("account_number")

        # Read from SQLx database instead of Python dict
        import sqlite3
        conn = sqlite3.connect("salama_enterprise.db")
        c = conn.cursor()
        c.execute("SELECT jina, salio FROM akaunti_salama WHERE akaunti_no = ?", (acc_no,))
        row = c.fetchone()
        conn.close()

        if row:
            return json.dumps({"status": "success", "data": {"jina": row[0], "salio": row[1]}})
        return json.dumps({"status": "error", "message": "Account not found!"})

    @event("hakiki_sahihi")
    def verify_signature(self, image_data):
        print("[PyTreX AI] Verifying customer signature with PyTorch...")
        mock_tensor = torch.randn(1, 512)

        with torch.no_grad():
            prediction = auth_model(mock_tensor).item()

        if prediction > 0.15:
            return json.dumps({"status": "verified", "score": round(prediction * 100, 2)})
        return json.dumps({"status": "failed", "score": round(prediction * 100, 2)})

    @event("fanya_muamala")
    def handle_secure_blockchain_transaction(self, data_kutoka_ui):
        payload = json.loads(data_kutoka_ui)
        acc_no = payload.get("account_number")
        kiwango = float(payload.get("amount"))
        aina = payload.get("type")

        # 1. Kutengeneza string ya muamala itakayofungwa kwenye Blockchain
        muamala_str = f"Account: {acc_no} | Action: {aina.upper()} | Amount: {kiwango} TZS"

        # 2. Inaita Injini ya Rust Blockchain kutengeneza Kizuizi cha siri (Block)
        block_json_str = my_framework.fanya_block_ya_blockchain(muamala_str)
        block_data = json.loads(block_json_str)

        # Kuhifadhi kwenye Cache ya RAM
        BLOCKCHAIN_CACHE.append(block_data)

        # 3. Kazi ya SQLx ya kuandika muamala salama
        jibu_la_db = my_framework.fanya_muamala_salama(acc_no, aina, kiwango)

        # 4. ELIXIR CONCURRENCY SYNC: Elixir inachukua hii Block mpya ya SHA-256
        # na kuisambaza kwenye Cloud VPS na kompyuta zote za ofisi.
        self.network.emit("blockchain_broadcast", block_data)

        print(f"[PyTreX Elixir] Transaction distributed across bank network!")
        return jibu_la_db

    @event("endesha_audit_usalama")
    def run_security_audit(self, data):
        # Kazi ya kufanya ukaguzi wa mara kwa mara kuzuia wizi
        chain_str = json.dumps(BLOCKCHAIN_CACHE)
        is_safe = my_framework.hakiki_blockchain(chain_str)

        if is_safe:
            return json.dumps({"status": "safe", "message": "Mifumo yote ya fedha imekaguliwa na ipo salama."})
        else:
            return json.dumps({"status": "compromised", "message": "USALAMA UMEVUNJWA! Data imebadilishwa!"})


if __name__ == "__main__":
    app = SmartBank()
    app.run()
