"""
╔══════════════════════════════════════════════════════════════════╗
║  MIXTURE OF EXPERTS (MoE) — Sparse Gated Routing               ║
║                                                                  ║
║  Real algorithm simulating how models like Mixtral 8x7B,       ║
║  DeepSeek-V2/V3, and GPT-4 route tokens to specialized         ║
║  expert sub-networks.                                           ║
║                                                                  ║
║  Key concepts implemented:                                      ║
║  • Top-K gating (k=2 default)                                   ║
║  • Noisy top-k gating with Gaussian noise                       ║
║  • Load balancing loss (auxiliary loss)                         ║
║  • Expert capacity and overflow (token dropping)                ║
║  • Sparse activation — only ~2/N experts fire per token        ║
║  • Expert specialization tracking                              ║
╚══════════════════════════════════════════════════════════════════╝
"""
import numpy as np
from typing import Tuple, List, Dict, Optional
from dataclasses import dataclass, field
import time


@dataclass
class MoEConfig:
    """Configuration for Mixture of Experts layer."""
    num_experts: int = 8
    top_k: int = 2
    hidden_dim: int = 512
    expert_dim: int = 2048  # FFN size inside each expert
    capacity_factor: float = 1.25
    noise_epsilon: float = 0.01  # For noisy gating (adds exploration)
    load_balance_coef: float = 0.01  # Weight of auxiliary load-balancing loss
    use_noisy_gating: bool = True
    router_z_loss_coef: float = 0.001  # Z-loss from DeepSeek-V2 paper


@dataclass
class RouterOutput:
    """Output from the MoE router/gate."""
    dispatch_mask: np.ndarray     # [num_tokens, num_experts] — which expert gets token
    combine_weights: np.ndarray   # [num_tokens, num_experts] — softmax weights
    aux_loss: float               # Load balancing auxiliary loss
    z_loss: float                 # Router Z-loss
    expert_load: np.ndarray       # How many tokens each expert got


class ExpertFFN:
    """
    A single expert — a 2-layer Feed-Forward Network.
    In real MoE, each expert is a full FFN with its own weights.
    For realistic simulation we use actual weight matrices.
    """

    def __init__(self, input_dim: int, hidden_dim: int, seed: int = 42):
        rng = np.random.RandomState(seed)
        scale = np.sqrt(2.0 / input_dim)  # He initialization
        self.W1 = rng.randn(input_dim, hidden_dim).astype(np.float32) * scale
        self.b1 = np.zeros(hidden_dim, dtype=np.float32)
        self.W2 = rng.randn(hidden_dim, input_dim).astype(np.float32) * np.sqrt(2.0 / hidden_dim)
        self.b2 = np.zeros(input_dim, dtype=np.float32)
        self.activation_count = 0

    def forward(self, x: np.ndarray) -> np.ndarray:
        """GELU activation → linear projection back to input_dim."""
        # GELU approximation: x * sigmoid(1.702 * x)
        hidden = x @ self.W1 + self.b1
        hidden_gelu = hidden * (1.0 / (1.0 + np.exp(-1.702 * hidden)))
        output = hidden_gelu @ self.W2 + self.b2
        self.activation_count += x.shape[0]
        return output


