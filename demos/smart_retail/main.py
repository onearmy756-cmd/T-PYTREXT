# main.py — PyTreX Smart Retail POS System
from pytrex import PyTreXApp, event
import torch
import json


class SmartDuka(PyTreXApp):
    def __init__(self):
        super().__init__(name="Smart Duka POS v1.0")
        self.model = None
        self.kuandaa_ai_kamera()

    def kuandaa_ai_kamera(self):
        print("[Smart Duka AI] Inapakia model ya PyTorch ya kutambua bidhaa...")
        try:
            self.model = torch.hub.load('ultralytics/yolov5', 'yolov5s', pretrained=True)
            print("[Smart Duka AI] Model ipo tayari!")
        except Exception as e:
            print(f"[Smart Duka AI] Model haijapakia: {e}")
            print("[Smart Duka AI] Inatumia mode ya majaribio (mock detection).")

    @event("scan_product")
    def chakata_picha_ya_kamera(self, data):
        payload = json.loads(data)
        image_path = payload.get("image_path", "")

        bidhaa_zilizopatikana = []
        if self.model is not None:
            try:
                import cv2
                frame = cv2.imread(image_path)
                if frame is not None:
                    results = self.model(frame)
                    bidhaa_zilizopatikana = results.pandas().xyxy[0]['name'].tolist()
            except Exception as e:
                print(f"[Smart Duka AI] Detection error: {e}")
                bidhaa_zilizopatikana = ["bottle", "cup"]
        else:
            bidhaa_zilizopatikana = ["bottle", "cup"]

        print(f"[Smart Duka AI] Bidhaa zilizotambuliwa: {bidhaa_zilizopatikana}")

        # Kutuma data kwenda kwenye Elixir ili isambazwe kwenye mfumo wa stoo na mtandao
        for bidhaa in bidhaa_zilizopatikana:
            self.network.emit("mauzo_mapya", {"bidhaa": bidhaa, "bei": 1500})

        return json.dumps({"status": "success", "items": bidhaa_zilizopatikana})


if __name__ == "__main__":
    app = SmartDuka()
    app.run()
