"""
PyTreXT Human-in-the-Loop — Interactive AI Workflows
======================================================
Inawezesha mwingiliano kati ya binadamu na AI agents ndani ya PyTreXT.
- Approval workflows (pause, approve, reject)
- Workflow state machine
- Real-time notifications via Elixir + Tauri
- Timeout management
- Audit trail integration

Usage:
    from pytrex.human_in_loop import HumanInTheLoop
    hitl = HumanInTheLoop()
    action_id = hitl.request_approval("transfer_funds", {"amount": 5000, "to": "ACC123"})
    hitl.approve(action_id)  # au hitl.reject(action_id, "Amount too high")
"""

import json
import logging
import threading
import time
import uuid
from typing import Any, Callable, Dict, List, Optional
from dataclasses import dataclass, field
from enum import Enum
from concurrent.futures import Future

logger = logging.getLogger("pytrex.hitl")


class ActionStatus(Enum):
    """Hali ya action inayosubiri idhini"""
    PENDING = "pending"
    APPROVED = "approved"
    REJECTED = "rejected"
    EXPIRED = "expired"
    EXECUTING = "executing"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class PendingAction:
    """Action inayosubiri idhini ya binadamu"""
    action_id: str
    action_type: str
    context: Dict[str, Any]
    status: ActionStatus = ActionStatus.PENDING
    created_at: float = field(default_factory=time.time)
    resolved_at: Optional[float] = None
    rejected_reason: Optional[str] = None
    result: Optional[Any] = None
    callback: Optional[Callable] = None
    future: Optional[Future] = None

    def age_seconds(self) -> float:
        return time.time() - self.created_at

    def to_dict(self) -> Dict[str, Any]:
        return {
            "action_id": self.action_id,
            "action_type": self.action_type,
            "context": self.context,
            "status": self.status.value,
            "created_at": self.created_at,
            "age_seconds": self.age_seconds(),
            "resolved_at": self.resolved_at,
            "rejected_reason": self.rejected_reason,
        }


