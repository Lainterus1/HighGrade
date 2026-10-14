"""Bounded subprocess trees. Windows uses owned Job Objects, not taskkill rights."""
from __future__ import annotations
import os
import signal
import subprocess
import time
from typing import Any


class _WindowsJob:
    def __init__(self):
        import ctypes as c
        from ctypes import wintypes as w
        self.c = c
        self.kernel = c.WinDLL("kernel32", use_last_error=True)
        class Basic(c.Structure):
            _fields_ = [("process_time", c.c_int64), ("job_time", c.c_int64),
                        ("flags", w.DWORD), ("min_working", c.c_size_t), ("max_working", c.c_size_t),
                        ("active_limit", w.DWORD), ("affinity", c.c_size_t),
                        ("priority", w.DWORD), ("scheduling", w.DWORD)]
        class IO(c.Structure):
            _fields_ = [(name, c.c_uint64) for name in ("reads", "writes", "others", "read_bytes", "write_bytes", "other_bytes")]
        class Extended(c.Structure):
            _fields_ = [("basic", Basic), ("io", IO), ("process_memory", c.c_size_t),
                        ("job_memory", c.c_size_t), ("peak_process", c.c_size_t), ("peak_job", c.c_size_t)]
        class Accounting(c.Structure):
            _fields_ = [(name, c.c_int64) for name in ("user", "kernel", "period_user", "period_kernel")] + [(name, w.DWORD) for name in ("faults", "total", "active", "terminated")]
        self.Accounting = Accounting
        declarations = {
            "CreateJobObjectW": ([c.c_void_p, w.LPCWSTR], w.HANDLE),
            "SetInformationJobObject": ([w.HANDLE, c.c_int, c.c_void_p, w.DWORD], w.BOOL),
            "AssignProcessToJobObject": ([w.HANDLE, w.HANDLE], w.BOOL),
            "TerminateJobObject": ([w.HANDLE, w.UINT], w.BOOL),
            "QueryInformationJobObject": ([w.HANDLE, c.c_int, c.c_void_p, w.DWORD, c.c_void_p], w.BOOL),
            "CloseHandle": ([w.HANDLE], w.BOOL),
        }
        for name, (args, result) in declarations.items():
            function = getattr(self.kernel, name)
            function.argtypes, function.restype = args, result
        self.handle = self.kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise c.WinError(c.get_last_error())
        limits = Extended()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not self.kernel.SetInformationJobObject(self.handle, 9, c.byref(limits), c.sizeof(limits)):
            self.kernel.CloseHandle(self.handle)
            raise c.WinError(c.get_last_error())

    def attach_and_resume(self, process):
        c = self.c
        if not self.kernel.AssignProcessToJobObject(self.handle, int(process._handle)):
            raise c.WinError(c.get_last_error())
        ntdll = c.WinDLL("ntdll")
        ntdll.NtResumeProcess.argtypes = [c.c_void_p]
        ntdll.NtResumeProcess.restype = c.c_long
        status = ntdll.NtResumeProcess(int(process._handle))
        if status != 0:
            raise OSError(f"NtResumeProcess failed: {status}")

    def stop(self):
        c = self.c
        if not self.kernel.TerminateJobObject(self.handle, 1):
            raise c.WinError(c.get_last_error())
        deadline = time.monotonic() + 10
        while True:
            info = self.Accounting()
            if not self.kernel.QueryInformationJobObject(self.handle, 1, c.byref(info), c.sizeof(info), None):
                raise c.WinError(c.get_last_error())
            if info.active == 0:
                return
            if time.monotonic() >= deadline:
                raise TimeoutError("owned process tree did not terminate")
            time.sleep(0.01)

    def close(self):
        self.kernel.CloseHandle(self.handle)


def run_bounded(command: list[str], *, timeout: float, check: bool = False,
                capture_output: bool = False, **kwargs: Any) -> subprocess.CompletedProcess:
    if capture_output:
        kwargs.update(stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    job = _WindowsJob() if os.name == "nt" else None
    owns_group = os.environ.get("CODE_HEALTH_PROCESS_GROUP") != "1"
    if job:
        kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW | 4  # CREATE_SUSPENDED: no child can escape assignment race.
    else:
        kwargs["start_new_session"] = owns_group
        env = dict(kwargs.get("env") or os.environ)
        env["CODE_HEALTH_PROCESS_GROUP"] = "1"
        kwargs["env"] = env
    process = None
    try:
        process = subprocess.Popen(command, **kwargs)
        if job:
            job.attach_and_resume(process)
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            if job:
                job.stop()
            elif owns_group:
                os.killpg(process.pid, signal.SIGKILL)
            process.kill()
            stdout, stderr = process.communicate(timeout=10)
            raise subprocess.TimeoutExpired(command, timeout, output=stdout, stderr=stderr)
        completed = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
        if check:
            completed.check_returncode()
        return completed
    finally:
        try:
            if job:
                job.stop()
            elif process is not None and owns_group:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait(timeout=10)
            if job:
                job.close()