class MoELayer:
    """
    Mixture of Experts Layer.
    
    Mathematics:
      For each token x_i:
        1. Compute gate logits: g_i = W_gate @ x_i + noise * N(0,1)
        2. Select top-K: indices = topk(g_i, k)  
        3. Softmax (only over selected): p_i = softmax(g_i[topk])
        4. Dispatch: y_i = sum(p_ij * Expert_j(x_i)) for j in topk
    
    Load balancing loss (Switch Transformer):
      L_balance = num_experts * sum(f_i * P_i)
      where f_i = fraction of tokens routed to expert i
            P_i = mean gate probability for expert i
    """

    def __init__(self, config: MoEConfig):
        self.config = config
        rng = np.random.RandomState(42)
        # Router/gate weight matrix
        self.W_gate = rng.randn(config.hidden_dim, config.num_experts).astype(np.float32) * 0.02
        # One expert FFN per expert
        self.experts = [
            ExpertFFN(config.hidden_dim, config.expert_dim, seed=100 + i)
            for i in range(config.num_experts)
        ]

    def _compute_gate_logits(self, x: np.ndarray) -> np.ndarray:
        """Compute raw gate logits for each token-expert pair."""
        logits = x @ self.W_gate  # [num_tokens, num_experts]
        if self.config.use_noisy_gating:
            noise = np.random.randn(*logits.shape).astype(np.float32) * self.config.noise_epsilon
            # Softplus-based noise injection (from Noisy Top-K Gating paper)
            noise_std = np.log(1.0 + np.exp(noise))
            logits = logits + noise_std
        return logits

    def _top_k_gating(
        self, logits: np.ndarray
    ) -> Tuple[np.ndarray, np.ndarray, float, float]:
        """
        Top-K sparse gating.
        
        Returns:
            dispatch_mask: [tokens, experts] binary mask
            combine_weights: [tokens, experts] normalized weights
            aux_loss: Load balancing auxiliary loss
            z_loss: Router Z-loss (stabilizes training)
        """
        num_tokens, num_experts = logits.shape
        k = min(self.config.top_k, num_experts)

        # Find top-k expert indices per token
        topk_logits, topk_indices = self._topk_per_row(logits, k)

        # Softmax over top-k logits only (rest are -inf before softmax)
        topk_exp = np.exp(topk_logits - topk_logits.max(axis=1, keepdims=True))
        topk_weights = topk_exp / topk_exp.sum(axis=1, keepdims=True)

        # Build dispatch mask
        dispatch_mask = np.zeros((num_tokens, num_experts), dtype=np.float32)
        combine_weights = np.zeros((num_tokens, num_experts), dtype=np.float32)
        for i in range(num_tokens):
            for j in range(k):
                expert_idx = topk_indices[i, j]
                dispatch_mask[i, expert_idx] = 1.0
                combine_weights[i, expert_idx] = topk_weights[i, j]

        # ─── Expert Capacity (optional token dropping) ───
        capacity = int(np.ceil(num_tokens / num_experts * self.config.capacity_factor * k))
        expert_counts = dispatch_mask.sum(axis=0)  # tokens per expert
        overflow_mask = expert_counts > capacity
        if overflow_mask.any():
            # Drop excess tokens from overloaded experts
            for expert_idx in np.where(overflow_mask)[0]:
                token_indices = np.where(dispatch_mask[:, expert_idx] > 0)[0]
                if len(token_indices) > capacity:
                    # Keep only first 'capacity' tokens by largest gate weight
                    gate_weights = combine_weights[token_indices, expert_idx]
                    keep = np.argsort(gate_weights)[-capacity:]
                    to_drop = np.setdiff1d(np.arange(len(token_indices)), keep)
                    for idx in token_indices[to_drop]:
                        dispatch_mask[idx, expert_idx] = 0
                        combine_weights[idx, expert_idx] = 0

        # ─── Load Balancing Loss (Switch Transformer) ───
        # f_i = fraction dispatched to expert i
        f = dispatch_mask.sum(axis=0) / (num_tokens + 1e-8)
        # P_i = mean gate probability for expert i
        P = combine_weights.sum(axis=0) / (num_tokens + 1e-8)
        aux_loss = num_experts * np.sum(f * P)

        # ─── Router Z-Loss (DeepSeek-V2) ───
        # Encourages logits to stay small → more stable routing
        z_loss = np.mean(np.square(np.max(logits, axis=1)))

        return dispatch_mask, combine_weights, float(aux_loss), float(z_loss)

    @staticmethod
    def _topk_per_row(matrix: np.ndarray, k: int) -> Tuple[np.ndarray, np.ndarray]:
        """Get top-k values and indices for each row."""
        # Efficient: argsort and take last k
        indices = np.argsort(matrix, axis=1)[:, -k:]
        values = np.take_along_axis(matrix, indices, axis=1)
        # Sort descending within each row
        sort_idx = np.argsort(-values, axis=1)
        indices = np.take_along_axis(indices, sort_idx, axis=1)
        values = np.take_along_axis(values, sort_idx, axis=1)
        return values, indices

    def forward(self, x: np.ndarray) -> Tuple[np.ndarray, RouterOutput]:
        """
        Forward pass through MoE layer.
        
        Args:
            x: [num_tokens, hidden_dim] input embeddings
            
        Returns:
            output: [num_tokens, hidden_dim] processed embeddings
            router_out: routing metadata
        """
        num_tokens = x.shape[0]

        # 1. Gate computation
        logits = self._compute_gate_logits(x)

        # 2. Top-K routing
        dispatch_mask, combine_weights, aux_loss, z_loss = self._top_k_gating(logits)

        # 3. Expert computation (parallel in real systems)
        expert_outputs = np.zeros_like(x)
        for expert_idx, expert in enumerate(self.experts):
            mask = dispatch_mask[:, expert_idx] > 0
            if mask.any():
                expert_input = x[mask]
                expert_out = expert.forward(expert_input)
                weights = combine_weights[mask, expert_idx].reshape(-1, 1)
                expert_outputs[mask] += expert_out * weights

        # 4. Residual-like: input + expert output (simplified; real adds router output to residual)
        output = x + expert_outputs

        expert_load = dispatch_mask.sum(axis=0)

        router_out = RouterOutput(
            dispatch_mask=dispatch_mask,
            combine_weights=combine_weights,
            aux_loss=aux_loss,
            z_loss=z_loss,
            expert_load=expert_load,
        )

        return output, router_out

    def get_expert_specialization(self, test_batch: np.ndarray) -> np.ndarray:
        """
        Measure how many tokens each expert specializes in processing.
        Returns load distribution across experts.
        """
        _, router_out = self.forward(test_batch)
        return router_out.expert_load


