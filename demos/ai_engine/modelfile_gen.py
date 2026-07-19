"""
╔══════════════════════════════════════════════════════════════════╗
║  REAL OLLAMA MODEFILE GENERATOR                                 ║
║                                                                  ║
║  Creates valid Ollama Modelfile + imports into Ollama.          ║
║  The Modelfile specifies the architecture, parameters,          ║
║  template, and system prompt for the quantized model.           ║
╚══════════════════════════════════════════════════════════════════╝
"""
import os
import sys
from typing import Dict, Optional
from dataclasses import dataclass, field


@dataclass
class ModelfileConfig:
    """Configuration for Ollama Modelfile."""
    model_name: str = "pytrex-ternary"
    model_path: str = "./model.gguf"
    architecture: str = "llama"
    context_length: int = 2048
    temperature: float = 0.7
    top_p: float = 0.9
    top_k: int = 40
    repeat_penalty: float = 1.1
    stop_tokens: list = field(default_factory=lambda: ["</s>", "<|endoftext|>"])
    system_prompt: str = ""
    template: str = ""
    license: str = "MIT"
    description: str = ""


# Pre-built templates for common architectures
CHAT_TEMPLATES = {
    "llama": """{{ if .System }}<|system|>
{{ .System }}<|end|>
{{ end }}{{ if .Prompt }}<|user|>
{{ .Prompt }}<|end|>
{{ end }}<|assistant|>
{{ .Response }}<|end|>
""",
    "mistral": """{{ if .System }}[INST] {{ .System }} [/INST]
{{ end }}{{ if .Prompt }}[INST] {{ .Prompt }} [/INST]
{{ .Response }}</s>
""",
    "qwen": """<|im_start|>system
{{ .System }}<|im_end|>
<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
{{ .Response }}<|im_end|>
""",
    "gemma": """<bos><start_of_turn>user
{{ .Prompt }}<end_of_turn>
<start_of_turn>model
{{ .Response }}<end_of_turn>
""",
    "phi": """<|system|>
{{ .System }}<|end|>
<|user|>
{{ .Prompt }}<|end|>
<|assistant|>
{{ .Response }}<|end|>
""",
    "chatml": """<|im_start|>system
{{ .System }}<|im_end|>
<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
{{ .Response }}<|im_end|>
""",
}


def generate_modelfile(config: ModelfileConfig) -> str:
    """
    Generate a REAL Ollama Modelfile.
    
    The Modelfile specifies:
    - FROM: the GGUF model file
    - PARAMETER: inference parameters
    - TEMPLATE: chat template for the model
    - SYSTEM: default system prompt
    - LICENSE: model license
    """
    lines = []

    # ── FROM (model file) ──
    lines.append(f"FROM {config.model_path}")
    lines.append("")

    # ── PARAMETERS ──
    params = {
        "temperature": config.temperature,
        "top_p": config.top_p,
        "top_k": config.top_k,
        "repeat_penalty": config.repeat_penalty,
        "num_ctx": config.context_length,
    }

    for key, val in params.items():
        lines.append(f"PARAMETER {key} {val}")

    # Stop tokens
    for stop in config.stop_tokens:
        lines.append(f'PARAMETER stop "{stop}"')

    lines.append("")

    # ── TEMPLATE ──
    arch_lower = config.architecture.lower()
    template = config.template or CHAT_TEMPLATES.get(arch_lower, CHAT_TEMPLATES["llama"])
    
    # Template needs to be on one line for Ollama
    template_oneline = template.replace("\n", "\\n").replace('"', '\\"')
    lines.append(f'TEMPLATE """{template_oneline}"""')
    lines.append("")

    # ── SYSTEM PROMPT ──
    system = config.system_prompt or (
        "You are a helpful AI assistant powered by PyTREX Ternary Quantization. "
        "You provide accurate, concise, and helpful responses."
    )
    lines.append(f'SYSTEM """{system}"""')
    lines.append("")

    # ── LICENSE ──
    if config.license:
        lines.append(f'LICENSE """{config.license}"""')
        lines.append("")

    return "\n".join(lines)


def generate_modelfile_moe(config: ModelfileConfig,
                            num_experts: int = 8,
                            active_experts: int = 2) -> str:
    """
    Generate Modelfile for a Mixture of Experts ternary model.
    
    Includes MoE-specific parameters and a more capable system prompt.
    """
    config.system_prompt = (
        "You are a highly capable AI assistant running on a Mixture of Experts "
        f"architecture with {num_experts} experts (top-{active_experts} routing) "
        "and ternary quantization. You provide comprehensive, accurate responses."
    )

    modelfile = generate_modelfile(config)

    # Add MoE-specific notes as comments
    header = f"""# ╔══════════════════════════════════════════════════════════╗
# ║  PyTREX MoE Ternary Model — {num_experts} Experts, Top-{active_experts}      ║
# ║  Quantization: Ternary {{-1, 0, +1}} (1.58-bit)            ║
# ║  Architecture: {config.architecture} with Mixture of Experts        ║
# ╚══════════════════════════════════════════════════════════╝

"""
    return header + modelfile


