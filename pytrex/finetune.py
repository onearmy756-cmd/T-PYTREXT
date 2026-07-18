import json
import os
import torch
from torch.utils.data import Dataset


class SwahiliTextDataset(Dataset):
    def __init__(self, json_path, tokenizer, max_length=512):
        with open(json_path, "r", encoding="utf-8") as f:
            self.data = json.load(f)
        self.tokenizer = tokenizer
        self.max_length = max_length

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx):
        text = self.data[idx]["text"]
        encoding = self.tokenizer(
            text,
            truncation=True,
            max_length=self.max_length,
            padding="max_length",
            return_tensors="pt",
        )
        return {
            "input_ids": encoding["input_ids"].flatten(),
            "attention_mask": encoding["attention_mask"].flatten(),
            "labels": encoding["input_ids"].flatten(),
        }


class PyTrexFineTuner:
    def __init__(self, model_id="facebook/opt-125m", output_dir="./my_trained_model"):
        self.model_id = model_id
        self.output_dir = output_dir
        self.model = None
        self.tokenizer = None
        self.trainer = None
        print(f"[PyTreX Fine-Tune] Initializing Fine-Tuning Studio with model: {model_id}")

    def load_model(self):
        from transformers import AutoModelForCausalLM, AutoTokenizer

        print(f"[PyTreX Fine-Tune] Loading model {self.model_id}...")
        self.tokenizer = AutoTokenizer.from_pretrained(self.model_id)
        self.model = AutoModelForCausalLM.from_pretrained(self.model_id)
        print("[PyTreX Fine-Tune] Model loaded successfully!")

    def load_dataset(self, json_path):
        if self.tokenizer is None:
            self.load_model()
        dataset = SwahiliTextDataset(json_path, self.tokenizer)
        print(f"[PyTreX Fine-Tune] Dataset loaded: {len(dataset)} samples from {json_path}")
        return dataset

    def train(self, json_path, num_epochs=3, batch_size=2, save_steps=50):
        from transformers import Trainer, TrainingArguments

        if self.model is None:
            self.load_model()

        dataset = self.load_dataset(json_path)

        training_args = TrainingArguments(
            output_dir=self.output_dir,
            num_train_epochs=num_epochs,
            per_device_train_batch_size=batch_size,
            save_steps=save_steps,
            save_total_limit=2,
            logging_steps=10,
            report_to="none",
        )

        progress_callback = PyTrexProgressCallback()

        self.trainer = Trainer(
            model=self.model,
            args=training_args,
            train_dataset=dataset,
            callbacks=[progress_callback],
        )

        print("[PyTreX Fine-Tune] Starting training...")
        self.trainer.train()
        print("[PyTreX Fine-Tune] Training complete!")

        final_model_path = os.path.join(self.output_dir, "final_model")
        self.model.save_pretrained(final_model_path)
        self.tokenizer.save_pretrained(final_model_path)
        print(f"[PyTreX Fine-Tune] Model saved to {final_model_path}")

        return final_model_path


class PyTrexProgressCallback:
    """Callback inayoshika data ya Loss na kuirusha kwenda kwenye Elixir Engine."""

    def __init__(self, network=None):
        self.network = network
        self.losses = []

    def on_log(self, args, state, control, logs=None, **kwargs):
        if logs and "loss" in logs:
            loss = logs["loss"]
            step = state.global_step
            self.losses.append({"step": step, "loss": loss})
            print(f"[PyTreX Progress] Step {step} | Loss: {loss}")

            if self.network:
                self.network.emit("training_progress", {
                    "step": step,
                    "loss": loss,
                })
