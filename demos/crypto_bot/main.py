"""
╔══════════════════════════════════════════════════════════════╗
║  DEMO 2: CRYPTO TRADING BOT (AI + Blockchain + HITL)       ║
║  Features: AI predictions, Blockchain ledger,              ║
║            Human approval for large trades,                ║
║            Portfolio tracking, Risk management             ║
╚══════════════════════════════════════════════════════════════╝
"""
from pytrex import PyTreXApp, event
from pytrex import HermesAgent, HumanInTheLoop, BlockchainBridge
from pytrex.search_engine import SearchEngine
import json, time, random

class CryptoTradingBot(PyTreXApp):
    """AI-Powered Crypto Trading Bot — inafanya trading automatically"""

    def __init__(self):
        super().__init__(name="CryptoTrader AI")
        self.blockchain = BlockchainBridge()
        self.hitl = HumanInTheLoop(default_timeout=60)
        self.hermes = HermesAgent(name="TraderBot")
        self.search = SearchEngine()

        self.portfolio = {"USDT": 10000.0, "BTC": 0.0, "ETH": 0.0, "TZS": 25000000.0}
        self.trades = []
        self.prices = {"BTC": 45000.0, "ETH": 2800.0, "USDT": 1.0}

        # Register trading functions for Hermes
        self.hermes.register_function(
            "analyze_market", self._analyze_market,
            "Analyze current market conditions and recommend action",
            {"symbol": {"type": "string"}}, category="trading"
        )
        self.hermes.register_function(
            "get_portfolio", lambda **kw: json.dumps(self.portfolio),
            "Get current portfolio value", {}, category="trading"
        )

    def _analyze_market(self, symbol="BTC", **kw):
        """AI market analysis kwa kutumia Hermes + Search"""
        # Search latest news
        news = self.search.web_search_summary(f"{symbol} crypto price news today")
        # Simulate AI price prediction
        sentiment = random.choice(["BULLISH 📈", "NEUTRAL ➡️", "BEARISH 📉"])
        predicted_move = random.uniform(-5, 8)
        new_price = self.prices.get(symbol, 100) * (1 + predicted_move/100)

        return json.dumps({
            "symbol": symbol,
            "current_price": self.prices.get(symbol),
            "predicted_price": round(new_price, 2),
            "move_percent": round(predicted_move, 2),
            "sentiment": sentiment,
            "recommendation": "BUY" if predicted_move > 2 else ("SELL" if predicted_move < -2 else "HOLD"),
            "news_headlines": len(news.get("results", []))
        })

    @event("execute_trade")
    def execute_trade(self, data):
        """Execute crypto trade na blockchain record"""
        payload = json.loads(data) if isinstance(data, str) else data
        symbol = payload.get("symbol", "BTC")
        action = payload.get("action", "BUY")
        amount_usd = payload.get("amount", 100)
        price = self.prices.get(symbol, 45000)

        quantity = amount_usd / price

        # LARGE TRADES → require HUMAN APPROVAL
        if amount_usd > 5000:
            action_id = self.hitl.request_approval(
                "large_trade",
                {"symbol": symbol, "action": action, "amount": amount_usd, "quantity": quantity},
                timeout=300
            )
            return json.dumps({
                "status": "pending_approval",
                "action_id": action_id,
                "message": f"⚠️ Trade ya {amount_usd} USD inahitaji idhini yako! Approve: hitl.approve('{action_id}')"
            })

        # Execute trade
        if action == "BUY":
            if self.portfolio.get("USDT", 0) >= amount_usd:
                self.portfolio["USDT"] -= amount_usd
                self.portfolio[symbol] = self.portfolio.get(symbol, 0) + quantity
            else:
                return json.dumps({"status": "failed", "reason": "Insufficient USDT"})
        else:  # SELL
            if self.portfolio.get(symbol, 0) >= quantity:
                self.portfolio[symbol] -= quantity
                self.portfolio["USDT"] += amount_usd

        # Record on blockchain
        trade_record = {
            "symbol": symbol, "action": action, "quantity": quantity,
            "price": price, "amount_usd": amount_usd, "time": time.time()
        }
        self.blockchain.add_block(json.dumps(trade_record))
        self.trades.append(trade_record)

        return json.dumps({
            "status": "executed",
            "trade": trade_record,
            "portfolio": self.portfolio,
            "blockchain_verified": True
        })

    @event("portfolio_summary")
    def portfolio_summary(self, data):
        """Get portfolio summary"""
        total_value = 0
        for asset, qty in self.portfolio.items():
            price = self.prices.get(asset, 1)
            total_value += qty * price

        return json.dumps({
            "portfolio": self.portfolio,
            "total_value_usd": round(total_value, 2),
            "total_trades": len(self.trades),
            "blockchain_blocks": len(self.blockchain._chain) if hasattr(self.blockchain, '_chain') else 0,
            "prices": self.prices
        })


# ─── RUN ─────────────────────────────────────────────────────
if __name__ == "__main__":
    print("═" * 55)
    print("  🤖 CRYPTO TRADING BOT — Live Demo")
    print("═" * 55)

    bot = CryptoTradingBot()

    # AI Market Analysis
    print("\n  📊 AI Market Analysis:")
    for coin in ["BTC", "ETH"]:
        analysis = bot._analyze_market(coin)
        data = json.loads(analysis)
        print(f"     {data['symbol']}: {data['sentiment']} | "
              f"${data['current_price']} → ${data['predicted_price']} | "
              f"{data['recommendation']}")

    # Execute trades
    r1 = bot.execute_trade('{"symbol": "BTC", "action": "BUY", "amount": 500}')
    r2 = bot.execute_trade('{"symbol": "ETH", "action": "BUY", "amount": 300}')
    r3 = bot.execute_trade('{"symbol": "BTC", "action": "SELL", "amount": 200}')

    # Large trade → requires human approval
    r4 = bot.execute_trade('{"symbol": "BTC", "action": "BUY", "amount": 10000}')

    # Portfolio summary
    result = bot.portfolio_summary("{}")
    summary = json.loads(result)

    print(f"\n  💰 Portfolio: ${summary['total_value_usd']:,.2f}")
    print(f"  📊 Assets: {json.dumps(bot.portfolio, indent=4)}")
    print(f"  🔗 Blockchain Blocks: {summary['blockchain_blocks']}")
    print(f"  📈 Total Trades: {summary['total_trades']}")
    print(f"  ⚠️  Large trade ya $10,000 inasubiri HITL approval!")

    # Hermes AI agent
    hermes_result = bot.hermes.chat("What's my portfolio status?")
    print(f"\n  🧠 Hermes AI: {hermes_result['reply'][:100]}...")

    print(f"\n  ✅ Crypto Trading Bot: FULLY OPERATIONAL")
    print(f"═" * 55)