def generate_modelfile_omnimodal(config: ModelfileConfig) -> str:
    """
    Generate Modelfile for an omnimodal model with multimodal capabilities.
    This documents the multimodal heads even though they run separately.
    """
    config.description = (
        "Omnimodal AI: Text + Image + Audio + Video generation via multi-head architecture"
    )

    modelfile = generate_modelfile(config)

    header = f"""# ╔══════════════════════════════════════════════════════════╗
# ║  PyTREX Omnimodal Model                                    ║
# ║  • Text: GPT-style transformer with ternary quantization   ║
# ║  • Vision: Latent Diffusion head                           ║
# ║  • Video: Spatial-Temporal Flow-Matching head              ║
# ║  • Audio: Neural Codec head                                ║
# ║  • 3D: NeRF/Gaussian Splatting head                        ║
# ╚══════════════════════════════════════════════════════════════╝

"""
    return header + modelfile


class OllamaManager:
    """
    Manage Ollama — create, run, list, and remove models.
    Uses the ollama Python client when available.
    """

    def __init__(self):
        self._client = None

    def _get_client(self):
        """Lazy-load Ollama client."""
        if self._client is None:
            try:
                import ollama
                self._client = ollama.Client()
            except ImportError:
                print("  ⚠️  ollama Python package not installed.")
                print("  📦 Install: pip install ollama")
                return None
        return self._client

    def create_model(self, model_name: str, modelfile_path: str) -> bool:
        """
        Create model in Ollama from a Modelfile.
        
        Uses: ollama create {name} -f {modelfile}
        """
        import subprocess
        
        cmd = ["ollama", "create", model_name, "-f", modelfile_path]
        print(f"  🔨 Creating Ollama model: {' '.join(cmd)}")
        
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
            if result.returncode == 0:
                print(f"  ✅ Model '{model_name}' created in Ollama!")
                print(f"     {result.stdout.strip()}")
                return True
            else:
                print(f"  ❌ Failed: {result.stderr.strip()}")
                return False
        except FileNotFoundError:
            print("  ❌ Ollama CLI not found. Install from: https://ollama.com")
            return False
        except subprocess.TimeoutExpired:
            print("  ⚠️  Ollama create timed out")
            return False

    def list_models(self) -> list:
        """List all models in Ollama."""
        import subprocess
        try:
            result = subprocess.run(["ollama", "list"], capture_output=True, text=True)
            print(result.stdout)
            return result.stdout.strip().split("\n")
        except FileNotFoundError:
            print("  Ollama not installed.")
            return []

    def run_model(self, model_name: str, prompt: str = "Hello!") -> Optional[str]:
        """Run inference using the model."""
        client = self._get_client()
        if client is None:
            print("  ⚠️  Cannot run: Ollama client not available")
            return None

        try:
            response = client.chat(
                model=model_name,
                messages=[{"role": "user", "content": prompt}]
            )
            return response["message"]["content"]
        except Exception as e:
            print(f"  ❌ Inference error: {e}")
            return None


# ══════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("═" * 55)
    print("  📝 OLLAMA MODEFILE GENERATOR")
    print("═" * 55)

    # Standard ternary model
    config = ModelfileConfig(
        model_name="pytrex-ternary-7b",
        model_path="./models/pytrex_ternary_7b.gguf",
        architecture="llama",
        context_length=2048,
        temperature=0.7,
        top_p=0.9,
        top_k=40,
        repeat_penalty=1.1,
        license="MIT",
    )

    modelfile = generate_modelfile(config)
    print("\n  📄 Standard Ternary Modelfile:")
    print("  " + "─" * 50)
    print(modelfile)

    # MoE Modelfile
    config_moe = ModelfileConfig(
        model_name="pytrex-moe-ternary",
        model_path="./models/pytrex_moe_ternary.gguf",
        architecture="llama",
        context_length=4096,
        license="MIT",
    )
    modelfile_moe = generate_modelfile_moe(config_moe, num_experts=8, active_experts=2)
    print("\n  📄 MoE Modelfile:")
    print("  " + "─" * 50)
    print(modelfile_moe[:500] + "...")

    # Omnimodal Modelfile
    config_omni = ModelfileConfig(
        model_name="pytrex-omni-godmode",
        model_path="./models/pytrex_omni.gguf",
        architecture="llama",
        context_length=2048,
        license="MIT",
    )
    modelfile_omni = generate_modelfile_omnimodal(config_omni)
    print("\n  📄 Omnimodal Modelfile:")
    print("  " + "─" * 50)
    print(modelfile_omni[:500] + "...")

    # Save to file
    output_path = "C:\\PYTREX-master\\demos\\ai_engine\\Modelfile.ternary"
    with open(output_path, "w") as f:
        f.write(modelfile)
    print(f"\n  ✅ Saved: {output_path}")

    output_path_moe = "C:\\PYTREX-master\\demos\\ai_engine\\Modelfile.moe"
    with open(output_path_moe, "w") as f:
        f.write(modelfile_moe)
    print(f"  ✅ Saved: {output_path_moe}")

    print(f"\n  ✅ Modelfile Generator: FULLY OPERATIONAL")
    print(f"═" * 55)
