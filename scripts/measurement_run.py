"""Publish measurements only through a completed, source-checked attempt journal.

One writer owns each output directory. An interrupted invocation remains running;
old public raw files alone never establish a successful current invocation.
"""
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
import uuid


def atomic_bytes(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as output:
            temporary = Path(output.name)
            output.write(data)
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def json_bytes(value):
    return (json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def positive_interval(value):
    if type(value) is not int or value <= 0:
        raise ValueError("nonpositive/noninteger elapsed duration")
    return value


class MeasurementRun:
    def __init__(self, directory, context_name="context.json", raw_name="current.jsonl", root=None):
        self.directory = Path(directory)
        self.destination = self.directory / context_name
        self.raw_name = raw_name
        self.root = root
        self.records = []
        self.attempt = Path("attempts") / Path(context_name).stem / uuid.uuid4().hex
        self.context = {"schema": 2, "status": "running", "attempt": self.attempt.as_posix(),
                        "started_at_utc": datetime.now(timezone.utc).isoformat(),
                        "completed_cases": 0,
                        "validity": "Only status=complete with matching raw_sha256 qualifies this invocation; older public files may remain."}

    def __enter__(self):
        self.update()
        return self

    def update(self, **fields):
        self.context.update(fields)
        data = json_bytes(self.context)
        # Retain every attempt, including failures and superseded successes.
        atomic_bytes(self.directory / self.attempt / "context.json", data)
        atomic_bytes(self.destination, data)

    def record(self, value):
        if self.context["status"] != "running":
            raise ValueError("measurement attempt is already closed")
        if any(row["case"] == value["case"] for row in self.records):
            raise ValueError("duplicate measurement case: " + value["case"])
        raw = (json.dumps(value, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")
        self.records.append(value)
        self._raw = getattr(self, "_raw", b"") + raw
        atomic_bytes(self.directory / self.attempt / "cases.jsonl", self._raw)
        self.update(completed_cases=len(self.records), partial_raw_sha256=hashlib.sha256(self._raw).hexdigest())

    def complete(self, **metadata):
        if self.context["status"] != "running" or not self.records:
            raise ValueError("cannot complete an empty or closed measurement attempt")
        # Validate JSON metadata before replacing any public raw file. Completion
        # is the final atomic manifest publication, after all comparability checks.
        json_bytes(metadata)
        atomic_bytes(self.directory / self.raw_name, self._raw)
        self.update(**metadata, status="complete", raw=self.raw_name,
                    raw_sha256=hashlib.sha256(self._raw).hexdigest(),
                    finished_at_utc=datetime.now(timezone.utc).isoformat())

    def __exit__(self, kind, error, traceback):
        if kind is None and self.context["status"] == "complete":
            return False
        error = error or ValueError("measurement attempt exited without completion")
        message = str(error)
        for path, label in [(self.root, "${REPOSITORY}"), (Path.home(), "${HOME}")]:
            if path is not None:
                message = message.replace(str(path), label)
        self.update(status="interrupted" if isinstance(error, (KeyboardInterrupt, SystemExit)) else "failed",
                    error=message, error_type=type(error).__name__,
                    finished_at_utc=datetime.now(timezone.utc).isoformat())
        if kind is None:
            raise error
        return False