class MoETransformerBlock:
    """
    A simplified Transformer block with MoE replacing the FFN.
    
    Real architecture: Self-Attention → Add&Norm → MoE → Add&Norm
    This is a realistic simulation with actual matrix operations.
    """

    def __init__(self, config: MoEConfig):
        self.config = config
        self.moe = MoELayer(config)
        rng = np.random.RandomState(999)
        self.W_q = rng.randn(config.hidden_dim, config.hidden_dim).astype(np.float32) * 0.02
        self.W_k = rng.randn(config.hidden_dim, config.hidden_dim).astype(np.float32) * 0.02
        self.W_v = rng.randn(config.hidden_dim, config.hidden_dim).astype(np.float32) * 0.02
        self.W_o = rng.randn(config.hidden_dim, config.hidden_dim).astype(np.float32) * 0.02
        self.layer_norm1 = LayerNorm(config.hidden_dim)
        self.layer_norm2 = LayerNorm(config.hidden_dim)
        self.total_flops = 0

    def _self_attention(self, x: np.ndarray) -> Tuple[np.ndarray, np.ndarray]:
        """Multi-head self-attention (simplified to single-head for clarity)."""
        num_tokens, d = x.shape
        scale = np.sqrt(d)
        
        Q = x @ self.W_q
        K = x @ self.W_k
        V = x @ self.W_v
        
        # Attention scores
        scores = Q @ K.T / scale  # [tokens, tokens]
        
        # Causal mask (upper triangular is -inf)
        mask = np.triu(np.ones_like(scores) * -1e9, k=1)
        scores = scores + mask
        
        # Softmax
        scores_max = scores.max(axis=1, keepdims=True)
        scores_exp = np.exp(scores - scores_max)
        attn_weights = scores_exp / scores_exp.sum(axis=1, keepdims=True)
        
        output = attn_weights @ V
        output = output @ self.W_o
        
        # Track approximate FLOPs
        self.total_flops += num_tokens * num_tokens * d * 4  # QK^T + softmax + AV + WO
        
        return output, attn_weights

    def forward(self, x: np.ndarray) -> Tuple[np.ndarray, RouterOutput, np.ndarray]:
        # Self-attention with residual + layer norm
        attn_out, attn_weights = self._self_attention(self.layer_norm1(x))
        x = x + attn_out
        
        # MoE FFN with residual + layer norm
        moe_out, router_out = self.moe.forward(self.layer_norm2(x))
        
        # Track MoE FLOPs
        num_active_experts = (router_out.dispatch_mask.sum(axis=0) > 0).sum()
        tokens_per_expert = router_out.expert_load
        d_ff = self.config.expert_dim
        flops_moe = num_active_experts * (2 * d_ff * self.config.hidden_dim * tokens_per_expert.mean())
        self.total_flops += flops_moe
        
        return moe_out, router_out, attn_weights


class LayerNorm:
    """Simple Layer Normalization."""
    def __init__(self, dim: int, eps: float = 1e-5):
        self.gamma = np.ones(dim, dtype=np.float32)
        self.beta = np.zeros(dim, dtype=np.float32)
        self.eps = eps
    
    def forward(self, x: np.ndarray) -> np.ndarray:
        mean = x.mean(axis=-1, keepdims=True)
        var = x.var(axis=-1, keepdims=True)
        return self.gamma * (x - mean) / np.sqrt(var + self.eps) + self.beta


# ══════════════════════════════════════════════════════════════════
# DEMO & ANALYSIS
# ══════════════════════════════════════════════════════════════════

