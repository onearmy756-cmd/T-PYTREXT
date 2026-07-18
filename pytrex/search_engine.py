"""
PyTreXT Search Engine — SearXNG + DuckDuckGo Integration
==========================================================
Inawezesha utafutaji wa wavuti ndani ya PyTreX kwa kutumia:
- SearXNG (privacy-first metasearch engine)
- DuckDuckGo (privacy-focused instant answers)
- Multi-engine fallback strategy

Usage:
    from pytrex.search_engine import SearchEngine
    engine = SearchEngine()
    results = engine.search("PyTreX framework AI blockchain")
    results = engine.duckduckgo_search("latest AI news")
    results = engine.searxng_search("python async", instance="https://searx.be")
"""

import json
import logging
from typing import Any, Dict, List, Optional
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor, as_completed

logger = logging.getLogger("pytrex.search")


@dataclass
class SearchResult:
    """Matokeo ya utafutaji mmoja"""
    title: str
    url: str
    snippet: str
    engine: str = "unknown"
    score: float = 1.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet,
            "engine": self.engine,
            "score": self.score,
        }


class SearchEngine:
    """
    PyTreX Search Engine — inasaidia SearXNG na DuckDuckGo.
    Multi-engine search with automatic fallback.
    """

    def __init__(
        self,
        default_engine: str = "duckduckgo",
        searxng_instance: str = "https://searx.be",
        max_results: int = 10,
        timeout: float = 10.0,
    ):
        self.default_engine = default_engine
        self.searxng_instance = searxng_instance.rstrip("/")
        self.max_results = max_results
        self.timeout = timeout

        # Try importing optional dependencies
        self._has_duckduckgo = False
        self._has_aiohttp = False
        self._has_httpx = False

        try:
            from duckduckgo_search import DDGS
            self._ddgs_class = DDGS
            self._has_duckduckgo = True
        except ImportError:
            logger.warning("duckduckgo-search not installed — DuckDuckGo search unavailable")

        try:
            import aiohttp
            self._has_aiohttp = True
        except ImportError:
            pass

        try:
            import httpx
            self._has_httpx = True
        except ImportError:
            pass

        logger.info(
            f"SearchEngine initialized: default={default_engine}, "
            f"searxng={searxng_instance}, ddg={'✓' if self._has_duckduckgo else '✗'}"
        )

    # ─── Main Search Interface ────────────────────────────────

    def search(
        self,
        query: str,
        engine: Optional[str] = None,
        max_results: Optional[int] = None,
        **kwargs,
    ) -> List[SearchResult]:
        """
        Tafuta kwa kutumia engine maalum.

        Args:
            query: Maneno ya kutafuta
            engine: "duckduckgo", "searxng", au "all"
            max_results: Idadi ya juu ya matokeo
        """
        engine = engine or self.default_engine
        max_results = max_results or self.max_results

        if engine == "all":
            return self._multi_engine_search(query, max_results, **kwargs)
        elif engine == "searxng":
            return self.searxng_search(query, max_results, self.searxng_instance, **kwargs)
        else:
            return self.duckduckgo_search(query, max_results, **kwargs)

    def _multi_engine_search(
        self, query: str, max_results: int, **kwargs
    ) -> List[SearchResult]:
        """Tafuta kwenye engines zote kwa sambamba"""
        all_results: List[SearchResult] = []

        with ThreadPoolExecutor(max_workers=2) as executor:
            futures = {}

            if self._has_duckduckgo:
                futures[executor.submit(self.duckduckgo_search, query, max_results // 2)] = "ddg"

            futures[executor.submit(
                self.searxng_search, query, max_results // 2, self.searxng_instance
            )] = "searxng"

            for future in as_completed(futures):
                engine_name = futures[future]
                try:
                    results = future.result(timeout=self.timeout)
                    all_results.extend(results)
                    logger.debug(f"{engine_name}: {len(results)} results")
                except Exception as e:
                    logger.warning(f"Engine '{engine_name}' failed: {e}")

        # Remove duplicates by URL
        seen_urls = set()
        unique_results = []
        for r in sorted(all_results, key=lambda x: x.score, reverse=True):
            if r.url not in seen_urls:
                seen_urls.add(r.url)
                unique_results.append(r)

        return unique_results[:max_results]

    # ─── DuckDuckGo Search ────────────────────────────────────

    def duckduckgo_search(
        self,
        query: str,
        max_results: Optional[int] = None,
        region: str = "wt-wt",
        safesearch: str = "moderate",
        **kwargs,
    ) -> List[SearchResult]:
        """
        Tafuta kwa kutumia DuckDuckGo Instant Answers API.

        Args:
            query: Maneno ya kutafuta
            max_results: Idadi ya juu ya matokeo
            region: Mkoa wa utafutaji (wt-wt = worldwide)
            safesearch: "strict", "moderate", au "off"
        """
        max_results = max_results or self.max_results

        if not self._has_duckduckgo:
            return self._fallback_search(query, "duckduckgo")

        try:
            results = []
            with self._ddgs_class() as ddgs:
                for i, r in enumerate(ddgs.text(
                    query,
                    region=region,
                    safesearch=safesearch,
                    max_results=max_results,
                )):
                    if i >= max_results:
                        break
                    results.append(SearchResult(
                        title=r.get("title", ""),
                        url=r.get("href", r.get("url", "")),
                        snippet=r.get("body", r.get("snippet", "")),
                        engine="duckduckgo",
                        score=1.0 - (i * 0.05),
                    ))

            logger.info(f"DuckDuckGo: '{query}' → {len(results)} results")
            return results

        except Exception as e:
            logger.error(f"DuckDuckGo search failed: {e}")
            return self._fallback_search(query, "duckduckgo")

    def duckduckgo_instant_answer(self, query: str) -> Optional[Dict[str, Any]]:
        """Pata jibu la papo kwa papo kutoka DuckDuckGo (instant answer)"""
        if not self._has_duckduckgo:
            return None

        try:
            with self._ddgs_class() as ddgs:
                answers = list(ddgs.answers(query))
                if answers:
                    answer = answers[0]
                    return {
                        "text": answer.get("text", ""),
                        "url": answer.get("url", ""),
                        "source": "DuckDuckGo Instant Answer",
                    }
        except Exception as e:
            logger.debug(f"DuckDuckGo instant answer failed: {e}")

        return None

    def duckduckgo_news(self, query: str, max_results: int = 5) -> List[SearchResult]:
        """Tafuta habari mpya kwa kutumia DuckDuckGo News"""
        if not self._has_duckduckgo:
            return []

        try:
            results = []
            with self._ddgs_class() as ddgs:
                for i, r in enumerate(ddgs.news(query, max_results=max_results)):
                    results.append(SearchResult(
                        title=r.get("title", ""),
                        url=r.get("url", ""),
                        snippet=r.get("body", r.get("snippet", "")),
                        engine="duckduckgo_news",
                        score=1.0 - (i * 0.05),
                    ))

            logger.info(f"DuckDuckGo News: '{query}' → {len(results)} results")
            return results

        except Exception as e:
            logger.error(f"DuckDuckGo news search failed: {e}")
            return []

    # ─── SearXNG Search ───────────────────────────────────────

    def searxng_search(
        self,
        query: str,
        max_results: Optional[int] = None,
        instance: Optional[str] = None,
        categories: str = "general",
        **kwargs,
    ) -> List[SearchResult]:
        """
        Tafuta kwa kutumia SearXNG metasearch engine.

        Args:
            query: Maneno ya kutafuta
            max_results: Idadi ya juu ya matokeo
            instance: URL ya SearXNG instance (default: https://searx.be)
            categories: "general", "news", "science", "files", n.k.
        """
        max_results = max_results or self.max_results
        instance = instance or self.searxng_instance

        try:
            import urllib.request
            import urllib.parse
            import urllib.error

            params = urllib.parse.urlencode({
                "q": query,
                "format": "json",
                "categories": categories,
                "pageno": 1,
            })

            url = f"{instance}/search?{params}"
            req = urllib.request.Request(url, headers={"User-Agent": "PyTreXT/1.0"})

            with urllib.request.urlopen(req, timeout=self.timeout) as response:
                data = json.loads(response.read().decode())

            results = []
            for i, r in enumerate(data.get("results", [])):
                if i >= max_results:
                    break
                results.append(SearchResult(
                    title=r.get("title", ""),
                    url=r.get("url", ""),
                    snippet=r.get("content", r.get("snippet", "")),
                    engine=f"searxng/{r.get('engine', 'unknown')}",
                    score=1.0 - (i * 0.05),
                ))

            logger.info(f"SearXNG ({instance}): '{query}' → {len(results)} results")
            return results

        except urllib.error.HTTPError as e:
            logger.error(f"SearXNG HTTP error {e.code}: {instance}")
            return self._fallback_search(query, f"searxng@{instance}")
        except urllib.error.URLError as e:
            logger.error(f"SearXNG connection failed to {instance}: {e.reason}")
            return self._fallback_search(query, f"searxng@{instance}")
        except Exception as e:
            logger.error(f"SearXNG search failed: {e}")
            return self._fallback_search(query, f"searxng@{instance}")

    # ─── Fallback & Utility ───────────────────────────────────

    def _fallback_search(self, query: str, engine: str) -> List[SearchResult]:
        """Fallback — arifu mtumiaji kuwa engine haipatikani"""
        return [SearchResult(
            title=f"Search for: {query}",
            url=f"https://www.google.com/search?q={query.replace(' ', '+')}",
            snippet=f"[PyTreXT] {engine} search is not available. Install required packages: duckduckgo-search for DDG, or configure a SearXNG instance.",
            engine=engine,
            score=0.0,
        )]

    def web_search_summary(self, query: str, max_results: int = 5) -> Dict[str, Any]:
        """
        Tafuta na rudisha muhtasari wa matokeo.
        Inafaa kwa AI agents na RAG pipelines.
        """
        results = self.search(query, engine="all", max_results=max_results)

        summary_parts = []
        for r in results:
            summary_parts.append(f"- [{r.title}]({r.url}): {r.snippet[:200]}")

        return {
            "query": query,
            "total_results": len(results),
            "results": [r.to_dict() for r in results],
            "summary": "\n".join(summary_parts),
            "engines_used": list(set(r.engine for r in results)),
        }

    def to_dict(self) -> Dict[str, Any]:
        """Export search engine state"""
        return {
            "default_engine": self.default_engine,
            "searxng_instance": self.searxng_instance,
            "max_results": self.max_results,
            "duckduckgo_available": self._has_duckduckgo,
            "engines": {
                "duckduckgo": self._has_duckduckgo,
                "searxng": True,
                "httpx": self._has_httpx,
                "aiohttp": self._has_aiohttp,
            },
        }

    def __repr__(self) -> str:
        return f"SearchEngine(default={self.default_engine}, ddg={'✓' if self._has_duckduckgo else '✗'})"


# ─── Convenience Functions ────────────────────────────────────

def quick_search(query: str, engine: str = "duckduckgo", max_results: int = 5) -> List[Dict[str, Any]]:
    """Tafuta haraka bila kuanzisha SearchEngine object"""
    engine_obj = SearchEngine(default_engine=engine, max_results=max_results)
    results = engine_obj.search(query)
    return [r.to_dict() for r in results]


def quick_web_summary(query: str) -> Dict[str, Any]:
    """Tafuta haraka na pata muhtasari"""
    engine = SearchEngine()
    return engine.web_search_summary(query)