class HumanInTheLoop:
    """
    Human-in-the-Loop manager — inasimamia workflows za AI zinazohitaji idhini ya binadamu.
    Supports Tauri desktop notifications and Elixir real-time updates.
    """

    def __init__(
        self,
        default_timeout: float = 300.0,  # dakika 5
        max_pending: int = 100,
        auto_expire: bool = True,
        notification_callback: Optional[Callable] = None,
    ):
        self.default_timeout = default_timeout
        self.max_pending = max_pending
        self.auto_expire = auto_expire
        self.notification_callback = notification_callback

        self._pending: Dict[str, PendingAction] = {}
        self._history: List[PendingAction] = []
        self._lock = threading.Lock()
        self._expiry_thread: Optional[threading.Thread] = None
        self._running = False

        self._approval_hooks: Dict[str, List[Callable]] = {}
        self._rejection_hooks: Dict[str, List[Callable]] = {}

        logger.info(f"HITL initialized: timeout={default_timeout}s, max_pending={max_pending}")

    # ─── Request Approval ─────────────────────────────────────

    def request_approval(
        self,
        action_type: str,
        context: Dict[str, Any],
        timeout: Optional[float] = None,
        callback: Optional[Callable] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> str:
        """
        Omba idhini ya binadamu kabla ya kutekeleza action.

        Args:
            action_type: Aina ya action (mf: "transfer_funds", "delete_record", "send_email")
            context: Maelezo ya action
            timeout: Muda wa kusubiri (sekunde) kabla ya action ku-expire
            callback: Function itakayoitwa baada ya ku-approve/reject
            metadata: Maelezo ya ziada

        Returns:
            action_id — tumia hii ku-approve/reject baadae
        """
        with self._lock:
            if len(self._pending) >= self.max_pending:
                oldest = min(self._pending.values(), key=lambda a: a.created_at)
                logger.warning(f"Max pending reached, expiring oldest: {oldest.action_id}")
                self._expire_action(oldest)

        action_id = str(uuid.uuid4())[:8]
        timeout = timeout or self.default_timeout

        action = PendingAction(
            action_id=action_id,
            action_type=action_type,
            context={**context, **(metadata or {})},
            callback=callback,
        )

        with self._lock:
            self._pending[action_id] = action

        logger.info(
            f"HITL Approval Requested: {action_id} type={action_type} "
            f"context={json.dumps(context, default=str)[:100]}"
        )

        # Send notification
        self._notify({
            "type": "approval_requested",
            "action_id": action_id,
            "action_type": action_type,
            "context": context,
            "timeout": timeout,
        })

        # Start expiry timer in background
        if timeout > 0:
            threading.Thread(
                target=self._expiry_timer,
                args=(action_id, timeout),
                daemon=True,
            ).start()

        return action_id

    # ─── Approve / Reject ─────────────────────────────────────

    def approve(self, action_id: str, notes: Optional[str] = None) -> bool:
        """
        Idhinisha action iliyosubiri.

        Args:
            action_id: Kitambulisho cha action
            notes: Maelezo ya ziada kuhusu idhini

        Returns:
            True kama action iliidhinishwa, False vinginevyo
        """
        with self._lock:
            action = self._pending.get(action_id)

        if action is None:
            logger.warning(f"Approve failed: action {action_id} not found")
            return False

        if action.status != ActionStatus.PENDING:
            logger.warning(f"Approve failed: action {action_id} is {action.status.value}")
            return False

        with self._lock:
            action.status = ActionStatus.APPROVED
            action.resolved_at = time.time()

        logger.info(f"HITL Approved: {action_id} type={action.action_type} notes={notes}")

        # Execute the action
        self._execute_action(action)

        # Run approval hooks
        self._run_hooks("approve", action)

        # Move to history
        self._archive_action(action_id)

        # Notify
        self._notify({
            "type": "action_approved",
            "action_id": action_id,
            "notes": notes,
        })

        return True

    def reject(self, action_id: str, reason: str = "") -> bool:
        """
        Kataa action iliyosubiri.

        Args:
            action_id: Kitambulisho cha action
            reason: Sababu ya kukataa

        Returns:
            True kama action ilikataliwa, False vinginevyo
        """
        with self._lock:
            action = self._pending.get(action_id)

        if action is None:
            logger.warning(f"Reject failed: action {action_id} not found")
            return False

        if action.status != ActionStatus.PENDING:
            logger.warning(f"Reject failed: action {action_id} is {action.status.value}")
            return False

        with self._lock:
            action.status = ActionStatus.REJECTED
            action.resolved_at = time.time()
            action.rejected_reason = reason

        logger.info(f"HITL Rejected: {action_id} reason='{reason}'")

        # Run rejection hooks
        self._run_hooks("reject", action)

        # Move to history
        self._archive_action(action_id)

        # Notify
        self._notify({
            "type": "action_rejected",
            "action_id": action_id,
            "reason": reason,
        })

        return True

    # ─── Action Execution ─────────────────────────────────────

    def _execute_action(self, action: PendingAction) -> None:
        """Execute the approved action"""
        with self._lock:
            action.status = ActionStatus.EXECUTING

        try:
            if action.callback:
                result = action.callback(action.context)
                with self._lock:
                    action.result = result
                    action.status = ActionStatus.COMPLETED
                logger.info(f"HITL Action completed: {action.action_id}")
            else:
                with self._lock:
                    action.status = ActionStatus.COMPLETED
                    action.result = {"status": "executed", "action_type": action.action_type}
        except Exception as e:
            with self._lock:
                action.status = ActionStatus.FAILED
                action.result = {"error": str(e)}
            logger.error(f"HITL Action failed: {action.action_id} error={e}")

    # ─── Query ────────────────────────────────────────────────

    def get_pending(self) -> List[Dict[str, Any]]:
        """Pata actions zote zinazosubiri idhini"""
        with self._lock:
            return [a.to_dict() for a in self._pending.values() if a.status == ActionStatus.PENDING]

    def get_action(self, action_id: str) -> Optional[Dict[str, Any]]:
        """Pata action kwa ID"""
        with self._lock:
            action = self._pending.get(action_id)
            if action:
                return action.to_dict()

            # Check history
            for a in self._history:
                if a.action_id == action_id:
                    return a.to_dict()

        return None

    def get_history(
        self,
        action_type: Optional[str] = None,
        limit: int = 50,
    ) -> List[Dict[str, Any]]:
        """Pata historia ya actions zilizokamilika"""
        with self._lock:
            history = self._history.copy()

        if action_type:
            history = [a for a in history if a.action_type == action_type]

        return [a.to_dict() for a in history[-limit:]]

    def pending_count(self) -> int:
        """Idadi ya actions zinazosubiri"""
        with self._lock:
            return sum(1 for a in self._pending.values() if a.status == ActionStatus.PENDING)

    # ─── Hooks ────────────────────────────────────────────────

    def on_approve(self, action_type: str, callback: Callable) -> None:
        """Sajili hook kwa ajili ya approval"""
        if action_type not in self._approval_hooks:
            self._approval_hooks[action_type] = []
        self._approval_hooks[action_type].append(callback)

    def on_reject(self, action_type: str, callback: Callable) -> None:
        """Sajili hook kwa ajili ya rejection"""
        if action_type not in self._rejection_hooks:
            self._rejection_hooks[action_type] = []
        self._rejection_hooks[action_type].append(callback)

    def _run_hooks(self, hook_type: str, action: PendingAction) -> None:
        """Execute registered hooks"""
        hooks_dict = self._approval_hooks if hook_type == "approve" else self._rejection_hooks

        # Run type-specific hooks
        for callback in hooks_dict.get(action.action_type, []):
            try:
                callback(action.to_dict())
            except Exception as e:
                logger.error(f"Hook failed ({hook_type}/{action.action_type}): {e}")

        # Run wildcard hooks
        for callback in hooks_dict.get("*", []):
            try:
                callback(action.to_dict())
            except Exception as e:
                logger.error(f"Wildcard hook failed ({hook_type}): {e}")

    # ─── Expiry Management ────────────────────────────────────

    def _expiry_timer(self, action_id: str, timeout: float) -> None:
        """Expire action baada ya timeout"""
        time.sleep(timeout)

        with self._lock:
            action = self._pending.get(action_id)

        if action and action.status == ActionStatus.PENDING:
            self._expire_action(action)
            self._archive_action(action_id)

            self._notify({
                "type": "action_expired",
                "action_id": action_id,
                "timeout": timeout,
            })

    def _expire_action(self, action: PendingAction) -> None:
        """Expire an action"""
        with self._lock:
            action.status = ActionStatus.EXPIRED
            action.resolved_at = time.time()
        logger.info(f"HITL Expired: {action.action_id}")

    def _archive_action(self, action_id: str) -> None:
        """Move action from pending to history"""
        with self._lock:
            if action_id in self._pending:
                action = self._pending.pop(action_id)
                self._history.append(action)
                # Trim history to last 1000
                if len(self._history) > 1000:
                    self._history = self._history[-1000:]

    # ─── Notifications ────────────────────────────────────────

    def _notify(self, event: Dict[str, Any]) -> None:
        """Send notification via callback (Tauri, Elixir, or custom)"""
        if self.notification_callback:
            try:
                self.notification_callback(event)
            except Exception as e:
                logger.error(f"Notification callback failed: {e}")

    def set_notification_callback(self, callback: Callable) -> None:
        """Set custom notification callback"""
        self.notification_callback = callback

    # ─── Cleanup ──────────────────────────────────────────────

    def cleanup(self) -> int:
        """Clean up expired/failed actions from history"""
        with self._lock:
            before = len(self._history)
            self._history = [
                a for a in self._history
                if a.status not in (ActionStatus.EXPIRED, ActionStatus.FAILED)
            ]
            removed = before - len(self._history)
            if removed:
                logger.info(f"HITL cleanup: removed {removed} stale entries")
            return removed

    def to_dict(self) -> Dict[str, Any]:
        """Export HITL state"""
        return {
            "pending_count": self.pending_count(),
            "history_count": len(self._history),
            "default_timeout": self.default_timeout,
            "status": "active",
        }

    def __repr__(self) -> str:
        return f"HumanInTheLoop(pending={self.pending_count()}, history={len(self._history)})"


# ─── Convenience Functions ────────────────────────────────────

def create_hitl_workflow(
    action_type: str,
    context: Dict[str, Any],
    timeout: float = 300.0,
) -> Dict[str, Any]:
    """
    Tengeneza workflow rahisi ya HITL.
    Inarudisha action_id na maagizo.
    """
    hitl = HumanInTheLoop(default_timeout=timeout)
    action_id = hitl.request_approval(action_type, context, timeout=timeout)

    return {
        "action_id": action_id,
        "action_type": action_type,
        "instructions": f"Action '{action_type}' requires human approval. Use approve('{action_id}') or reject('{action_id}', reason).",
        "timeout_seconds": timeout,
    }