def demo_moe_routing():
    """Demonstrate Mixture of Experts routing with analysis."""
    print("═" * 60)
    print("  🧠 MIXTURE OF EXPERTS — Sparse Gated Routing Demo")
    print("═" * 60)

    config = MoEConfig(
        num_experts=16,
        top_k=2,
        hidden_dim=512,
        expert_dim=2048,
        capacity_factor=1.25,
        use_noisy_gating=True,
    )

    block = MoETransformerBlock(config)

    # Simulate a batch of 64 tokens (like a short sentence)
    num_tokens = 64
    rng = np.random.RandomState(42)
    x = rng.randn(num_tokens, config.hidden_dim).astype(np.float32)

    t0 = time.perf_counter()
    output, router_out, attn_weights = block.forward(x)
    elapsed = time.perf_counter() - t0

    # ─── Analysis ───
    print(f"\n  📊 Configuration:")
    print(f"     Experts: {config.num_experts} | Top-K: {config.top_k}")
    print(f"     Hidden Dim: {config.hidden_dim} | Expert FFN Dim: {config.expert_dim}")
    print(f"     Tokens per forward pass: {num_tokens}")

    print(f"\n  ⚡ Routing Statistics:")
    print(f"     Active experts: {(router_out.expert_load > 0).sum()}/{config.num_experts}")
    print(f"     Avg tokens/expert: {router_out.expert_load[router_out.expert_load > 0].mean():.1f}")
    print(f"     Max tokens/expert: {router_out.expert_load.max():.0f}")
    print(f"     Min tokens/expert (active): {router_out.expert_load[router_out.expert_load > 0].min():.0f}")
    print(f"     Imbalance ratio: {router_out.expert_load.max() / (router_out.expert_load[router_out.expert_load > 0].mean() + 1e-8):.2f}x")

    print(f"\n  📉 Losses:")
    print(f"     Load Balancing Loss: {router_out.aux_loss:.6f}")
    print(f"     Router Z-Loss:       {router_out.z_loss:.6f}")

    print(f"\n  ⏱️  Forward pass: {elapsed*1000:.2f} ms")
    print(f"     ~FLOPs: {block.total_flops / 1e6:.1f}M")

    # ─── Sparsity advantage ───
    dense_flops = num_tokens * 2 * config.hidden_dim * config.expert_dim * config.num_experts
    sparse_flops_approx = num_tokens * 2 * config.hidden_dim * config.expert_dim * config.top_k
    savings = (1 - sparse_flops_approx / dense_flops) * 100
    print(f"\n  💡 Compute Savings (vs Dense FFN of same total params):")
    print(f"     Dense FLOPs:   ~{dense_flops/1e9:.2f}B")
    print(f"     Sparse FLOPs:  ~{sparse_flops_approx/1e9:.2f}B")
    print(f"     Savings:       {savings:.1f}%")

    # ─── Expert load distribution ───
    print(f"\n  📊 Expert Load Distribution:")
    bar_max = max(int(router_out.expert_load.max()), 1)
    for i, load in enumerate(router_out.expert_load):
        bar = "█" * int(load / bar_max * 30)
        status = "🟢" if load > 0 else "⚫"
        print(f"     Expert {i:3d}: {status} {bar} ({int(load)})")

    print(f"\n  ✅ MoE Routing Demo: SUCCESS")
    print(f"═" * 60)

    return {
        "config": config,
        "router_out": router_out,
        "elapsed_ms": elapsed * 1000,
        "flops_m": block.total_flops / 1e6,
        "savings_pct": savings,
    }


def demo_expert_specialization():
    """Show how experts develop specialization over multiple forward passes."""
    print("\n" + "═" * 60)
    print("  🎯 EXPERT SPECIALIZATION TRACKING")
    print("═" * 60)

    config = MoEConfig(num_experts=8, top_k=2, hidden_dim=256, expert_dim=1024)
    moe = MoELayer(config)

    # Three different "domains" of input
    domains = {
        "Code": np.random.RandomState(10).randn(32, 256).astype(np.float32) * 1.5 + 0.5,
        "Math": np.random.RandomState(20).randn(32, 256).astype(np.float32) * 0.8 - 0.3,
        "Language": np.random.RandomState(30).randn(32, 256).astype(np.float32) * 1.2,
    }

    for domain_name, domain_data in domains.items():
        _, router_out = moe.forward(domain_data)
        top_experts = np.argsort(-router_out.expert_load)[:3]
        print(f"\n  📂 Domain '{domain_name}':")
        print(f"     Top experts: {top_experts.tolist()}")
        for rank, expert_idx in enumerate(top_experts):
            load = router_out.expert_load[expert_idx]
            print(f"       #{rank+1}: Expert {expert_idx} — {load:.0f} tokens ({load/32*100:.0f}%)")

    print(f"\n  ✅ Expert Specialization: TRACKED")
    print(f"═" * 60)


if __name__ == "__main__":
    demo_moe_routing()
    demo_expert_specialization()
